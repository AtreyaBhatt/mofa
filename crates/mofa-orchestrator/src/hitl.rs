//! Placeholder HITL governor.

use crate::error::{OrchestratorError, OrchestratorResult};
use crate::models::{ApprovalDecision, ApprovalRequest, ApprovalStatus, PlanId, SubtaskSpec};
use crate::storage::ApprovalStore;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct HitlGovernor {
    pub(crate) approvals: Arc<dyn ApprovalStore>,
    notifier: Option<Arc<dyn HitlNotifier>>,
    clock: Arc<dyn crate::governance::Clock>,
}

pub trait HitlNotifier: Send + Sync {
    fn notify(&self, req: &ApprovalRequest);
}

impl std::fmt::Debug for HitlGovernor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HitlGovernor")
            .field("approvals", &"ApprovalStore")
            .finish()
    }
}

impl HitlGovernor {
    pub fn new() -> Self {
        Self {
            approvals: Arc::new(crate::storage::MemoryApprovalStore::default()),
            notifier: None,
            clock: Arc::new(crate::governance::SystemClock),
        }
    }

    pub fn with_store(store: Arc<dyn ApprovalStore>) -> Self {
        Self {
            approvals: store,
            notifier: None,
            clock: Arc::new(crate::governance::SystemClock),
        }
    }

    pub fn with_notifier(mut self, notifier: Arc<dyn HitlNotifier>) -> Self {
        self.notifier = Some(notifier);
        self
    }

    pub fn with_clock(mut self, clock: Arc<dyn crate::governance::Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub fn approval_store(&self) -> Arc<dyn ApprovalStore> {
        self.approvals.clone()
    }

    pub fn request(&self, req: ApprovalRequest) -> OrchestratorResult<ApprovalRequest> {
        if req.id.is_empty() {
            return Err(OrchestratorError::InvalidInput(
                "approval id is empty".into(),
            ));
        }
        self.approvals.insert(req.clone())?;
        Ok(req)
    }

    pub fn apply(&self, decision: ApprovalDecision) -> OrchestratorResult<ApprovalStatus> {
        if decision.approval_id.is_empty() {
            return Err(OrchestratorError::InvalidInput(
                "approval id is empty".into(),
            ));
        }
        let status = if decision.approved {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Rejected
        };
        self.approvals
            .update_status(&decision.approval_id, status.clone())?;
        Ok(status)
    }

    pub fn maybe_request_approval(
        &self,
        plan_id: &PlanId,
        subtask: &SubtaskSpec,
    ) -> OrchestratorResult<Option<ApprovalRequest>> {
        if matches!(subtask.risk_level, Some(r) if r.requires_hitl()) {
            // If an approval already exists, reuse it.
            if let Some(existing) = self.approvals.get(&format!("appr-{}", subtask.id))? {
                match existing.status {
                    ApprovalStatus::Pending => return Ok(Some(existing)),
                    ApprovalStatus::Approved => return Ok(None),
                    ApprovalStatus::Rejected | ApprovalStatus::Expired | ApprovalStatus::Escalated => {
                        return Err(OrchestratorError::Approval(format!(
                            "subtask {} approval not in approvable state",
                            subtask.id
                        )))
                    }
                }
            }

            let req = ApprovalRequest {
                id: format!("appr-{}", subtask.id),
                plan_id: plan_id.clone(),
                subtask_id: subtask.id.clone(),
                rationale: format!("Approval required for {}", subtask.id),
                status: ApprovalStatus::Pending,
                expires_at_ms: None,
                escalation_ms: None,
                escalation_level: None,
            };
            self.approvals.insert(req.clone())?;
            if let Some(notifier) = &self.notifier {
                notifier.notify(&req);
            }
            return Ok(Some(req));
        }
        Ok(None)
    }

    pub fn pending(&self) -> OrchestratorResult<Vec<ApprovalRequest>> {
        self.approvals.list_pending()
    }

    pub fn escalate_expired(&self, now_ms: u64) -> OrchestratorResult<()> {
        let pending = self.approvals.list_pending()?;
        for req in pending {
            if let Some(expiry) = req.expires_at_ms {
                if now_ms >= expiry {
                    self.approvals
                        .update_status(&req.id, ApprovalStatus::Escalated)?;
                }
            }
        }
        Ok(())
    }

    pub async fn auto_escalate_loop(
        &self,
        interval: Duration,
        stop_after: Duration,
    ) -> OrchestratorResult<()> {
        let start = std::time::Instant::now();
        while start.elapsed() < stop_after {
            let now = self.clock.now_millis();
            self.escalate_expired(now)?;
            sleep(interval).await;
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> OrchestratorResult<Option<ApprovalRequest>> {
        self.approvals.get(&id.to_string())
    }

    pub async fn wait_for_approval(
        &self,
        approval_id: &str,
        timeout: Duration,
    ) -> OrchestratorResult<ApprovalStatus> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() >= timeout {
                return Err(OrchestratorError::Approval("approval wait timed out".into()));
            }
            if let Some(current) = self.approvals.get(&approval_id.to_string())? {
                if current.status != ApprovalStatus::Pending {
                    return Ok(current.status);
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    }
}
