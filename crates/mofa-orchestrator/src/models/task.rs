use serde::{Deserialize, Serialize};

pub type SubtaskId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn requires_hitl(self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubtaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    WaitingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskSpec {
    pub id: SubtaskId,
    pub description: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub complexity: f64,
    #[serde(default)]
    pub deps: Vec<SubtaskId>,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub estimated_duration_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: SubtaskId,
    pub description: String,
    pub capabilities: Vec<String>,
    pub complexity: f64,
    pub status: SubtaskStatus,
    pub risk_level: RiskLevel,
    pub estimated_duration_secs: Option<u64>,
    pub hitl_required: bool,
}

impl Subtask {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            capabilities: Vec::new(),
            complexity: 0.5,
            status: SubtaskStatus::Pending,
            risk_level: RiskLevel::Medium,
            estimated_duration_secs: None,
            hitl_required: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: SubtaskId,
    pub to: SubtaskId,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyKind {
    Sequential,
}
