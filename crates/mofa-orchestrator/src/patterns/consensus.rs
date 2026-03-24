use super::{PatternExecutor, TaskRunner};
use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::{ExecutionPlan, SubtaskSpec};
use crate::models::SubtaskStatus;

#[derive(Debug, Default, Clone, Copy)]
pub struct ConsensusExecutor;

impl PatternExecutor for ConsensusExecutor {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()> {
        // Simple consensus: run all voters concurrently, then if any failed, mark plan nodes; otherwise success.
        let mut jobs = Vec::new();
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
            let spec = sub.clone();
            let runner_ref = runner;
            jobs.push(async move { (spec.id.clone(), runner_ref.run(&spec)) });
        }
        let results = futures::executor::block_on(async { futures::future::join_all(jobs).await });
        let mut any_failed = false;
        for (id, outcome) in results {
            let outcome = outcome?;
            if outcome.status != SubtaskStatus::Completed {
                any_failed = true;
            }
            plan.set_status(&id, outcome.status);
            match outcome.status {
                SubtaskStatus::Completed => governance.record_subtask_completed(&plan.id, &id),
                SubtaskStatus::Failed => governance.record_subtask_failed(
                    &plan.id,
                    &id,
                    Some("runner failed".into()),
                ),
                _ => {}
            }
        }
        if any_failed {
            plan.status = crate::models::plan::PlanStatus::Failed;
        }
        Ok(())
    }
}
