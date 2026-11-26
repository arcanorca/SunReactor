//! Gates that read hardware: external-change holds and integrity reasserts,
//! plus the ownership rules shared with the wake probe.

use super::plan::TargetFacts;
use super::summary::monitor_record;
use super::ApplySinks;
use crate::apply::dispatch::read_monitor_percent;
use crate::apply::health::{self, HardwareEvidence};
use crate::apply::types::{
    ApplyRecord, ApplySettings, ApplyStatus, ExternalHold, BLIND_REASSERT_INTERVAL_SECONDS,
    EXTERNAL_CHANGE_THRESHOLD_PCT, OWN_WRITE_GRACE_SECONDS, QUANTIZATION_LEARN_MAX_DELTA_PCT,
    QUANTIZATION_LEARN_WINDOW_SECONDS,
};
use crate::backends::{BackendObservation, ProcessRunner};
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
    /// The connector went through a power transition since the last judged
    /// readback: whatever the monitor shows may be a factory value.
    pub(super) power_transition: bool,
    /// Shortly after a recovery window: reset-looking values (0, 50, 100)
    /// are still resets.
    pub(super) reset_grace: bool,
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
            power_transition: false,
            reset_grace: false,
        }
    }

    pub(super) fn with_hardware(self, evidence: HardwareEvidence) -> Self {
        Self {
            power_transition: evidence.power_transition,
            reset_grace: evidence.reset_grace,
            ..self
        }
    }

    /// True when a power transition, or a reset-looking value right after a
    /// recovery event, explains `observed` without any person involved.
    pub(super) fn explains_reset(self, observed: u8) -> bool {
        self.power_transition || (self.reset_grace && is_reset_shaped(observed))
    }

    /// True when `observed` should start a new external-change hold: detection
    /// is enabled with a non-zero hold, the readback moved more than
    /// [`EXTERNAL_CHANGE_THRESHOLD_PCT`] from our last write, and neither our
    /// own recent write, a recovery window, nor a just-expired hold explains it.
    ///
    /// A 0% readback never counts: monitors report 0 after a standby /
    /// DisplayPort power-state reset (seen on Xiaomi Mi Monitor), and a
    /// person dimming to exactly 0% by hand is far rarer than that artefact.
    /// Treating it as a manual change would park the panel at 0% for the
    /// whole hold, so it is corrected like any other drift instead.
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
            && !is_reset_readback(observed)
            && last_applied_percent
                .is_some_and(|last| observed.abs_diff(last) > EXTERNAL_CHANGE_THRESHOLD_PCT)
            && !self.own_write_recent
            && !self.in_recovery
            && !self.explains_reset(observed)
            && !expired_hold
    }
}

/// A readback within this many percentage points of the value we wrote counts
/// as a match. Monitors quantize brightness to their own internal steps (the
/// Lenovo P24h reads 44 after a write of 43, forever), so an off-by-one is
/// hardware rounding, never drift or a user change; rewriting it only burns
/// DDC bus time and panel EEPROM cycles. Deliberately smaller than
/// [`EXTERNAL_CHANGE_THRESHOLD_PCT`], which gates holds.
pub(super) const QUANTIZATION_TOLERANCE_PCT: u8 = 1;

/// Pause between a suspicious readback and its confirmation read (and between
/// a correction write and its verification read). Long enough for a waking
/// monitor to settle, short enough to be harmless on the rare path that uses
/// it. Zero under test so unit tests stay instant.
const READBACK_SETTLE_DELAY: Duration = Duration::from_millis(500);

pub(super) fn readback_settle_delay() -> Duration {
    if cfg!(test) {
        Duration::ZERO
    } else {
        READBACK_SETTLE_DELAY
    }
}

/// True when `observed` equals `expected` up to
/// [`QUANTIZATION_TOLERANCE_PCT`].
pub(super) fn matches_within_quantization(observed: u8, expected: u8) -> bool {
    observed.abs_diff(expected) <= QUANTIZATION_TOLERANCE_PCT
}

/// Outcome of re-reading a value that looked like an external change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Confirmation {
    /// The second read agrees with the first (within the hold threshold) and
    /// still qualifies as an external change: hold at this value.
    Confirmed(u8),
    /// The second read disagrees: the first was a transient. Carries the
    /// fresher value so the caller can judge it on its own.
    Glitch(u8),
    /// No second read was available.
    Unknown,
}

