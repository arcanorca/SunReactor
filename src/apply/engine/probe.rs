//! Wake probe: cheap readback-based drift reconciliation.

use super::gate::{new_hold, Ownership};
use super::ApplySinks;
use crate::apply::types::{ExternalHold, EXTERNAL_CHANGE_THRESHOLD_PCT};
use crate::backends::{BackendError, BackendObservation, BackendWrite};
use crate::config::MonitorConfig;
use crate::policy::{PerMonitorTarget, PolicyOutput};
use crate::runtime::fade::FadeEngine;
use crate::state::RuntimeState;
use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::time::Duration;

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
        monitors,
        policy,
        state,
        now_epoch_s,
        dry_run,
        Duration::ZERO,
        read,
        write,
        None,
        None,
        None,
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
/// Invariant: targets are probed sequentially in policy order; each enabled,
/// configured target is read exactly once and written at most once.
pub(crate) fn probe_and_correct_with_external_state(
    monitors: &[MonitorConfig],
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    now_epoch_s: u64,
    dry_run: bool,
    external_change_hold: Duration,
    read: impl Fn(&MonitorConfig) -> Result<Option<BackendObservation>, BackendError>,
    write: impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
    external_holds: Option<&mut BTreeMap<String, ExternalHold>>,
    recovery_until_epoch_s: Option<u64>,
    fade_engine: Option<&mut FadeEngine>,
) -> ProbeSummary {
    let env = ProbeEnv {
        now_epoch_s,
        dry_run,
        external_change_hold,
        recovery_until_epoch_s,
    };
    let mut sinks = ApplySinks {
        state,
        fade_engine,
        external_holds,
    };
    let mut summary = ProbeSummary::default();
    for target in &policy.targets {
        let Some(monitor) = monitors
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
                tracing::debug!(
                    logical_id = %monitor.logical_id,
                    error = %error,
                    "wake_probe_no_answer"
                );
                continue;
            }
        };
        summary.answered += 1;
        match reconcile_observation(monitor, target, observed, &env, &mut sinks, &write) {
            Correction::None => {}
            Correction::Corrected => summary.corrected += 1,
            Correction::Failed => summary.failed += 1,
        }
    }
    summary
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
    write: &impl Fn(&MonitorConfig, &PerMonitorTarget, u8) -> Result<BackendWrite, BackendError>,
) -> Correction {
    let requested = target.percent.min(100);
    let monitor_state = sinks
        .state
        .monitor(&monitor.logical_id)
        .cloned()
        .unwrap_or_default();
    let ownership = Ownership::observe(&monitor_state, env.recovery_until_epoch_s, env.now_epoch_s);

    let ControlFlow::Continue(expired_hold) = probe_hold_gate(
        &monitor.logical_id,
        observed,
        requested,
        ownership,
        env,
        sinks,
    ) else {
        return Correction::None;
    };

    if ownership.is_external_change(
        sinks.external_holds.is_some(),
        env.external_change_hold,
        monitor_state.last_applied_percent,
        observed,
        expired_hold,
    ) {
        let hold = new_hold(env.now_epoch_s, env.external_change_hold, observed);
        if let Some(holds) = sinks.external_holds.as_deref_mut() {
            holds.insert(monitor.logical_id.clone(), hold);
        }
        sinks
            .state
            .record_integrity_check(&monitor.logical_id, env.now_epoch_s);
        tracing::info!(
            logical_id = %monitor.logical_id,
            observed_percent = observed,
            until_epoch_s = hold.until_epoch_s,
            "external_brightness_change_detected"
        );
        return Correction::None;
    }

    if observed == requested {
        sinks
            .state
            .record_integrity_check(&monitor.logical_id, env.now_epoch_s);
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
    match write(monitor, target, requested) {
        Ok(result) => {
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

/// Probe-side external hold gate.
///
/// Invariant: while a hold is active the monitor is never written; a readback
/// that moved again (and is not our own lagging write) restarts the hold at
/// the new value. An expired hold is removed and, outside dry-run, may start
/// a fade from the readback. Every break records an integrity check.
/// Continues with `true` exactly when a hold expired during this call.
fn probe_hold_gate(
    logical_id: &str,
    observed: u8,
    requested: u8,
    ownership: Ownership,
    env: &ProbeEnv,
    sinks: &mut ApplySinks<'_>,
) -> ControlFlow<(), bool> {
    let Some(hold) = sinks
        .external_holds
        .as_deref()
        .and_then(|holds| holds.get(logical_id).copied())
    else {
        return ControlFlow::Continue(false);
    };

    if hold.until_epoch_s > env.now_epoch_s {
        if observed.abs_diff(hold.observed_percent) > EXTERNAL_CHANGE_THRESHOLD_PCT
            && !ownership.own_write_recent
        {
            if let Some(current) = sinks
                .external_holds
                .as_deref_mut()
                .and_then(|holds| holds.get_mut(logical_id))
            {
                current.until_epoch_s = env
                    .now_epoch_s
                    .saturating_add(env.external_change_hold.as_secs());
                current.observed_percent = observed;
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
