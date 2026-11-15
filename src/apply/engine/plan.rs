//! Settings resolution and the per-target gate chain.

use super::gate::{self, Ownership};
use super::summary::monitor_record;
use super::{ApplyInputs, ApplySinks};
use crate::apply::types::{ApplyRecord, ApplySettings, ApplyStatus};
use crate::backends::ProcessRunner;
use crate::config::MonitorConfig;
use crate::policy::PerMonitorTarget;
use crate::runtime::topology::ReconcileAction;
use crate::state::{self, MonitorRuntimeState, RuntimeState};
use std::collections::BTreeMap;
use std::ops::ControlFlow;

/// The settings one apply run actually uses.
pub(super) struct ResolvedSettings {
    pub(super) settings: ApplySettings,
    /// Manual overrides skip failure backoff.
    pub(super) bypass_backoff: bool,
}

impl ResolvedSettings {
    /// Invariant: an override replaces the defaults wholesale and bypasses
    /// backoff. Disabled smooth transitions cancel every in-flight fade before
    /// any target is planned, so a config reload takes effect immediately.
    pub(super) fn resolve<R>(inputs: &ApplyInputs<'_, R>, sinks: &mut ApplySinks<'_>) -> Self {
        let bypass_backoff = inputs.settings_override.is_some();
        let settings = inputs.settings_override.unwrap_or(inputs.default_settings);
        if !settings.smooth_transition {
            if let Some(engine) = sinks.fade_engine.as_deref_mut() {
                engine.cancel_all();
            }
        }
        Self {
            settings,
            bypass_backoff,
        }
    }
}

/// One planned hardware write.
#[derive(Debug, Clone, Copy)]
pub(super) struct PlannedWrite<'a> {
    pub(super) monitor: &'a MonitorConfig,
    pub(super) target: &'a PerMonitorTarget,
    pub(super) applied_percent: u8,
    pub(super) requested_percent: u8,
}

/// The plan for one policy target.
pub(super) enum WorkItem<'a> {
    /// Fully decided; no hardware write needed.
    Skip(ApplyRecord),
    /// Needs a hardware write.
    Dispatch(PlannedWrite<'a>),
}

/// Facts about one target computed once, before any gate runs.
pub(super) struct TargetFacts<'a> {
    pub(super) monitor: &'a MonitorConfig,
    pub(super) requested_percent: u8,
    /// Snapshot taken before the gates run; gates that update runtime state do
    /// not change what later gates see.
    pub(super) monitor_state: MonitorRuntimeState,
    pub(super) ownership: Ownership,
    pub(super) now_epoch_s: u64,
}

impl<'a> TargetFacts<'a> {
    fn observe(
        monitor: &'a MonitorConfig,
        requested_percent: u8,
        state: &RuntimeState,
        recovery_until_epoch_s: Option<u64>,
        now_epoch_s: u64,
    ) -> Self {
        let monitor_state = state
            .monitor(&monitor.logical_id)
            .cloned()
            .unwrap_or_default();
        let ownership = Ownership::observe(&monitor_state, recovery_until_epoch_s, now_epoch_s);
        Self {
            monitor,
            requested_percent,
            monitor_state,
            ownership,
            now_epoch_s,
        }
    }
}

/// Plans every policy target, sequentially and in policy order.
///
/// Invariant: the result has exactly one item per policy target, in policy
/// order. Gates run one target at a time, so hardware reads here are never
/// concurrent.
pub(super) fn plan_targets<'a, R: ProcessRunner>(
    inputs: &ApplyInputs<'a, R>,
    resolved: &ResolvedSettings,
    sinks: &mut ApplySinks<'_>,
) -> Vec<WorkItem<'a>> {
    // Duplicate logical ids resolve to the last configured monitor.
    let index = inputs
        .monitors
        .iter()
        .enumerate()
        .map(|(position, monitor)| (monitor.logical_id.as_str(), (position, monitor)))
        .collect::<BTreeMap<_, _>>();

    inputs
        .policy
        .targets
        .iter()
        .map(
            |target| match plan_target(inputs, resolved, &index, target, sinks) {
                ControlFlow::Break(record) => WorkItem::Skip(record),
                ControlFlow::Continue(write) => WorkItem::Dispatch(write),
            },
        )
        .collect()
}

