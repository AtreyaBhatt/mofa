use serde::{Deserialize, Serialize};

pub type ApprovalId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalId,
    pub plan_id: String,
    pub subtask_id: String,
    pub rationale: String,
    pub status: ApprovalStatus,
    pub expires_at_ms: Option<u64>,
    #[serde(default)]
    pub escalation_ms: Option<u64>,
    #[serde(default)]
    pub escalation_level: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub approval_id: ApprovalId,
    pub approved: bool,
    pub reason: Option<String>,
}
