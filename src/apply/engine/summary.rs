//! Commit phase: folds outcomes into runtime state and the apply summary.

use super::plan::PlannedWrite;
use super::write::Outcome;
use crate::apply::types::{ApplyRecord, ApplyStatus, ApplySummary};
use crate::backends::{BackendError, BackendWrite};
use crate::config::MonitorConfig;
use crate::state::RuntimeState;

/// A record for `monitor` with no hardware attempt, failure or backoff; gates
/// override individual fields with struct-update syntax.
pub(super) fn monitor_record(
    monitor: &MonitorConfig,
    requested_percent: u8,
    applied_percent: u8,
    status: ApplyStatus,
    detail: String,
) -> ApplyRecord {
    ApplyRecord {
        logical_id: monitor.logical_id.clone(),
        backend: Some(monitor.backend),
        requested_percent,
        applied_percent,
        attempts: 0,
        failure_kind: None,
        consecutive_failures: None,
        backoff_until_epoch_s: None,
        status,
        detail,
    }
}

/// Invariant: records keep plan (= policy) order, and runtime state changes
/// only for targets that were actually written: success clears backoff and
/// stamps the write, failure extends backoff.
pub(super) fn commit(
    outcomes: Vec<Outcome<'_>>,
    state: &mut RuntimeState,
    now_epoch_s: u64,
) -> ApplySummary {
    let mut summary = ApplySummary::default();
    for outcome in outcomes {
        let record = match outcome {
            Outcome::Skipped(record) => record,
            Outcome::Written { write, result } => commit_write(write, result, state, now_epoch_s),
        };
        summary.push(record);
    }
    summary
}

fn commit_write(
    write: PlannedWrite<'_>,
    result: Result<BackendWrite, BackendError>,
    state: &mut RuntimeState,
    now_epoch_s: u64,
) -> ApplyRecord {
    let monitor = write.monitor;
    crate::apply::decision::record_attempted_hardware_write(
        state,
        &monitor.logical_id,
        now_epoch_s,
        &result,
        "hardware_write",
    );
    match result {
        Ok(done) => {
            state.record_apply_success(&monitor.logical_id, done.applied_percent, now_epoch_s);
            ApplyRecord {
                logical_id: monitor.logical_id.clone(),
                backend: Some(done.backend),
                requested_percent: write.requested_percent,
                applied_percent: done.applied_percent,
                attempts: done.attempts,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::Succeeded,
                detail: done.detail,
            }
        }
        Err(error) => {
            let attempts = error.attempts();
            let backoff = state.record_apply_failure(
                &monitor.logical_id,
                monitor.backend,
                error.failure_kind(),
                now_epoch_s,
            );
            let detail = format!(
                "{}; backing off for {}s after {} consecutive {:?} failure(s)",
                error,
                backoff
                    .suppress_until_epoch_s
                    .unwrap_or(now_epoch_s)
                    .saturating_sub(now_epoch_s),
                backoff.consecutive_failures,
                backoff.failure_kind
            );
            ApplyRecord {
                attempts,
                failure_kind: Some(backoff.failure_kind),
                consecutive_failures: Some(backoff.consecutive_failures),
                backoff_until_epoch_s: backoff.suppress_until_epoch_s,
                ..monitor_record(
                    monitor,
                    write.requested_percent,
                    write.applied_percent,
                    ApplyStatus::Failed,
                    detail,
                )
            }
        }
    }
}
