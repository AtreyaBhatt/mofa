use mofa_orchestrator::api::OrchestratorService;
use mofa_orchestrator::error::OrchestratorError;
use mofa_orchestrator::governance::GovernanceMetricsSnapshot;
use mofa_orchestrator::patterns::CoordinationPattern;

#[test]
fn analyzer_offline_produces_single_step() {
    let svc = OrchestratorService::new();
    let specs = svc.analyze_task("demo task").unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id, "step-1");
}

#[test]
fn pick_pattern_defaults_to_sequential() {
    let svc = OrchestratorService::new();
    let specs = svc.analyze_task("demo task").unwrap();
    let pattern = svc.pick_pattern(&specs).unwrap();
    matches!(pattern, CoordinationPattern::Sequential);
}

struct NoopRunner;

impl mofa_orchestrator::patterns::TaskRunner for NoopRunner {
    fn run(
        &self,
        _subtask: &mofa_orchestrator::models::SubtaskSpec,
    ) -> Result<mofa_orchestrator::patterns::SubtaskOutcome, OrchestratorError> {
        Ok(mofa_orchestrator::patterns::SubtaskOutcome {
            status: mofa_orchestrator::models::SubtaskStatus::Completed,
            output: None,
        })
    }
}

#[test]
fn execute_sets_waiting_on_hitl_required() {
    let svc = OrchestratorService::new();
    let runner = NoopRunner;
    let err = svc.execute("high risk task", &runner).unwrap_err();
    match err {
        OrchestratorError::Approval(_) => {}
        other => panic!("expected approval error, got {:?}", other),
    }
    let plan = svc
        .plan_store()
        .get(&"plan-1".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(
        plan.status,
        mofa_orchestrator::models::plan::PlanStatus::WaitingApproval
    );
    assert_eq!(
        plan.nodes[0].status,
        mofa_orchestrator::models::SubtaskStatus::WaitingApproval
    );

    // Apply approval and resume
    let pending = svc.approval_store().list_pending().unwrap();
    assert_eq!(pending.len(), 1);
    let approval_id = pending[0].id.clone();
    svc.apply_approval(mofa_orchestrator::models::ApprovalDecision {
        approval_id: approval_id.clone(),
        approved: true,
        reason: None,
    })
    .unwrap();

    let specs = svc.analyze_task("high risk task").unwrap();
    let mut plan = svc
        .plan_store()
        .get(&"plan-1".to_string())
        .unwrap()
        .unwrap();
    let resumed = svc.resume(&mut plan, &specs, &runner).unwrap();
    assert_eq!(
        resumed.status,
        mofa_orchestrator::models::plan::PlanStatus::Completed
    );
    assert_eq!(
        resumed.nodes[0].status,
        mofa_orchestrator::models::SubtaskStatus::Completed
    );

    // Metrics snapshot sanity
    let metrics = mofa_orchestrator::governance::GovernanceMetrics::default();
    let snapshot = metrics.snapshot();
    let empty = GovernanceMetricsSnapshot::default();
    assert_eq!(snapshot, empty);
}