/// Runs the gate chain for one target. The order of the gates is part of the
/// observable behavior (it decides which skip reason wins).
fn plan_target<'a, R: ProcessRunner>(
    inputs: &ApplyInputs<'a, R>,
    resolved: &ResolvedSettings,
    index: &BTreeMap<&str, (usize, &'a MonitorConfig)>,
    target: &'a PerMonitorTarget,
    sinks: &mut ApplySinks<'_>,
) -> ControlFlow<ApplyRecord, PlannedWrite<'a>> {
    let settings = &resolved.settings;
    let requested_percent = target.percent.min(100);
    let Some(&(position, monitor)) = index.get(target.logical_id.as_str()) else {
        return ControlFlow::Break(unmatched_target_record(target, requested_percent));
    };
    topology_gate(
        monitor,
        requested_percent,
        inputs.reconcile_actions.get(position),
    )?;
    if !monitor.enabled {
        return ControlFlow::Break(monitor_record(
            monitor,
            requested_percent,
            requested_percent,
            ApplyStatus::SkippedDisabled,
            String::from("monitor is disabled"),
        ));
    }

    let facts = TargetFacts::observe(
        monitor,
        requested_percent,
        sinks.state,
        inputs.recovery_until_epoch_s,
        inputs.now_epoch_s,
    );
    let expired_hold = gate::external_hold_gate(&facts, inputs.runner, settings, sinks)?;
    let reassert_is_due =
        gate::integrity_gate(&facts, inputs.runner, settings, expired_hold, sinks)?;
    hysteresis_gate(&facts, settings, reassert_is_due)?;

    let applied_percent = step_limited_percent(&facts, settings);
    if !resolved.bypass_backoff {
        backoff_gate(&facts, applied_percent)?;
    }
    min_interval_gate(&facts, settings, applied_percent)?;
    if settings.dry_run {
        return ControlFlow::Break(monitor_record(
            monitor,
            requested_percent,
            applied_percent,
            ApplyStatus::SkippedDryRun,
            String::from("dry_run is enabled"),
        ));
    }
    fade_gate(&facts, settings, applied_percent, sinks)?;

    ControlFlow::Continue(PlannedWrite {
        monitor,
        target,
        applied_percent,
        requested_percent,
    })
}

fn unmatched_target_record(target: &PerMonitorTarget, requested_percent: u8) -> ApplyRecord {
    ApplyRecord {
        logical_id: target.logical_id.clone(),
        backend: None,
        requested_percent,
        applied_percent: requested_percent,
        attempts: 0,
        failure_kind: None,
        consecutive_failures: None,
        backoff_until_epoch_s: None,
        status: ApplyStatus::Failed,
        detail: String::from("no matching monitor configuration for policy target"),
    }
}

/// Invariant: targets the runtime topology does not allow never reach a
/// hardware read or write. A monitor without a verdict is not gated.
fn topology_gate(
    monitor: &MonitorConfig,
    requested_percent: u8,
    action: Option<&ReconcileAction>,
) -> ControlFlow<ApplyRecord> {
    match action {
        Some(action) if !action.allowed_to_apply() => ControlFlow::Break(monitor_record(
            monitor,
            requested_percent,
            requested_percent,
            ApplyStatus::SkippedTopology,
            format!("runtime topology reconciliation: {}", action.name()),
        )),
        _ => ControlFlow::Continue(()),
    }
}

/// Invariant: a delta below `min_write_delta_pct` is skipped unless an
/// integrity reassert is due.
fn hysteresis_gate(
    facts: &TargetFacts<'_>,
    settings: &ApplySettings,
    reassert_is_due: bool,
) -> ControlFlow<ApplyRecord> {
    let last_applied_percent = facts.monitor_state.last_applied_percent;
    let skip = state::should_skip_hysteresis(
        last_applied_percent,
        facts.requested_percent,
        settings.min_write_delta_pct,
    );
    if !skip || reassert_is_due {
        return ControlFlow::Continue(());
    }
    let delta = last_applied_percent.map_or(0, |last| facts.requested_percent.abs_diff(last));
    ControlFlow::Break(monitor_record(
        facts.monitor,
        facts.requested_percent,
        facts.requested_percent,
        ApplyStatus::SkippedHysteresis,
        format!(
            "delta {delta} is below hysteresis threshold {}",
            settings.min_write_delta_pct
        ),
    ))
}

