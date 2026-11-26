//! Wake probe: cheap readback-based drift reconciliation.

use super::gate::{
    is_reset_readback, judge_confirmation, matches_within_quantization, new_hold,
    readback_settle_delay, Confirmation, Ownership,
};
use super::ApplySinks;
use crate::apply::decision::{
    admit_write, record_attempted_hardware_write, AdmissionFacts, SkipReason, WritePath,
};
use crate::apply::health;
use crate::apply::types::{
    ExternalHold, EXTERNAL_CHANGE_THRESHOLD_PCT, QUANTIZATION_LEARN_MAX_DELTA_PCT,
    QUANTIZATION_LEARN_WINDOW_SECONDS,
};
use crate::backends::{BackendError, BackendObservation, BackendWrite};
use crate::config::MonitorConfig;
use crate::policy::{PerMonitorTarget, PolicyOutput};
use crate::runtime::fade::FadeEngine;
use crate::state::RuntimeState;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;
use std::time::Duration;

/// Read-only inputs for one wake probe run.
pub(crate) struct ProbeInputs<'a> {
    pub monitors: &'a [MonitorConfig],
    pub policy: &'a PolicyOutput,
    pub now_epoch_s: u64,
    pub dry_run: bool,
    pub external_change_hold: Duration,
    pub recovery_until_epoch_s: Option<u64>,
}

/// Mutable runtime collaborators updated by a wake probe run.
pub(crate) struct ProbeSinks<'a> {
    pub state: &'a mut RuntimeState,
    pub external_holds: Option<&'a mut BTreeMap<String, ExternalHold>>,
    pub fade_engine: Option<&'a mut FadeEngine>,
    pub unreachable_monitors: Option<&'a mut BTreeSet<String>>,
}

/// What one wake probe found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProbeSummary {
    /// Monitors that answered a brightness read.
    pub(crate) answered: usize,
    /// Monitors that answered with the wrong value and were rewritten.
    pub(crate) corrected: usize,
    /// Rewrites that failed; the next probe tries again.
    pub(crate) failed: usize,
    /// Monitors that should answer but did not: asleep, or still waking.
    /// Monitors that cannot be probed cheaply are not counted here.
    pub(crate) unreachable: usize,
    /// Monitors that answered again after having been unreachable.
    pub(crate) reconnected: usize,
}

#[cfg(test)]
pub(super) fn probe_and_correct_with(
    monitors: &[MonitorConfig],
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    now_epoch_s: u64,
    dry_run: bool,
    read: impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
    write: impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
) -> ProbeSummary {
    probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors,
            policy,
            now_epoch_s,
            dry_run,
            external_change_hold: Duration::ZERO,
            recovery_until_epoch_s: None,
        },
        ProbeSinks {
            state,
            external_holds: None,
            fade_engine: None,
            unreachable_monitors: None,
        },
        read,
        write,
    )
}

