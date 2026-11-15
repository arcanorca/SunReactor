//! Gates that read hardware: external-change holds and integrity reasserts,
//! plus the ownership rules shared with the wake probe.

use super::plan::TargetFacts;
use super::summary::monitor_record;
use super::ApplySinks;
use crate::apply::dispatch::read_monitor_percent;
use crate::apply::types::{
    ApplyRecord, ApplySettings, ApplyStatus, ExternalHold, EXTERNAL_CHANGE_THRESHOLD_PCT,
    OWN_WRITE_GRACE_SECONDS,
};
use crate::backends::ProcessRunner;
use crate::state::MonitorRuntimeState;
use std::ops::ControlFlow;
use std::time::Duration;

/// Whether a readback that differs from our last write may be blamed on
/// another controller.
#[derive(Debug, Clone, Copy)]
pub(super) struct Ownership {
    /// Our own write landed within [`OWN_WRITE_GRACE_SECONDS`]; hardware may
    /// still lag behind it.
    pub(super) own_write_recent: bool,
    /// A display recovery window is open: drift is ours to correct.
    pub(super) in_recovery: bool,
}

impl Ownership {
    pub(super) fn observe(
        monitor_state: &MonitorRuntimeState,
        recovery_until_epoch_s: Option<u64>,
        now_epoch_s: u64,
    ) -> Self {
        Self {
            own_write_recent: monitor_state
                .last_applied_at_epoch_s
                .is_some_and(|at| now_epoch_s < at.saturating_add(OWN_WRITE_GRACE_SECONDS)),
            in_recovery: recovery_until_epoch_s.is_some_and(|until| now_epoch_s < until),
        }
    }

    /// True when `observed` should start a new external-change hold: detection
    /// is enabled with a non-zero hold, the readback moved more than
    /// [`EXTERNAL_CHANGE_THRESHOLD_PCT`] from our last write, and neither our
    /// own recent write, a recovery window, nor a just-expired hold explains it.
    pub(super) fn is_external_change(
        self,
        detection_enabled: bool,
        hold: Duration,
        last_applied_percent: Option<u8>,
        observed: u8,
        expired_hold: bool,
    ) -> bool {
        detection_enabled
            && hold.as_secs() > 0
            && last_applied_percent
                .is_some_and(|last| observed.abs_diff(last) > EXTERNAL_CHANGE_THRESHOLD_PCT)
            && !self.own_write_recent
            && !self.in_recovery
            && !expired_hold
    }
}

pub(super) fn new_hold(now_epoch_s: u64, hold: Duration, observed_percent: u8) -> ExternalHold {
    ExternalHold {
        since_epoch_s: now_epoch_s,
        until_epoch_s: now_epoch_s.saturating_add(hold.as_secs()),
        observed_percent,
    }
}

/// External hold gate.
///
/// Invariant: an active hold always skips the target and drops its fade; an
/// expired hold is removed and, outside dry-run, a fresh readback may start a
/// fade from the current hardware value (one read, no write). Continues with
/// `true` exactly when a hold expired during this call.
pub(super) fn external_hold_gate<R: ProcessRunner>(
    facts: &TargetFacts<'_>,
    runner: &R,
    settings: &ApplySettings,
    sinks: &mut ApplySinks<'_>,
) -> ControlFlow<ApplyRecord, bool> {
    let monitor = facts.monitor;
    let requested_percent = facts.requested_percent;
    let Some(hold) = sinks
        .external_holds
        .as_deref()
        .and_then(|holds| holds.get(&monitor.logical_id).copied())
    else {
        return ControlFlow::Continue(false);
    };

    if hold.until_epoch_s > facts.now_epoch_s {
        if let Some(engine) = sinks.fade_engine.as_deref_mut() {
            engine.active_fades.remove(&monitor.logical_id);
        }
        return ControlFlow::Break(ApplyRecord {
            backoff_until_epoch_s: Some(hold.until_epoch_s),
            ..monitor_record(
                monitor,
                requested_percent,
                hold.observed_percent,
                ApplyStatus::SkippedHysteresis,
                format!(
                    "external brightness hold active for {}s",
                    hold.until_epoch_s.saturating_sub(facts.now_epoch_s)
                ),
            )
        });
    }

    if let Some(holds) = sinks.external_holds.as_deref_mut() {
        holds.remove(&monitor.logical_id);
    }

    if !settings.dry_run {
        if let Ok(observation) = read_monitor_percent(runner, monitor, settings) {
            if sinks.fade_engine.as_deref_mut().is_some_and(|engine| {
                engine.maybe_enqueue(&monitor.logical_id, observation.percent, requested_percent)
            }) {
                return ControlFlow::Break(ApplyRecord {
                    attempts: 1,
                    ..monitor_record(
                        monitor,
                        requested_percent,
                        requested_percent,
                        ApplyStatus::Succeeded,
                        format!(
                            "external hold expired; fading from {} to {}",
                            observation.percent, requested_percent
                        ),
                    )
                });
            }
        }
    }
    ControlFlow::Continue(true)
}

