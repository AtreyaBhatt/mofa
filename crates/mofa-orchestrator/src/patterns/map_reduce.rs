use super::{PatternExecutor, TaskRunner};
use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::{ExecutionPlan, SubtaskSpec};
use futures::future::join_all;

#[derive(Debug, Default, Clone, Copy)]
pub struct MapReduceExecutor;

impl PatternExecutor for MapReduceExecutor {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()> {
        if subtasks.is_empty() {
            return Ok(());
        }
        // Naive placeholder: treat all but last as map, last as reduce with aggregation of statuses only.
        let (maps, reduce) = subtasks.split_at(subtasks.len().saturating_sub(1));
        let mut jobs = Vec::new();
        for sub in maps {
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
        let results = futures::executor::block_on(async { join_all(jobs).await });
        for (id, outcome) in results {
            let outcome = outcome?;
            plan.set_status(&id, outcome.status);
            match outcome.status {
                crate::models::task::SubtaskStatus::Completed => {
                    governance.record_subtask_completed(&plan.id, &id)
                }
                crate::models::task::SubtaskStatus::Failed => governance
                    .record_subtask_failed(&plan.id, &id, Some("runner failed".into())),
                _ => {}
            }
        }

        if let Some(reduce_task) = reduce.first() {
            if let Some(req) = hitl.maybe_request_approval(&plan.id, reduce_task)? {
                governance.record_approval_requested(&plan.id, &reduce_task.id);
                plan.set_status(&reduce_task.id, crate::models::task::SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    req.subtask_id
                )));
            }
            if !super::maybe_resume(reduce_task, hitl)? {
                plan.set_status(&reduce_task.id, crate::models::task::SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    reduce_task.id
                )));
            }
            governance.record_subtask_started(&plan.id, &reduce_task.id);
            let outcome = runner.run(reduce_task)?;
            plan.set_status(&reduce_task.id, outcome.status);
            match outcome.status {
                crate::models::task::SubtaskStatus::Completed => governance
                    .record_subtask_completed(&plan.id, &reduce_task.id),
                crate::models::task::SubtaskStatus::Failed => governance.record_subtask_failed(
                    &plan.id,
                    &reduce_task.id,
                    Some("runner failed".into()),
                ),
                _ => {}
            }
        }
        Ok(())
    }
}