/// A wake probe: reads each enabled monitor cheaply and reconciles brightness
/// drift, respecting any active external-change hold. I/O goes through the
/// `read`/`write` closures; `external_holds: None` disables external-change
/// tracking.
///
/// A monitor that does not answer yet (still waking, or not cheaply
/// readable) is skipped quietly with no backoff, so the next probe simply
/// tries again. Writes bypass step limits because the target is already the
/// value the policy wants now.
///
/// `unreachable_monitors` remembers, across probes, which monitors failed to
/// answer. A monitor that answers again is *reconnected*: its hardware state
/// is unknown (standby and DP power cycles reset it to 0, 50, 100 or a factory
/// value), so its stale hold and last-applied baseline are dropped and the
/// first readback is corrected rather than treated as a user choice.
///
/// Invariant: targets are probed sequentially in policy order; each enabled,
/// configured target is read once (plus one confirmation read when the value
/// looks like an external change, plus one verification read after a
/// correction) and written at most once. No retry loops.
pub(crate) fn probe_and_correct_with_external_state(
    inputs: &ProbeInputs<'_>,
    mut probe_sinks: ProbeSinks<'_>,
    read: impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
    write: impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
) -> ProbeSummary {
    let env = ProbeEnv {
        now_epoch_s: inputs.now_epoch_s,
        dry_run: inputs.dry_run,
        external_change_hold: inputs.external_change_hold,
        recovery_until_epoch_s: inputs.recovery_until_epoch_s,
    };
    let mut sinks = ApplySinks {
        state: probe_sinks.state,
        fade_engine: probe_sinks.fade_engine,
        external_holds: probe_sinks.external_holds,
    };
    let mut summary = ProbeSummary::default();
    for target in &inputs.policy.targets {
        let Some(monitor) = inputs
            .monitors
            .iter()
            .find(|monitor| monitor.logical_id == target.logical_id && monitor.enabled)
        else {
            continue;
        };
        let observed = match read(monitor) {
            Ok(Some(observation)) => observation.percent,
            Ok(None) => continue,
            Err(error) => {
                summary.unreachable += 1;
                // One failed read is noise. Only a powered-off connector or a
                // run of failures over time arms "reconnected" detection.
                if health::record_read_failure(monitor, inputs.now_epoch_s) {
                    if let Some(down) = probe_sinks.unreachable_monitors.as_deref_mut() {
                        down.insert(monitor.logical_id.clone());
                    }
                }
                tracing::debug!(
                    logical_id = %monitor.logical_id,
                    error = %error,
                    "wake_probe_no_answer"
                );
                continue;
            }
        };
        summary.answered += 1;
        health::record_read_success(&monitor.logical_id);
        if probe_sinks
            .unreachable_monitors
            .as_deref_mut()
            .is_some_and(|down| down.remove(&monitor.logical_id))
        {
            summary.reconnected += 1;
            treat_as_reconnected(&monitor.logical_id, observed, &mut sinks);
        }
        let pre_facts = AdmissionFacts {
            logical_id: &monitor.logical_id,
            backend: monitor.backend,
            enabled: monitor.enabled,
            topology_allowed: true,
            active_hold_until_epoch_s: None,
            last_applied_percent: None,
            last_applied_at_epoch_s: None,
            backoff: None,
            min_apply_interval: Duration::ZERO,
            in_recovery: false,
            budget_exempt: false,
            fade_target_percent: None,
            check_rate_limits: false,
        };
        if let Err(SkipReason::Contested { until_epoch_s }) = admit_write(
            WritePath::Probe,
            &pre_facts,
            sinks.state,
            inputs.now_epoch_s,
        ) {
            // Another controller keeps overriding us: hands off, like a hold.
            if let Some(engine) = sinks.fade_engine.as_deref_mut() {
                engine.active_fades.remove(&monitor.logical_id);
            }
            tracing::debug!(
                logical_id = %monitor.logical_id,
                until = until_epoch_s,
                "wake_probe_skipped; contested"
            );
            continue;
        }
        match reconcile_observation(monitor, target, observed, &env, &mut sinks, &read, &write) {
            Correction::None => {}
            Correction::Corrected => summary.corrected += 1,
            Correction::Failed => summary.failed += 1,
        }
    }
    summary
}

/// A monitor that stopped answering and is back: whatever we knew about its
/// hardware state is void. Dropping the last-applied baseline means no readback
/// can be blamed on a person ([`Ownership::is_external_change`] needs a
/// baseline), and dropping any hold releases a decision made about the
/// pre-power-cycle panel. The baseline is re-established by the next verified
/// write or matching readback.
///
/// Exception: when the first readback still equals the held value, the
/// monitor kept the user's choice through the outage, so hold and baseline stay.
fn treat_as_reconnected(logical_id: &str, observed: u8, sinks: &mut ApplySinks<'_>) {
    health::mark_power_event(logical_id);
    let kept_hold = sinks
        .external_holds
        .as_deref()
        .and_then(|holds| holds.get(logical_id))
        .is_some_and(|hold| matches_within_quantization(observed, hold.observed_percent));
    if kept_hold {
        tracing::info!(
            logical_id = %logical_id,
            observed_percent = observed,
            "monitor_reconnected; external_hold_kept (value unchanged)"
        );
        return;
    }
    if let Some(holds) = sinks.external_holds.as_deref_mut() {
        holds.remove(logical_id);
    }
    if sinks.state.monitor(logical_id).is_some() {
        let monitor_state = sinks.state.monitor_mut(logical_id);
        monitor_state.last_applied_percent = None;
        monitor_state.last_applied_at_epoch_s = None;
    }
    tracing::info!(logical_id = %logical_id, "monitor_reconnected; recovery_correction_armed");
}

/// Probe-wide parameters.
struct ProbeEnv {
    now_epoch_s: u64,
    dry_run: bool,
    external_change_hold: Duration,
    recovery_until_epoch_s: Option<u64>,
}

enum Correction {
    None,
    Corrected,
    Failed,
}

