//! Public service façade (placeholder).

use crate::analyzer::TaskAnalyzer;
use crate::composer::SwarmComposer;
use crate::error::OrchestratorResult;
use crate::models::{ApprovalDecision, ApprovalStatus, ExecutionPlan, PlanId, SubtaskSpec};
use crate::patterns::CoordinationPattern;
use crate::patterns::{make_executor, TaskRunner};
use crate::storage::{ApprovalStore, MemoryApprovalStore, MemoryPlanStore, PlanStore};
use std::sync::Arc;

pub struct OrchestratorService {
    analyzer: TaskAnalyzer,
    composer: SwarmComposer,
    plan_store: Box<dyn PlanStore>,
    approval_store: Arc<dyn ApprovalStore>,
}

impl std::fmt::Debug for OrchestratorService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrchestratorService")
            .field("analyzer", &"TaskAnalyzer")
            .field("composer", &"SwarmComposer")
            .field("plan_store", &"PlanStore")
            .field("approval_store", &"ApprovalStore")
            .finish()
    }
}

impl OrchestratorService {
    pub fn new() -> Self {
        Self {
            analyzer: TaskAnalyzer::new(),
            composer: SwarmComposer::new(),
            plan_store: Box::new(MemoryPlanStore::default()),
            approval_store: Arc::new(MemoryApprovalStore::default()),
        }
    }

    pub fn plan_store(&self) -> &dyn PlanStore {
        &*self.plan_store
    }

    pub fn approval_store(&self) -> Arc<dyn ApprovalStore> {
        self.approval_store.clone()
    }

    pub fn apply_approval(&self, decision: ApprovalDecision) -> OrchestratorResult<ApprovalStatus> {
        let gov = crate::hitl::HitlGovernor::with_store(self.approval_store.clone());
        gov.apply(decision)
    }

    pub fn analyze_task(&self, task: &str) -> OrchestratorResult<Vec<SubtaskSpec>> {
        self.analyzer.analyze_offline(task)
    }

    pub fn plan(&self, task: &str) -> OrchestratorResult<ExecutionPlan> {
        let specs = self.analyze_task(task)?;
        let plan = ExecutionPlan {
            id: PlanId::from("plan-1"),
            name: task.to_string(),
            status: crate::models::plan::PlanStatus::Pending,
            nodes: specs
                .iter()
                .map(|s| crate::models::plan::PlanNode {
                    subtask_id: s.id.clone(),
                    status: crate::models::task::SubtaskStatus::Pending,
                })
                .collect(),
        };
        Ok(plan)
    }

    pub fn pick_pattern(&self, specs: &[SubtaskSpec]) -> OrchestratorResult<CoordinationPattern> {
        self.composer.pick_pattern(specs)
    }

    pub fn execute(
        &self,
        task: &str,
        runner: &dyn TaskRunner,
    ) -> OrchestratorResult<ExecutionPlan> {
        let specs = self.analyze_task(task)?;
        let pattern = self.composer.pick_pattern(&specs)?;
        let mut plan = ExecutionPlan {
            id: PlanId::from("plan-1"),
            name: task.to_string(),
            status: crate::models::plan::PlanStatus::Running,
            nodes: specs
                .iter()
                .map(|s| crate::models::plan::PlanNode {
                    subtask_id: s.id.clone(),
                    status: crate::models::task::SubtaskStatus::Pending,
                })
                .collect(),
        };
        self.run_plan(&specs, pattern, &mut plan, runner)
    }

    pub fn resume(
        &self,
        plan: &mut ExecutionPlan,
        specs: &[SubtaskSpec],
        runner: &dyn TaskRunner,
    ) -> OrchestratorResult<ExecutionPlan> {
        let pattern = self.composer.pick_pattern(specs)?;
        self.run_plan(specs, pattern, plan, runner)
    }

    fn run_plan(
        &self,
        specs: &[SubtaskSpec],
        pattern: CoordinationPattern,
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
    ) -> OrchestratorResult<ExecutionPlan> {
        let mut hitl = crate::hitl::HitlGovernor::with_store(self.approval_store.clone());
        let mut governance = crate::governance::Governance::new();
        governance.record_plan_started(&plan.id);
        let exec = make_executor(pattern);
        match exec.execute(specs, plan, runner, &mut hitl, &mut governance) {
            Ok(()) => {
                plan.status = crate::models::plan::PlanStatus::Completed;
                governance.record_plan_completed(&plan.id);
                self.plan_store.insert(plan.clone())?;
                Ok(plan.clone())
            }
            Err(err @ crate::error::OrchestratorError::Approval(_)) => {
                plan.status = crate::models::plan::PlanStatus::WaitingApproval;
                self.plan_store.insert(plan.clone())?;
                Err(err)
            }
            Err(err) => {
                plan.status = crate::models::plan::PlanStatus::Failed;
                governance.record_plan_failed(&plan.id, Some(err.to_string()));
                self.plan_store.insert(plan.clone())?;
                Err(err)
            }
        }
    }
}
