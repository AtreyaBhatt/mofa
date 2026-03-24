mod consensus;
mod debate;
mod map_reduce;
mod parallel;
mod routing;
mod sequential;
mod supervision;

use crate::error::OrchestratorResult;
use crate::governance::Governance;
use crate::hitl::HitlGovernor;
use crate::models::{ExecutionPlan, SubtaskSpec};

pub use consensus::ConsensusExecutor;
pub use debate::DebateExecutor;
pub use map_reduce::MapReduceExecutor;
pub use parallel::ParallelExecutor;
pub use routing::RoutingExecutor;
pub use sequential::SequentialExecutor;
pub use supervision::SupervisionExecutor;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum CoordinationPattern {
    Sequential,
    Parallel,
    MapReduce,
    Routing,
    Supervision,
    Debate,
    Consensus,
}

impl CoordinationPattern {
    pub fn name(self) -> &'static str {
        match self {
            CoordinationPattern::Sequential => "sequential",
            CoordinationPattern::Parallel => "parallel",
            CoordinationPattern::MapReduce => "map_reduce",
            CoordinationPattern::Routing => "routing",
            CoordinationPattern::Supervision => "supervision",
            CoordinationPattern::Debate => "debate",
            CoordinationPattern::Consensus => "consensus",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubtaskOutcome {
    pub status: crate::models::SubtaskStatus,
    pub output: Option<String>,
}

pub trait TaskRunner: Send + Sync {
    fn run(&self, subtask: &SubtaskSpec) -> OrchestratorResult<SubtaskOutcome>;
}

pub trait PatternExecutor: Send + Sync {
    fn execute(
        &self,
        subtasks: &[SubtaskSpec],
        plan: &mut ExecutionPlan,
        runner: &dyn TaskRunner,
        hitl: &mut HitlGovernor,
        governance: &mut Governance,
    ) -> OrchestratorResult<()>;
}

pub fn is_waiting(status: &crate::models::task::SubtaskStatus) -> bool {
    matches!(status, crate::models::task::SubtaskStatus::WaitingApproval)
}

pub fn maybe_resume(subtask: &SubtaskSpec, hitl: &HitlGovernor) -> OrchestratorResult<bool> {
    if let Some(existing) = hitl.get(&format!("appr-{}", subtask.id))? {
        return Ok(existing.status == crate::models::ApprovalStatus::Approved);
    }
    Ok(true)
}

pub fn make_executor(pattern: CoordinationPattern) -> Box<dyn PatternExecutor> {
    match pattern {
        CoordinationPattern::Sequential => Box::new(SequentialExecutor),
        CoordinationPattern::Parallel => Box::new(ParallelExecutor),
        CoordinationPattern::MapReduce => Box::new(MapReduceExecutor),
        CoordinationPattern::Routing => Box::new(RoutingExecutor),
        CoordinationPattern::Supervision => Box::new(SupervisionExecutor),
        CoordinationPattern::Debate => Box::new(DebateExecutor),
        CoordinationPattern::Consensus => Box::new(ConsensusExecutor),
    }
}

#[cfg(test)]
mod tests {
    use super::{make_executor, CoordinationPattern};

    #[test]
    fn coordination_pattern_names_are_stable() {
        assert_eq!(CoordinationPattern::Sequential.name(), "sequential");
        assert_eq!(CoordinationPattern::Parallel.name(), "parallel");
        assert_eq!(CoordinationPattern::MapReduce.name(), "map_reduce");
        assert_eq!(CoordinationPattern::Routing.name(), "routing");
        assert_eq!(CoordinationPattern::Supervision.name(), "supervision");
        assert_eq!(CoordinationPattern::Debate.name(), "debate");
        assert_eq!(CoordinationPattern::Consensus.name(), "consensus");
    }

    #[test]
    fn make_executor_constructs_all_variants() {
        let _ = make_executor(CoordinationPattern::Sequential);
        let _ = make_executor(CoordinationPattern::Parallel);
        let _ = make_executor(CoordinationPattern::MapReduce);
        let _ = make_executor(CoordinationPattern::Routing);
        let _ = make_executor(CoordinationPattern::Supervision);
        let _ = make_executor(CoordinationPattern::Debate);
        let _ = make_executor(CoordinationPattern::Consensus);
    }
}