/// Decides what to do with one readback: respect or extend holds, start a
/// new hold, accept a match, or rewrite the policy target.
fn reconcile_observation(
    monitor: &MonitorConfig,
    target: &PerMonitorTarget,
    observed: u8,
    env: &ProbeEnv,
    sinks: &mut ApplySinks<'_>,
    read: &impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
    write: &impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
) -> Correction {
    let monitor_state = sinks
        .state
        .monitor(&monitor.logical_id)
        .cloned()
        .unwrap_or_default();
    let ownership = Ownership::observe(&monitor_state, env.recovery_until_epoch_s, env.now_epoch_s)
        .with_hardware(health::observe_hardware(
            monitor,
            env.recovery_until_epoch_s,
            env.now_epoch_s,
        ));
    let correction = judge_observation(
        monitor,
        target,
        observed,
        ownership,
        &monitor_state,
        env,
        sinks,
        read,
        write,
    );
    health::acknowledge_readback(&monitor.logical_id);
    correction
}

/// The decision ladder of [`reconcile_observation`], run with the hardware
/// evidence already folded into `ownership`.
fn judge_observation(
    monitor: &MonitorConfig,
    target: &PerMonitorTarget,
    observed: u8,
    ownership: Ownership,
    monitor_state: &crate::state::MonitorRuntimeState,
    env: &ProbeEnv,
    sinks: &mut ApplySinks<'_>,
    read: &impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
    write: &impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
) -> Correction {
    let requested = target.percent.min(100);
    let ControlFlow::Continue(expired_hold) = probe_hold_gate(
        monitor,
        &monitor.logical_id,
        observed,
        requested,
        ownership,
        env,
        sinks,
        read,
    ) else {
        return Correction::None;
    };

    let ControlFlow::Continue(observed) = confirm_suspected_change(
        monitor,
        observed,
        requested,
        ownership,
        monitor_state,
        expired_hold,
        env,
        sinks,
        read,
    ) else {
        return Correction::None;
    };

    if matches_within_quantization(observed, requested)
        || sinks
            .state
            .quantization_accepted(&monitor.logical_id, requested, observed)
    {
        sinks
            .state
            .record_integrity_check(&monitor.logical_id, env.now_epoch_s);
        if monitor_state.last_applied_percent.is_none() && !env.dry_run {
            // Re-establish the baseline a reconnect dropped.
            sinks
                .state
                .record_apply_success(&monitor.logical_id, requested, env.now_epoch_s);
        }
        return Correction::None;
    }

    tracing::info!(
        logical_id = %monitor.logical_id,
        requested_percent = requested,
        observed_percent = observed,
        "wake_probe_drift_detected"
    );
    if env.dry_run {
        return Correction::None;
    }
    if !ownership.explains_reset(observed)
        && health::note_contest_event(
            &monitor.logical_id,
            env.now_epoch_s,
            monitor_state.last_applied_percent,
            monitor_state.last_applied_at_epoch_s,
            observed,
        )
    {
        return Correction::None;
    }
    if requested.abs_diff(observed) <= QUANTIZATION_LEARN_MAX_DELTA_PCT
        && sinks.state.note_correction(
            &monitor.logical_id,
            requested,
            observed,
            env.now_epoch_s,
            QUANTIZATION_LEARN_WINDOW_SECONDS,
        )
    {
        // The identical correction was made moments ago and the monitor still
        // reads the same: it rounds this target. Stop rewriting it.
        tracing::info!(
            logical_id = %monitor.logical_id,
            requested_percent = requested,
            observed_percent = observed,
            "quantization_learned; accepting_readback"
        );
        sinks
            .state
            .record_integrity_check(&monitor.logical_id, env.now_epoch_s);
        return Correction::None;
    }
    let write_facts = probe_write_admission_facts(monitor, monitor_state, ownership.in_recovery);
    if let Err(reason) = admit_write(WritePath::Probe, &write_facts, sinks.state, env.now_epoch_s) {
        let used = match reason {
            SkipReason::Budget { used, .. } => used,
            _ => 0,
        };
        tracing::info!(
            logical_id = %monitor.logical_id,
            used,
            reason = ?reason,
            "wake_probe_correction_deferred; write_budget"
        );
        return Correction::None;
    }
    write_correction(monitor, target, requested, env, sinks, read, write)
}

fn probe_write_admission_facts<'a>(
    monitor: &'a MonitorConfig,
    monitor_state: &'a crate::state::MonitorRuntimeState,
    in_recovery: bool,
) -> AdmissionFacts<'a> {
    AdmissionFacts {
        logical_id: &monitor.logical_id,
        backend: monitor.backend,
        enabled: monitor.enabled,
        topology_allowed: true,
        active_hold_until_epoch_s: None,
        last_applied_percent: monitor_state.last_applied_percent,
        last_applied_at_epoch_s: monitor_state.last_applied_at_epoch_s,
        backoff: monitor_state.backoff.as_ref(),
        min_apply_interval: Duration::ZERO,
        in_recovery,
        budget_exempt: false,
        fade_target_percent: None,
        check_rate_limits: true,
    }
}