/// Judges a confirmation read. `still_external` is the ownership predicate
/// evaluated on `second`; a single readback never starts a hold on its own,
/// because a waking monitor reports factory defaults and half-settled values.
pub(super) fn judge_confirmation(
    first: u8,
    second: Option<u8>,
    still_external: impl FnOnce(u8) -> bool,
) -> Confirmation {
    match second {
        None => Confirmation::Unknown,
        Some(second)
            if second.abs_diff(first) <= EXTERNAL_CHANGE_THRESHOLD_PCT
                && still_external(second) =>
        {
            Confirmation::Confirmed(second)
        }
        Some(second) => Confirmation::Glitch(second),
    }
}

/// A readback that means "the device lost its state", not "someone chose
/// this value". See [`Ownership::is_external_change`].
pub(super) fn is_reset_readback(observed: u8) -> bool {
    observed == 0
}

/// Values a monitor commonly wakes with after a power cycle: 0, its factory
/// 50, or its maximum (100%). Only ever a reset when a recovery or power
/// event explains it.
pub(super) fn is_reset_shaped(observed: u8) -> bool {
    matches!(observed, 0 | 50 | 100)
}

/// Baseline gate: the first write after the baseline was dropped (daemon
/// start, reconnect, resync) would otherwise rewrite whatever the panel
/// already shows. One cheap read first; when the hardware is already at the
/// target the baseline is adopted and no write is issued. A failed read
/// falls through to the write.
pub(super) fn baseline_gate<R: ProcessRunner>(
    facts: &TargetFacts<'_>,
    runner: &R,
    settings: &ApplySettings,
    sinks: &mut ApplySinks<'_>,
) -> ControlFlow<ApplyRecord> {
    // A forced apply of the value we already wrote (an idle wake that arrived
    // before the dim was written, a repeated slider position) needs one read,
    // not a write. Other forced applies keep writing unconditionally.
    let last_applied = facts.monitor_state.last_applied_percent;
    let redundant_forced = facts.forced && last_applied == Some(facts.requested_percent);
    let needs_read = if facts.forced {
        redundant_forced
    } else {
        last_applied.is_none()
    };
    if !needs_read {
        return ControlFlow::Continue(());
    }
    let monitor = facts.monitor;
    let Ok(observation) = read_monitor_percent(runner, monitor, settings) else {
        return ControlFlow::Continue(());
    };
    if !matches_within_quantization(observation.percent, facts.requested_percent) {
        return ControlFlow::Continue(());
    }
    if !redundant_forced {
        sinks.state.record_apply_success(
            &monitor.logical_id,
            facts.requested_percent,
            facts.now_epoch_s,
        );
    }
    ControlFlow::Break(ApplyRecord {
        attempts: 1,
        ..monitor_record(
            monitor,
            facts.requested_percent,
            observation.percent,
            ApplyStatus::SkippedHysteresis,
            format!(
                "hardware already at {}%; baseline adopted without a write",
                observation.percent
            ),
        )
    })
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
                engine.maybe_enqueue_with(
                    &monitor.logical_id,
                    observation.percent,
                    requested_percent,
                    facts.forced,
                )
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

    let read = read_monitor_percent(runner, monitor, settings);
    if read.is_ok() {
        // Consumed after the verdict below: `facts.ownership` already holds
        // the evidence this readback is judged with.
        health::acknowledge_readback(&monitor.logical_id);
    }
    match read {
        Ok(observation)
            if matches_within_quantization(observation.percent, requested_percent)
                || sinks.state.quantization_accepted(
                    &monitor.logical_id,
                    requested_percent,
                    observation.percent,
                ) =>
        {
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
            integrity_mismatch(facts, runner, settings, expired_hold, sinks, observation)?;
        }
        Err(error) => {
            tracing::debug!(
                logical_id = %monitor.logical_id,
                error = %error,
                "brightness_readback_failed; retaining_reassert_fallback"
            );
            // Blind reassert: nothing confirms the panel drifted, so keep it
            // rare instead of rewriting every reassert interval.
            let since_write = monitor_state
                .last_applied_at_epoch_s
                .map_or(u64::MAX, |at| now_epoch_s.saturating_sub(at));
            return ControlFlow::Continue(
                reassert_is_due && since_write >= BLIND_REASSERT_INTERVAL_SECONDS,
            );
        }
    }
    ControlFlow::Continue(reassert_is_due)
}

