pub mod approval;
pub mod audit;
pub mod plan;
pub mod plugin_manifest;
pub mod registry;
pub mod task;

pub use approval::{ApprovalDecision, ApprovalId, ApprovalRequest, ApprovalStatus};
pub use audit::{AuditEntry, AuditEventKind};
pub use plan::{ExecutionPlan, PlanId, PlanNode, PlanStatus};
pub use plugin_manifest::{PluginDependency, PluginId, PluginManifest, Signature};
pub use registry::{CapabilityDescriptor, CapabilityId, CapabilityMatch};
pub use task::{
    Dependency, DependencyKind, RiskLevel, Subtask, SubtaskId, SubtaskSpec, SubtaskStatus,
};