/// Issues the one correction write of this probe and verifies it with a read.
/// A write the monitor ignored is not recorded as applied.
fn write_correction(
    monitor: &MonitorConfig,
    target: &PerMonitorTarget,
    requested: u8,
    env: &ProbeEnv,
    sinks: &mut ApplySinks<'_>,
    read: &impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
    write: &impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
) -> Correction {
    let outcome = write(monitor, target, requested);
    record_attempted_hardware_write(
        sinks.state,
        &monitor.logical_id,
        env.now_epoch_s,
        &outcome,
        "hardware_write",
    );
    match outcome {
        Ok(result) => {
            if write_was_ignored(monitor, result.applied_percent, read) {
                // The monitor accepted the command but did not change (still
                // waking). Do not claim success: the baseline stays as it was
                // and the next probe retries.
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    requested_percent = requested,
                    "wake_probe_write_not_applied; will_retry_next_probe"
                );
                return Correction::Failed;
            }
            sinks.state.record_apply_success(
                &monitor.logical_id,
                result.applied_percent,
                env.now_epoch_s,
            );
            tracing::info!(
                logical_id = %monitor.logical_id,
                applied_percent = result.applied_percent,
                "wake_probe_corrected"
            );
            Correction::Corrected
        }
        Err(error) => {
            tracing::debug!(
                logical_id = %monitor.logical_id,
                error = %error,
                "wake_probe_write_failed"
            );
            Correction::Failed
        }
    }
}

/// Confirm-before-hold: a deviation that looks like a manual change is
/// re-read once after a short settle. Confirmed -> hold (`Break`); a value
/// that moved is a glitch and continues as the fresher reading; an unreadable
/// confirmation skips this probe (`Break`) without hold or write.
fn confirm_suspected_change(
    monitor: &MonitorConfig,
    observed: u8,
    requested: u8,
    ownership: Ownership,
    monitor_state: &crate::state::MonitorRuntimeState,
    expired_hold: bool,
    env: &ProbeEnv,
    sinks: &mut ApplySinks<'_>,
    read: &impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
) -> ControlFlow<(), u8> {
    let mut observed = observed;
    if !matches_within_quantization(observed, requested)
        && ownership_blames_user(ownership, sinks, env, monitor_state, observed, expired_hold)
    {
        // A single readback is never enough to hold: re-read once after a
        // short settle and require the same deviation.
        std::thread::sleep(readback_settle_delay());
        let second = match read(monitor) {
            Ok(Some(observation)) => Some(observation.percent),
            _ => None,
        };
        let blames = |value: u8| {
            ownership_blames_user(ownership, sinks, env, monitor_state, value, expired_hold)
        };
        match judge_confirmation(observed, second, blames) {
            Confirmation::Confirmed(confirmed) => {
                health::note_contest_event(
                    &monitor.logical_id,
                    env.now_epoch_s,
                    monitor_state.last_applied_percent,
                    monitor_state.last_applied_at_epoch_s,
                    confirmed,
                );
                let hold = new_hold(env.now_epoch_s, env.external_change_hold, confirmed);
                if let Some(holds) = sinks.external_holds.as_deref_mut() {
                    holds.insert(monitor.logical_id.clone(), hold);
                }
                sinks
                    .state
                    .record_integrity_check(&monitor.logical_id, env.now_epoch_s);
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    observed_percent = confirmed,
                    until_epoch_s = hold.until_epoch_s,
                    "external_brightness_change_detected"
                );
                return ControlFlow::Break(());
            }
            Confirmation::Glitch(fresh) => {
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    first_percent = observed,
                    second_percent = fresh,
                    "external_brightness_change_unconfirmed; treating_as_glitch"
                );
                observed = fresh;
            }
            Confirmation::Unknown => {
                // Cannot tell a person from a glitch and cannot read again:
                // neither hold nor write; the next probe decides.
                tracing::info!(
                    logical_id = %monitor.logical_id,
                    observed_percent = observed,
                    "external_brightness_change_unconfirmed; skipping"
                );
                return ControlFlow::Break(());
            }
        }
    }
    ControlFlow::Continue(observed)
}

