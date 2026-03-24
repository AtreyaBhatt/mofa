use super::{PatternExecutor, TaskRunner};
use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::SubtaskStatus;
use crate::models::{ExecutionPlan, SubtaskSpec};

#[derive(Debug, Default, Clone, Copy)]
pub struct RoutingExecutor;

impl PatternExecutor for RoutingExecutor {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()> {
        // Simple routing: try tasks in order until one completes; others are skipped.
        for sub in subtasks {
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
            match status {
                SubtaskStatus::Completed => governance.record_subtask_completed(&plan.id, &sub.id),
                SubtaskStatus::Failed => governance.record_subtask_failed(
                    &plan.id,
                    &sub.id,
                    Some("runner failed".into()),
                ),
                _ => {}
            }
            if status == SubtaskStatus::Completed {
                break;
            }
        }
        Ok(())
    }
}
