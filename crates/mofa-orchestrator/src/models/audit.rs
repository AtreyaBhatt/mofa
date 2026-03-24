use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuditEventKind {
    PlanCreated,
    PlanStarted,
    PlanCompleted,
    PlanFailed,
    SubtaskStarted,
    SubtaskCompleted,
    SubtaskFailed,
    ApprovalRequested,
    ApprovalResolved,
    SlaBreached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub ts_ms: u64,
    pub plan_id: String,
    pub subtask_id: Option<String>,
    pub event: AuditEventKind,
    pub detail: Option<String>,
}