fn ownership_blames_user(
    ownership: Ownership,
    sinks: &ApplySinks<'_>,
    env: &ProbeEnv,
    monitor_state: &crate::state::MonitorRuntimeState,
    observed: u8,
    expired_hold: bool,
) -> bool {
    ownership.is_external_change(
        sinks.external_holds.is_some(),
        env.external_change_hold,
        monitor_state.last_applied_percent,
        observed,
        expired_hold,
    )
}

/// After a correction write, reads back once (where the backend supports a
/// cheap read) and reports whether the value clearly did not take. A failed or
/// unsupported read counts as "cannot tell", i.e. not ignored.
fn write_was_ignored(
    monitor: &MonitorConfig,
    expected: u8,
    read: &impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
) -> bool {
    std::thread::sleep(readback_settle_delay());
    matches!(
        read(monitor),
        Ok(Some(observation)) if !matches_within_quantization(observation.percent, expected)
    )
}

/// Probe-side external hold gate.
///
/// Invariant: while a hold is active the monitor is never written; a readback
/// that moved again (and is not our own lagging write) restarts the hold at
/// the new value only after a confirmation read agrees (see
/// [`judge_confirmation`]); otherwise the hold is left untouched. An expired hold is removed and, outside dry-run, may start
/// a fade from the readback. Every break records an integrity check.
/// Continues with `true` exactly when a hold expired during this call.
fn probe_hold_gate(
    monitor: &MonitorConfig,
    logical_id: &str,
    observed: u8,
    requested: u8,
    ownership: Ownership,
    env: &ProbeEnv,
    sinks: &mut ApplySinks<'_>,
    read: &impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
) -> ControlFlow<(), bool> {
    let Some(hold) = sinks
        .external_holds
        .as_deref()
        .and_then(|holds| holds.get(logical_id).copied())
    else {
        return ControlFlow::Continue(false);
    };

    let lost_user_value = observed.abs_diff(hold.observed_percent) > EXTERNAL_CHANGE_THRESHOLD_PCT
        && ownership.explains_reset(observed);
    if hold.until_epoch_s > env.now_epoch_s && (is_reset_readback(observed) || lost_user_value) {
        // The monitor dropped to 0% under an active hold: it was reset
        // (standby / link power cycle), not adjusted. The held value is gone
        // from the hardware, so release the hold and restore the policy.
        if let Some(holds) = sinks.external_holds.as_deref_mut() {
            holds.remove(logical_id);
        }
        tracing::info!(
            logical_id = %logical_id,
            held_percent = hold.observed_percent,
            "external_hold_released_after_reset_readback; power_transition_or_reset"
        );
        return ControlFlow::Continue(true);
    }

    if hold.until_epoch_s > env.now_epoch_s {
        if observed.abs_diff(hold.observed_percent) > EXTERNAL_CHANGE_THRESHOLD_PCT
            && !ownership.own_write_recent
        {
            // A single deviating readback never moves or extends a hold: it
            // must survive the same confirmation read that starts one. An
            // unconfirmed or flapping value leaves the hold exactly as it was.
            std::thread::sleep(readback_settle_delay());
            let second = match read(monitor) {
                Ok(Some(observation)) => Some(observation.percent),
                _ => None,
            };
            let moved = |value: u8| {
                value.abs_diff(hold.observed_percent) > EXTERNAL_CHANGE_THRESHOLD_PCT
                    && !is_reset_readback(value)
            };
            if let Confirmation::Confirmed(confirmed) = judge_confirmation(observed, second, moved)
            {
                if let Some(current) = sinks
                    .external_holds
                    .as_deref_mut()
                    .and_then(|holds| holds.get_mut(logical_id))
                {
                    current.until_epoch_s = env
                        .now_epoch_s
                        .saturating_add(env.external_change_hold.as_secs());
                    current.observed_percent = confirmed;
                }
            } else {
                tracing::info!(
                    logical_id = %logical_id,
                    observed_percent = observed,
                    held_percent = hold.observed_percent,
                    "external_hold_extension_unconfirmed; hold_unchanged"
                );
            }
        }
        sinks
            .state
            .record_integrity_check(logical_id, env.now_epoch_s);
        return ControlFlow::Break(());
    }

    if let Some(holds) = sinks.external_holds.as_deref_mut() {
        holds.remove(logical_id);
    }
    if !env.dry_run
        && sinks
            .fade_engine
            .as_deref_mut()
            .is_some_and(|engine| engine.maybe_enqueue(logical_id, observed, requested))
    {
        sinks
            .state
            .record_integrity_check(logical_id, env.now_epoch_s);
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(true)
}