/// With smooth transitions disabled, deliberately skip the gradual per-tick
/// step limiter too: the dispatch is the one direct write to the requested
/// brightness target.
fn step_limited_percent(facts: &TargetFacts<'_>, settings: &ApplySettings) -> u8 {
    if settings.smooth_transition {
        state::limit_step_size(
            facts.monitor_state.last_applied_percent,
            facts.requested_percent,
            settings.max_step_pct_per_tick,
        )
    } else {
        facts.requested_percent
    }
}

/// Invariant: an unexpired backoff for the monitor's current backend skips
/// the write.
fn backoff_gate(facts: &TargetFacts<'_>, applied_percent: u8) -> ControlFlow<ApplyRecord> {
    let monitor = facts.monitor;
    let Some(backoff) = facts.monitor_state.backoff.as_ref().filter(|backoff| {
        backoff.backend == monitor.backend
            && backoff
                .suppress_until_epoch_s
                .is_some_and(|until| until > facts.now_epoch_s)
    }) else {
        return ControlFlow::Continue(());
    };
    let remaining = state::backoff_remaining(
        facts.monitor_state.backoff.as_ref(),
        monitor.backend,
        facts.now_epoch_s,
    )
    .unwrap_or_default();
    ControlFlow::Break(ApplyRecord {
        failure_kind: Some(backoff.failure_kind),
        consecutive_failures: Some(backoff.consecutive_failures),
        backoff_until_epoch_s: backoff.suppress_until_epoch_s,
        ..monitor_record(
            monitor,
            facts.requested_percent,
            applied_percent,
            ApplyStatus::SkippedBackoff,
            format!(
                "{:?} failure backoff active for {}s",
                backoff.failure_kind,
                remaining.as_secs()
            ),
        )
    })
}

/// Invariant: at most one write per monitor per `min_apply_interval`.
fn min_interval_gate(
    facts: &TargetFacts<'_>,
    settings: &ApplySettings,
    applied_percent: u8,
) -> ControlFlow<ApplyRecord> {
    let last_applied_at_epoch_s = facts.monitor_state.last_applied_at_epoch_s;
    if !state::write_interval_active(
        last_applied_at_epoch_s,
        facts.now_epoch_s,
        settings.min_apply_interval,
    ) {
        return ControlFlow::Continue(());
    }
    let elapsed_s = facts
        .now_epoch_s
        .saturating_sub(last_applied_at_epoch_s.unwrap_or(facts.now_epoch_s));
    ControlFlow::Break(monitor_record(
        facts.monitor,
        facts.requested_percent,
        applied_percent,
        ApplyStatus::SkippedMinimumInterval,
        format!(
            "last apply was {}s ago; minimum apply interval is {}s",
            elapsed_s,
            settings.min_apply_interval.as_secs()
        ),
    ))
}

/// Invariant: with smooth transitions on, a large enough step is handed to
/// the fade engine (starting from our last write) instead of being written
/// directly; the record reports success without a hardware attempt.
fn fade_gate(
    facts: &TargetFacts<'_>,
    settings: &ApplySettings,
    applied_percent: u8,
    sinks: &mut ApplySinks<'_>,
) -> ControlFlow<ApplyRecord> {
    let monitor = facts.monitor;
    let start_percent = facts
        .monitor_state
        .last_applied_percent
        .unwrap_or(facts.requested_percent);
    let fade_queued = settings.smooth_transition
        && sinks.fade_engine.as_deref_mut().is_some_and(|engine| {
            engine.maybe_enqueue(&monitor.logical_id, start_percent, applied_percent)
        });
    if !fade_queued {
        return ControlFlow::Continue(());
    }
    ControlFlow::Break(monitor_record(
        monitor,
        facts.requested_percent,
        applied_percent,
        ApplyStatus::Succeeded,
        format!("fading from {start_percent} to {applied_percent}"),
    ))
}
