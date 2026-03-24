use super::{PatternExecutor, TaskRunner};
use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::SubtaskStatus;
use crate::models::{ExecutionPlan, SubtaskSpec};

#[derive(Debug, Default, Clone, Copy)]
pub struct SupervisionExecutor;

impl PatternExecutor for SupervisionExecutor {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()> {
        // Supervision: run tasks sequentially; if a task fails, supervisor (last subtask if present) retries once.
        if subtasks.is_empty() {
            return Ok(());
        }
        let (workers, supervisor_opt) = if subtasks.len() > 1 {
            subtasks.split_at(subtasks.len() - 1)
        } else {
            (&subtasks[0..], &[][..])
        };

        for sub in workers {
            if let Some(req) = hitl.maybe_request_approval(&plan.id, sub)? {
                governance.record_approval_requested(&plan.id, &sub.id);
                plan.set_status(&sub.id, SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    req.subtask_id
                )));
            }
            if !super::maybe_resume(sub, hitl)? {
                plan.set_status(&sub.id, SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    sub.id
                )));
            }
            governance.record_subtask_started(&plan.id, &sub.id);
            let outcome = runner.run(sub)?;
            let status = outcome.status;
            plan.set_status(&sub.id, status);
            if status != SubtaskStatus::Completed {
                // Supervisor retry once if present
                if let Some(supervisor) = supervisor_opt.first() {
                    if let Some(req) = hitl.maybe_request_approval(&plan.id, supervisor)? {
                        governance.record_approval_requested(&plan.id, &supervisor.id);
                        plan.set_status(&supervisor.id, SubtaskStatus::WaitingApproval);
                        return Err(crate::error::OrchestratorError::Approval(format!(
                            "subtask {} awaiting approval",
                            req.subtask_id
                        )));
                    }
                    if !super::maybe_resume(supervisor, hitl)? {
                        plan.set_status(&supervisor.id, SubtaskStatus::WaitingApproval);
                        return Err(crate::error::OrchestratorError::Approval(format!(
                            "subtask {} awaiting approval",
                            supervisor.id
                        )));
                    }
                    governance.record_subtask_started(&plan.id, &supervisor.id);
                    let sup_outcome = runner.run(supervisor)?;
                    plan.set_status(&supervisor.id, sup_outcome.status);
                    match sup_outcome.status {
                        SubtaskStatus::Completed => {
                            governance.record_subtask_completed(&plan.id, &supervisor.id)
                        }
                        SubtaskStatus::Failed => governance.record_subtask_failed(
                            &plan.id,
                            &supervisor.id,
                            Some("runner failed".into()),
                        ),
                        _ => {}
                    }
                }
                break;
            }
            governance.record_success(&sub.id);
            governance.record_subtask_completed(&plan.id, &sub.id);
        }
        Ok(())
    }
}
