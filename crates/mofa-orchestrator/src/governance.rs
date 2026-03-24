//! Placeholder governance layer (SLA, audit, metrics hooks).

use crate::models::{AuditEntry, AuditEventKind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub trait Clock: Send + Sync {
    fn now_millis(&self) -> u64;
}

#[derive(Debug, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

pub struct Governance {
    audit: Vec<AuditEntry>,
    sla_deadline_ms: Option<u64>,
    clock: Box<dyn Clock>,
    metrics: Arc<GovernanceMetrics>,
}

impl std::fmt::Debug for Governance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Governance")
            .field("audit_len", &self.audit.len())
            .field("sla_deadline_ms", &self.sla_deadline_ms)
            .field("clock", &"Clock")
            .finish()
    }
}

impl Governance {
    pub fn new() -> Self {
        Self {
            audit: Vec::new(),
            sla_deadline_ms: None,
            clock: Box::new(SystemClock),
            metrics: Arc::new(GovernanceMetrics::default()),
        }
    }

    pub fn with_clock(clock: Box<dyn Clock>) -> Self {
        Self {
            audit: Vec::new(),
            sla_deadline_ms: None,
            clock,
            metrics: Arc::new(GovernanceMetrics::default()),
        }
    }

    pub fn record(&mut self, entry: AuditEntry) {
        self.audit.push(entry);
    }

    pub fn record_success(&mut self, _subtask_id: &str) {
        // Placeholder for SLA/metrics hooks; extend when SLA/metrics are added.
    }

    pub fn record_subtask_started(&mut self, plan_id: &str, subtask_id: &str) {
        self.metrics
            .subtasks_started
            .fetch_add(1, Ordering::Relaxed);
        self.record_event(
            plan_id,
            Some(subtask_id),
            AuditEventKind::SubtaskStarted,
            None,
        );
    }

    pub fn record_subtask_completed(&mut self, plan_id: &str, subtask_id: &str) {
        self.metrics
            .subtasks_completed
            .fetch_add(1, Ordering::Relaxed);
        self.record_event(
            plan_id,
            Some(subtask_id),
            AuditEventKind::SubtaskCompleted,
            None,
        );
    }

    pub fn record_subtask_failed(
        &mut self,
        plan_id: &str,
        subtask_id: &str,
        detail: Option<String>,
    ) {
        self.metrics.subtasks_failed.fetch_add(1, Ordering::Relaxed);
        self.record_event(
            plan_id,
            Some(subtask_id),
            AuditEventKind::SubtaskFailed,
            detail,
        );
    }

    pub fn record_plan_started(&mut self, plan_id: &str) {
        self.metrics.plans_started.fetch_add(1, Ordering::Relaxed);
        self.record_event(plan_id, None, AuditEventKind::PlanStarted, None);
    }

    pub fn record_plan_completed(&mut self, plan_id: &str) {
        self.metrics.plans_completed.fetch_add(1, Ordering::Relaxed);
        self.record_event(plan_id, None, AuditEventKind::PlanCompleted, None);
    }

    pub fn record_plan_failed(&mut self, plan_id: &str, detail: Option<String>) {
        self.metrics.plans_failed.fetch_add(1, Ordering::Relaxed);
        self.record_event(plan_id, None, AuditEventKind::PlanFailed, detail);
    }

    pub fn record_approval_requested(&mut self, plan_id: &str, subtask_id: &str) {
        self.metrics
            .approvals_requested
            .fetch_add(1, Ordering::Relaxed);
        self.record_event(
            plan_id,
            Some(subtask_id),
            AuditEventKind::ApprovalRequested,
            None,
        );
    }

    pub fn record_approval_resolved(
        &mut self,
        plan_id: &str,
        subtask_id: &str,
        detail: Option<String>,
    ) {
        self.metrics
            .approvals_resolved
            .fetch_add(1, Ordering::Relaxed);
        self.record_event(
            plan_id,
            Some(subtask_id),
            AuditEventKind::ApprovalResolved,
            detail,
        );
    }

