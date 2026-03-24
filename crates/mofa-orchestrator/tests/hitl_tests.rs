use mofa_orchestrator::hitl::{HitlGovernor, HitlNotifier};
use mofa_orchestrator::models::{
    ApprovalDecision, ApprovalRequest, ApprovalStatus, RiskLevel, SubtaskSpec,
};
use std::time::Duration;

fn risky_subtask() -> SubtaskSpec {
    SubtaskSpec {
        id: "step-risky".into(),
        description: "risky".into(),
        capabilities: vec![],
        complexity: 0.5,
        deps: vec![],
        risk_level: Some(RiskLevel::High),
        estimated_duration_secs: None,
    }
}

#[test]
fn hitl_creates_pending_request() {
    let gov = HitlGovernor::new();
    let result = gov.maybe_request_approval(&"plan-1".to_string(), &risky_subtask());
    assert!(matches!(result, Ok(Some(_))));
    let pending = gov.pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, ApprovalStatus::Pending);
}

#[test]
fn hitl_apply_updates_status() {
    let gov = HitlGovernor::new();
    let _ = gov.maybe_request_approval(&"plan-1".to_string(), &risky_subtask());
    let pending = gov.pending().unwrap();
    let id = &pending[0].id;
    let status = gov
        .apply(ApprovalDecision {
            approval_id: id.clone(),
            approved: true,
            reason: None,
        })
        .unwrap();
    assert_eq!(status, ApprovalStatus::Approved);
    let pending_after = gov.pending().unwrap();
    assert!(pending_after.is_empty());
}

#[test]
fn hitl_escalates_expired() {
    let gov = HitlGovernor::new();
    let _ = gov.maybe_request_approval(&"plan-1".to_string(), &risky_subtask());
    let pending = gov.pending().unwrap();
    let id = &pending[0].id;
    // Replace with a request that is already expired
    let expired = ApprovalRequest {
        id: id.clone(),
        plan_id: "plan-1".into(),
        subtask_id: "step-risky".into(),
        rationale: "Approval required".into(),
        status: ApprovalStatus::Pending,
        expires_at_ms: Some(0),
        escalation_ms: Some(0),
        escalation_level: None,
    };
    gov.request(expired).unwrap();
    gov.escalate_expired(1).unwrap();
    let pending_after = gov.pending().unwrap();
    assert!(pending_after.is_empty());
}

#[tokio::test]
async fn hitl_waits_for_approval() {
    let gov = HitlGovernor::new();
    let req = gov
        .maybe_request_approval(&"plan-1".to_string(), &risky_subtask())
        .unwrap()
        .unwrap();
    let approval_id = req.id.clone();
    let waiter = gov.wait_for_approval(&approval_id, Duration::from_millis(200));
    // Approve shortly after
    let gov_clone = HitlGovernor::with_store(gov.approval_store());
    let approve = async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let _ = gov_clone
            .apply(ApprovalDecision {
                approval_id: req.id.clone(),
                approved: true,
                reason: None,
            })
            .unwrap();
    };
    let (status, _) = tokio::join!(waiter, approve);
    assert_eq!(status.unwrap(), ApprovalStatus::Approved);
}

#[derive(Default)]
struct TestNotifier {
    seen: parking_lot::Mutex<Vec<String>>,
}

impl HitlNotifier for TestNotifier {
    fn notify(&self, req: &ApprovalRequest) {
        self.seen.lock().push(req.id.clone());
    }
}

#[tokio::test]
async fn hitl_notifier_and_auto_escalate() {
    let notifier = std::sync::Arc::new(TestNotifier::default());
    let gov = HitlGovernor::new().with_notifier(notifier.clone());
    let req = gov
        .maybe_request_approval(&"plan-1".to_string(), &risky_subtask())
        .unwrap()
        .unwrap();
    assert_eq!(notifier.seen.lock().len(), 1);

    // Replace with expired request and run auto-escalate
    let expired = ApprovalRequest {
        id: req.id.clone(),
        plan_id: "plan-1".into(),
        subtask_id: "step-risky".into(),
        rationale: "Approval required".into(),
        status: ApprovalStatus::Pending,
        expires_at_ms: Some(0),
        escalation_ms: Some(0),
        escalation_level: None,
    };
    gov.request(expired).unwrap();
    gov.auto_escalate_loop(Duration::from_millis(5), Duration::from_millis(15))
        .await
        .unwrap();
    let pending_after = gov.pending().unwrap();
    assert!(pending_after.is_empty());
}
