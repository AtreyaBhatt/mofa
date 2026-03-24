use super::{PatternExecutor, TaskRunner};
use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::{ExecutionPlan, SubtaskSpec};

#[derive(Debug, Default, Clone, Copy)]
pub struct SequentialExecutor;

impl PatternExecutor for SequentialExecutor {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()> {
        for sub in subtasks {
            if let Some(req) = hitl.maybe_request_approval(&plan.id, sub)? {
                governance.record_approval_requested(&plan.id, &sub.id);
                plan.set_status(&sub.id, crate::models::task::SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    req.subtask_id
                )));
            }

            if !super::maybe_resume(sub, hitl)? {
                plan.set_status(&sub.id, crate::models::task::SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    sub.id
                )));
            }

            governance.record_subtask_started(&plan.id, &sub.id);
            let outcome = runner.run(sub)?;
            plan.set_status(&sub.id, outcome.status);
            match outcome.status {
                crate::models::task::SubtaskStatus::Completed => {
                    governance.record_subtask_completed(&plan.id, &sub.id)
                }
                crate::models::task::SubtaskStatus::Failed => governance.record_subtask_failed(
                    &plan.id,
                    &sub.id,
                    Some("runner failed".into()),
                ),
                _ => {}
            }
        }
        Ok(())
    }
}
