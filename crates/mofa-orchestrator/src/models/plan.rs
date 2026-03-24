use serde::{Deserialize, Serialize};

pub type PlanId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanStatus {
    Pending,
    Running,
    WaitingApproval,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub subtask_id: String,
    pub status: super::task::SubtaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: PlanId,
    pub name: String,
    pub status: PlanStatus,
    pub nodes: Vec<PlanNode>,
}

impl ExecutionPlan {
    pub fn set_status(&mut self, subtask_id: &str, status: super::task::SubtaskStatus) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.subtask_id == subtask_id) {
            node.status = status;
        }
    }
}
