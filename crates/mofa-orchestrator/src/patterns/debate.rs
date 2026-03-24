use super::{PatternExecutor, TaskRunner};
use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::{ExecutionPlan, SubtaskSpec};

#[derive(Debug, Default, Clone, Copy)]
pub struct DebateExecutor;

impl PatternExecutor for DebateExecutor {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()> {
        // Simple debate: first two subtasks are debaters run concurrently; final subtask (if any) is judge.
        if subtasks.len() <= 1 {
            if let Some(sub) = subtasks.first() {
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
                    crate::models::task::SubtaskStatus::Completed => governance
                        .record_subtask_completed(&plan.id, &sub.id),
                    crate::models::task::SubtaskStatus::Failed => governance
                        .record_subtask_failed(&plan.id, &sub.id, Some("runner failed".into())),
                    _ => {}
                }
            }
            return Ok(());
        }

        let (debaters, rest) = subtasks.split_at(subtasks.len().saturating_sub(1));
        let mut jobs = Vec::new();
        for sub in debaters {
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

        if let Some(judge) = rest.first() {
            if let Some(req) = hitl.maybe_request_approval(&plan.id, judge)? {
                governance.record_approval_requested(&plan.id, &judge.id);
                plan.set_status(&judge.id, crate::models::task::SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    req.subtask_id
                )));
            }
            if !super::maybe_resume(judge, hitl)? {
                plan.set_status(&judge.id, crate::models::task::SubtaskStatus::WaitingApproval);
                return Err(crate::error::OrchestratorError::Approval(format!(
                    "subtask {} awaiting approval",
                    judge.id
                )));
            }
            governance.record_subtask_started(&plan.id, &judge.id);
            let outcome = runner.run(judge)?;
            plan.set_status(&judge.id, outcome.status);
            match outcome.status {
                crate::models::task::SubtaskStatus::Completed => governance
                    .record_subtask_completed(&plan.id, &judge.id),
                crate::models::task::SubtaskStatus::Failed => governance
                    .record_subtask_failed(&plan.id, &judge.id, Some("runner failed".into())),
                _ => {}
            }
        }
        Ok(())
    }
}