    pub fn metrics_handle(&self) -> Arc<GovernanceMetrics> {
        self.metrics.clone()
    }

    pub fn metrics_snapshot(&self) -> GovernanceMetricsSnapshot {
        self.metrics.snapshot()
    }

    pub fn set_sla_deadline(&mut self, deadline_ms: u64) {
        self.sla_deadline_ms = Some(deadline_ms);
    }

    pub fn sla_deadline(&self) -> Option<u64> {
        self.sla_deadline_ms
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.audit
    }

    pub fn now_ms(&self) -> u64 {
        self.clock.now_millis()
    }

    pub fn record_event(
        &mut self,
        plan_id: &str,
        subtask_id: Option<&str>,
        event: AuditEventKind,
        detail: Option<String>,
    ) {
        let entry = AuditEntry {
            ts_ms: self.now_ms(),
            plan_id: plan_id.to_string(),
            subtask_id: subtask_id.map(|s| s.to_string()),
            event,
            detail,
        };
        self.record(entry);
    }
}

#[derive(Debug, Default)]
pub struct GovernanceMetrics {
    pub(crate) approvals_requested: AtomicU64,
    pub(crate) approvals_resolved: AtomicU64,
    pub(crate) approvals_escalated: AtomicU64,
    pub(crate) subtasks_started: AtomicU64,
    pub(crate) subtasks_completed: AtomicU64,
    pub(crate) subtasks_failed: AtomicU64,
    pub(crate) plans_started: AtomicU64,
    pub(crate) plans_completed: AtomicU64,
    pub(crate) plans_failed: AtomicU64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GovernanceMetricsSnapshot {
    pub approvals_requested: u64,
    pub approvals_resolved: u64,
    pub approvals_escalated: u64,
    pub subtasks_started: u64,
    pub subtasks_completed: u64,
    pub subtasks_failed: u64,
    pub plans_started: u64,
    pub plans_completed: u64,
    pub plans_failed: u64,
}

impl GovernanceMetrics {
    pub fn snapshot(&self) -> GovernanceMetricsSnapshot {
        GovernanceMetricsSnapshot {
            approvals_requested: self.approvals_requested.load(Ordering::Relaxed),
            approvals_resolved: self.approvals_resolved.load(Ordering::Relaxed),
            approvals_escalated: self.approvals_escalated.load(Ordering::Relaxed),
            subtasks_started: self.subtasks_started.load(Ordering::Relaxed),
            subtasks_completed: self.subtasks_completed.load(Ordering::Relaxed),
            subtasks_failed: self.subtasks_failed.load(Ordering::Relaxed),
            plans_started: self.plans_started.load(Ordering::Relaxed),
            plans_completed: self.plans_completed.load(Ordering::Relaxed),
            plans_failed: self.plans_failed.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Governance, GovernanceMetricsSnapshot};

    #[test]
    fn governance_metrics_increment_and_snapshot() {
        let mut gov = Governance::new();
        gov.record_plan_started("plan-1");
        gov.record_subtask_started("plan-1", "step-1");
        gov.record_subtask_completed("plan-1", "step-1");
        gov.record_approval_requested("plan-1", "step-1");
        gov.record_approval_resolved("plan-1", "step-1", None);
        gov.record_plan_completed("plan-1");

        let snapshot = gov.metrics_snapshot();
        assert_eq!(
            snapshot,
            GovernanceMetricsSnapshot {
                approvals_requested: 1,
                approvals_resolved: 1,
                approvals_escalated: 0,
                subtasks_started: 1,
                subtasks_completed: 1,
                subtasks_failed: 0,
                plans_started: 1,
                plans_completed: 1,
                plans_failed: 0,
            }
        );
        assert_eq!(gov.entries().len(), 6);
    }
}