/// A reassert readback that differs from the target: record the check, then
/// start a confirmed external hold, accept a learned rounding, or leave the
/// drift to be corrected (`Continue`).
#[allow(clippy::too_many_lines)] // one linear decision ladder; splitting hides the order
fn integrity_mismatch<R: ProcessRunner>(
    facts: &TargetFacts<'_>,
    runner: &R,
    settings: &ApplySettings,
    expired_hold: bool,
    sinks: &mut ApplySinks<'_>,
    observation: BackendObservation,
) -> ControlFlow<ApplyRecord> {
    let monitor = facts.monitor;
    let monitor_state = &facts.monitor_state;
    let requested_percent = facts.requested_percent;
    let now_epoch_s = facts.now_epoch_s;
    sinks
        .state
        .record_integrity_check(&monitor.logical_id, now_epoch_s);
    let detection_enabled = sinks.external_holds.is_some();
    let is_external = |observed: u8| {
        facts.ownership.is_external_change(
            detection_enabled,
            settings.external_change_hold,
            monitor_state.last_applied_percent,
            observed,
            expired_hold,
        )
    };
    if is_external(observation.percent) {
        std::thread::sleep(readback_settle_delay());
        let second = read_monitor_percent(runner, monitor, settings)
            .ok()
            .map(|again| again.percent);
        match judge_confirmation(observation.percent, second, is_external) {
            Confirmation::Confirmed(confirmed) => {
                health::note_contest_event(
                    &monitor.logical_id,
                    now_epoch_s,
                    monitor_state.last_applied_percent,
                    monitor_state.last_applied_at_epoch_s,
                    confirmed,
                );
                let hold = new_hold(now_epoch_s, settings.external_change_hold, confirmed);
                if let Some(holds) = sinks.external_holds.as_deref_mut() {
                    holds.insert(monitor.logical_id.clone(), hold);
                }
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    observed_percent = confirmed,
                    until_epoch_s = hold.until_epoch_s,
                    "external_brightness_change_detected"
                );
                return ControlFlow::Break(ApplyRecord {
                    attempts: 2,
                    backoff_until_epoch_s: Some(hold.until_epoch_s),
                    ..monitor_record(
                        monitor,
                        requested_percent,
                        confirmed,
                        ApplyStatus::SkippedHysteresis,
                        String::from("external brightness change held"),
                    )
                });
            }
            Confirmation::Glitch(fresh)
                if matches_within_quantization(fresh, requested_percent) =>
            {
                return ControlFlow::Break(ApplyRecord {
                    attempts: 2,
                    ..monitor_record(
                        monitor,
                        requested_percent,
                        fresh,
                        ApplyStatus::SkippedHysteresis,
                        format!(
                            "integrity readback settled at {fresh}%; transient mismatch ignored"
                        ),
                    )
                });
            }
            Confirmation::Glitch(_) | Confirmation::Unknown => {
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    observed_percent = observation.percent,
                    "external_brightness_change_unconfirmed; treating_as_glitch"
                );
            }
        }
    }
    if requested_percent.abs_diff(observation.percent) <= QUANTIZATION_LEARN_MAX_DELTA_PCT
        && sinks.state.note_correction(
            &monitor.logical_id,
            requested_percent,
            observation.percent,
            now_epoch_s,
            QUANTIZATION_LEARN_WINDOW_SECONDS,
        )
    {
        tracing::info!(
            logical_id = %monitor.logical_id,
            requested_percent,
            observed_percent = observation.percent,
            "quantization_learned; accepting_readback"
        );
        return ControlFlow::Break(ApplyRecord {
            attempts: 1,
            ..monitor_record(
                monitor,
                requested_percent,
                observation.percent,
                ApplyStatus::SkippedHysteresis,
                format!(
                    "monitor rounds {requested_percent}% to {}%; readback accepted",
                    observation.percent
                ),
            )
        });
    }
    if !facts.ownership.explains_reset(observation.percent)
        && !facts.forced
        && health::note_contest_event(
            &monitor.logical_id,
            now_epoch_s,
            monitor_state.last_applied_percent,
            monitor_state.last_applied_at_epoch_s,
            observation.percent,
        )
    {
        let until = health::contested_until(&monitor.logical_id, now_epoch_s);
        return ControlFlow::Break(contested_record(monitor, requested_percent, until));
    }
    tracing::info!(
        logical_id = %monitor.logical_id,
        requested_percent,
        observed_percent = observation.percent,
        "brightness_drift_detected; correcting_to_current_policy"
    );
    ControlFlow::Continue(())
}

/// Skip record for a monitor another DDC controller is fighting us over.
pub(super) fn contested_record(
    monitor: &crate::config::MonitorConfig,
    requested_percent: u8,
    until_epoch_s: Option<u64>,
) -> ApplyRecord {
    let detail = match until_epoch_s {
        Some(until) => format!(
            "contested until {until}: another program keeps changing this monitor's brightness; \
             automatic correction paused"
        ),
        None => String::from(
            "contested: another program keeps changing this monitor's brightness; \
             automatic correction paused",
        ),
    };
    monitor_record(
        monitor,
        requested_percent,
        requested_percent,
        ApplyStatus::SkippedContested,
        detail,
    )
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
