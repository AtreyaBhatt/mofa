//! Storage traits and in-memory stubs.

use crate::error::OrchestratorResult;
use crate::models::plan;
use crate::models::{ApprovalId, ApprovalRequest, AuditEntry, ExecutionPlan, PlanId};
use std::collections::HashMap;

pub trait PlanStore: Send + Sync {
    fn insert(&self, plan: ExecutionPlan) -> OrchestratorResult<()>;
    fn get(&self, id: &PlanId) -> OrchestratorResult<Option<ExecutionPlan>>;
    fn update_status(&self, id: &PlanId, status: plan::PlanStatus) -> OrchestratorResult<()>;
}

pub trait ApprovalStore: Send + Sync {
    fn insert(&self, req: ApprovalRequest) -> OrchestratorResult<()>;
    fn get(&self, id: &ApprovalId) -> OrchestratorResult<Option<ApprovalRequest>>;
    fn update_status(
        &self,
        id: &ApprovalId,
        status: crate::models::ApprovalStatus,
    ) -> OrchestratorResult<()>;
    fn list_pending(&self) -> OrchestratorResult<Vec<ApprovalRequest>>;
}

pub trait AuditStore: Send + Sync {
    fn append(&self, entry: AuditEntry) -> OrchestratorResult<()>;
    fn all(&self) -> OrchestratorResult<Vec<AuditEntry>>;
}

#[derive(Debug, Default)]
pub struct MemoryPlanStore {
    plans: parking_lot::RwLock<HashMap<PlanId, ExecutionPlan>>,
}

impl PlanStore for MemoryPlanStore {
    fn insert(&self, plan: ExecutionPlan) -> OrchestratorResult<()> {
        self.plans.write().insert(plan.id.clone(), plan);
        Ok(())
    }

    fn get(&self, id: &PlanId) -> OrchestratorResult<Option<ExecutionPlan>> {
        Ok(self.plans.read().get(id).cloned())
    }

    fn update_status(&self, id: &PlanId, status: plan::PlanStatus) -> OrchestratorResult<()> {
        if let Some(p) = self.plans.write().get_mut(id) {
            p.status = status;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MemoryApprovalStore {
    approvals: parking_lot::RwLock<HashMap<ApprovalId, ApprovalRequest>>,
}

impl ApprovalStore for MemoryApprovalStore {
    fn insert(&self, req: ApprovalRequest) -> OrchestratorResult<()> {
        self.approvals.write().insert(req.id.clone(), req);
        Ok(())
    }

    fn get(&self, id: &ApprovalId) -> OrchestratorResult<Option<ApprovalRequest>> {
        Ok(self.approvals.read().get(id).cloned())
    }

    fn update_status(
        &self,
        id: &ApprovalId,
        status: crate::models::ApprovalStatus,
    ) -> OrchestratorResult<()> {
        if let Some(existing) = self.approvals.write().get_mut(id) {
            existing.status = status;
        }
        Ok(())
    }

    fn list_pending(&self) -> OrchestratorResult<Vec<ApprovalRequest>> {
        Ok(self
            .approvals
            .read()
            .values()
            .filter(|a| a.status == crate::models::ApprovalStatus::Pending)
            .cloned()
            .collect())
    }
}

#[derive(Debug, Default)]
pub struct MemoryAuditStore {
    entries: parking_lot::RwLock<Vec<AuditEntry>>,
}

impl AuditStore for MemoryAuditStore {
    fn append(&self, entry: AuditEntry) -> OrchestratorResult<()> {
        self.entries.write().push(entry);
        Ok(())
    }

    fn all(&self) -> OrchestratorResult<Vec<AuditEntry>> {
        Ok(self.entries.read().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{ApprovalStore, MemoryApprovalStore, MemoryPlanStore, PlanStore};
    use crate::models::plan::{ExecutionPlan, PlanId, PlanNode, PlanStatus};
    use crate::models::{ApprovalRequest, ApprovalStatus};

    #[test]
    fn memory_plan_store_roundtrip() {
        let store = MemoryPlanStore::default();
        let plan = ExecutionPlan {
            id: PlanId::from("p1"),
            name: "demo".into(),
            status: PlanStatus::Pending,
            nodes: vec![PlanNode {
                subtask_id: "s1".into(),
                status: crate::models::task::SubtaskStatus::Pending,
            }],
        };
        store.insert(plan.clone()).unwrap();
        let fetched = store.get(&PlanId::from("p1")).unwrap().unwrap();
        assert_eq!(fetched.name, "demo");
        store
            .update_status(&PlanId::from("p1"), PlanStatus::Completed)
            .unwrap();
        let updated = store.get(&PlanId::from("p1")).unwrap().unwrap();
        assert_eq!(updated.status, PlanStatus::Completed);
    }

    #[test]
    fn memory_approval_store_roundtrip() {
        let store = MemoryApprovalStore::default();
        let req = ApprovalRequest {
            id: "a1".into(),
            plan_id: "p1".into(),
            subtask_id: "s1".into(),
            rationale: "need".into(),
            status: ApprovalStatus::Pending,
            expires_at_ms: None,
            escalation_ms: None,
            escalation_level: None,
        };
        store.insert(req.clone()).unwrap();
        let fetched = store.get(&"a1".into()).unwrap().unwrap();
        assert_eq!(fetched.plan_id, "p1");
        store
            .update_status(&"a1".into(), ApprovalStatus::Approved)
            .unwrap();
        let pending = store.list_pending().unwrap();
        assert!(pending.is_empty());
    }
}