/// Integrity reassert gate.
///
/// Invariant: hardware is read only when a reassert is due *and* the target
/// equals our last write. A matching readback skips the write; a mismatching
/// one either starts an external hold (skip) or falls through to correct the
/// drift; a failed read falls through to the reassert write. Every successful
/// read records an integrity check. Continues with whether a reassert is due.
pub(super) fn integrity_gate<R: ProcessRunner>(
    facts: &TargetFacts<'_>,
    runner: &R,
    settings: &ApplySettings,
    expired_hold: bool,
    sinks: &mut ApplySinks<'_>,
) -> ControlFlow<ApplyRecord, bool> {
    let monitor = facts.monitor;
    let monitor_state = &facts.monitor_state;
    let requested_percent = facts.requested_percent;
    let now_epoch_s = facts.now_epoch_s;
    let integrity_reference_epoch_s = monitor_state
        .last_integrity_check_at_epoch_s
        .or(monitor_state.last_applied_at_epoch_s);
    let reassert_is_due = monitor_state.last_applied_percent.is_some()
        && reassert_due(
            integrity_reference_epoch_s,
            now_epoch_s,
            settings.apply_reassert_interval.as_secs(),
        );

    if !(reassert_is_due && monitor_state.last_applied_percent == Some(requested_percent)) {
        return ControlFlow::Continue(reassert_is_due);
    }

    match read_monitor_percent(runner, monitor, settings) {
        Ok(observation) if observation.percent == requested_percent => {
            sinks
                .state
                .record_integrity_check(&monitor.logical_id, now_epoch_s);
            return ControlFlow::Break(ApplyRecord {
                attempts: 1,
                ..monitor_record(
                    monitor,
                    requested_percent,
                    observation.percent,
                    ApplyStatus::SkippedHysteresis,
                    format!(
                        "integrity readback matches target at {}%; no write required",
                        observation.percent
                    ),
                )
            });
        }
        Ok(observation) => {
            sinks
                .state
                .record_integrity_check(&monitor.logical_id, now_epoch_s);
            if facts.ownership.is_external_change(
                sinks.external_holds.is_some(),
                settings.external_change_hold,
                monitor_state.last_applied_percent,
                observation.percent,
                expired_hold,
            ) {
                let hold = new_hold(
                    now_epoch_s,
                    settings.external_change_hold,
                    observation.percent,
                );
                if let Some(holds) = sinks.external_holds.as_deref_mut() {
                    holds.insert(monitor.logical_id.clone(), hold);
                }
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    observed_percent = observation.percent,
                    until_epoch_s = hold.until_epoch_s,
                    "external_brightness_change_detected"
                );
                return ControlFlow::Break(ApplyRecord {
                    attempts: 1,
                    backoff_until_epoch_s: Some(hold.until_epoch_s),
                    ..monitor_record(
                        monitor,
                        requested_percent,
                        observation.percent,
                        ApplyStatus::SkippedHysteresis,
                        String::from("external brightness change held"),
                    )
                });
            }
            tracing::info!(
                logical_id = %monitor.logical_id,
                requested_percent,
                observed_percent = observation.percent,
                "brightness_drift_detected; correcting_to_current_policy"
            );
        }
        Err(error) => {
            tracing::debug!(
                logical_id = %monitor.logical_id,
                error = %error,
                "brightness_readback_failed; retaining_reassert_fallback"
            );
        }
    }
    ControlFlow::Continue(reassert_is_due)
}

fn reassert_due(
    last_applied_at_epoch_s: Option<u64>,
    now_epoch_s: u64,
    reassert_interval_seconds: u64,
) -> bool {
    last_applied_at_epoch_s.is_none_or(|last_applied_at| {
        now_epoch_s.saturating_sub(last_applied_at) >= reassert_interval_seconds
    })
}
