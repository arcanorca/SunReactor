//! Settings resolution and the per-target gate chain.

use super::gate::{self, Ownership};
use super::summary::monitor_record;
use super::{ApplyInputs, ApplySinks};
use crate::apply::decision::{admit_write, AdmissionFacts, SkipReason, WritePath};
use crate::apply::health;
use crate::apply::types::{
    ApplyRecord, ApplySettings, ApplyStatus, MANUAL_MAX_WRITES_PER_WINDOW, MANUAL_MIN_GAP_SECONDS,
    MANUAL_WINDOW_SECONDS, WRITE_BUDGET_WINDOW_SECONDS,
};
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
    /// Explicit user/IPC apply: bypasses failure backoff and write budget.
    pub(super) forced: bool,
}

impl ResolvedSettings {
    /// Invariant: an override replaces the defaults wholesale and marks the run
    /// forced. Disabled smooth transitions cancel every in-flight fade before
    /// any target is planned, so a config reload takes effect immediately.
    pub(super) fn resolve<R>(inputs: &ApplyInputs<'_, R>, sinks: &mut ApplySinks<'_>) -> Self {
        let forced = inputs.settings_override.is_some();
        let settings = inputs.settings_override.unwrap_or(inputs.default_settings);
        if !settings.smooth_transition {
            if let Some(engine) = sinks.fade_engine.as_deref_mut() {
                engine.cancel_all();
            }
        }
        Self { settings, forced }
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

/// Facts about one target computed once, before readback gates run.
pub(super) struct TargetFacts<'a> {
    pub(super) monitor: &'a MonitorConfig,
    pub(super) requested_percent: u8,
    /// Snapshot taken before the gates run; gates that update runtime state do
    /// not change what later gates see.
    pub(super) monitor_state: MonitorRuntimeState,
    pub(super) ownership: Ownership,
    pub(super) now_epoch_s: u64,
    /// Explicit user action (IPC): exempt from the write budget and backoff.
    pub(super) forced: bool,
}

impl<'a> TargetFacts<'a> {
    fn observe(
        monitor: &'a MonitorConfig,
        requested_percent: u8,
        state: &RuntimeState,
        recovery_until_epoch_s: Option<u64>,
        now_epoch_s: u64,
        forced: bool,
    ) -> Self {
        let monitor_state = state
            .monitor(&monitor.logical_id)
            .cloned()
            .unwrap_or_default();
        let ownership =
            Ownership::observe(&monitor_state, recovery_until_epoch_s, now_epoch_s).with_hardware(
                health::observe_hardware(monitor, recovery_until_epoch_s, now_epoch_s),
            );
        Self {
            monitor,
            requested_percent,
            monitor_state,
            ownership,
            now_epoch_s,
            forced,
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

fn build_admission_facts<'a>(
    monitor: &'a MonitorConfig,
    topology_allowed: bool,
    active_hold_until_epoch_s: Option<u64>,
    monitor_state: Option<&'a MonitorRuntimeState>,
    settings: &ApplySettings,
    forced: bool,
    post_readback_in_recovery: Option<bool>,
) -> AdmissionFacts<'a> {
    AdmissionFacts {
        logical_id: &monitor.logical_id,
        backend: monitor.backend,
        enabled: monitor.enabled,
        topology_allowed,
        active_hold_until_epoch_s,
        last_applied_percent: monitor_state.and_then(|s| s.last_applied_percent),
        last_applied_at_epoch_s: monitor_state.and_then(|s| s.last_applied_at_epoch_s),
        backoff: monitor_state.and_then(|s| s.backoff.as_ref()),
        min_apply_interval: settings.min_apply_interval,
        in_recovery: post_readback_in_recovery.unwrap_or(false),
        budget_exempt: forced,
        fade_target_percent: None,
        check_rate_limits: post_readback_in_recovery.is_some(),
    }
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
    let path = if resolved.forced {
        WritePath::Forced
    } else {
        WritePath::Tick
    };
    let reconcile_action = inputs.reconcile_actions.get(position).copied();
    let topology_allowed = reconcile_action.is_none_or(ReconcileAction::allowed_to_apply);
    let active_hold = sinks
        .external_holds
        .as_deref()
        .and_then(|holds| holds.get(&monitor.logical_id).copied())
        .filter(|hold| hold.until_epoch_s > inputs.now_epoch_s);
    let snapshot_state = sinks.state.monitor(&monitor.logical_id);

    // Pre-readback admission: never touch hardware (not even sysfs or getvcp)
    // when topology, enablement, contested back-off, or an active hold skips it.
    let pre_facts = build_admission_facts(
        monitor,
        topology_allowed,
        active_hold.map(|hold| hold.until_epoch_s),
        snapshot_state,
        settings,
        resolved.forced,
        None,
    );
    if let Err(reason) = admit_write(path, &pre_facts, sinks.state, inputs.now_epoch_s) {
        return ControlFlow::Break(skip_record_for_reason(
            reason,
            monitor,
            requested_percent,
            requested_percent,
            inputs.now_epoch_s,
            reconcile_action,
            active_hold.map(|hold| hold.observed_percent),
            sinks,
        ));
    }

    let facts = TargetFacts::observe(
        monitor,
        requested_percent,
        sinks.state,
        inputs.recovery_until_epoch_s,
        inputs.now_epoch_s,
        resolved.forced,
    );
    let expired_hold = gate::external_hold_gate(&facts, inputs.runner, settings, sinks)?;
    let reassert_is_due =
        gate::integrity_gate(&facts, inputs.runner, settings, expired_hold, sinks)?;
    hysteresis_gate(&facts, settings, reassert_is_due)?;

    let applied_percent = step_limited_percent(&facts, settings);
    let post_facts = build_admission_facts(
        monitor,
        topology_allowed,
        None,
        Some(&facts.monitor_state),
        settings,
        facts.forced,
        Some(facts.ownership.in_recovery),
    );
    if let Err(reason) = admit_write(path, &post_facts, sinks.state, inputs.now_epoch_s) {
        return ControlFlow::Break(skip_record_for_reason(
            reason,
            monitor,
            requested_percent,
            applied_percent,
            inputs.now_epoch_s,
            reconcile_action,
            None,
            sinks,
        ));
    }

    if !settings.dry_run {
        gate::baseline_gate(&facts, inputs.runner, settings, sinks)?;
    }
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

    health::acknowledge_readback(&monitor.logical_id);
    if facts.forced {
        health::record_manual_write(&monitor.logical_id, facts.now_epoch_s);
    }
    ControlFlow::Continue(PlannedWrite {
        monitor,
        target,
        applied_percent,
        requested_percent,
    })
}

fn cancel_active_fade(sinks: &mut ApplySinks<'_>, logical_id: &str) {
    if let Some(engine) = sinks.fade_engine.as_deref_mut() {
        engine.active_fades.remove(logical_id);
    }
}

fn skip_record_for_reason(
    reason: SkipReason,
    monitor: &MonitorConfig,
    requested_percent: u8,
    applied_percent: u8,
    now_epoch_s: u64,
    reconcile_action: Option<ReconcileAction>,
    hold_observed_percent: Option<u8>,
    sinks: &mut ApplySinks<'_>,
) -> ApplyRecord {
    match reason {
        SkipReason::Topology => monitor_record(
            monitor,
            requested_percent,
            requested_percent,
            ApplyStatus::SkippedTopology,
            format!(
                "runtime topology reconciliation: {}",
                reconcile_action.map_or("unknown", ReconcileAction::name)
            ),
        ),
        SkipReason::Disabled => monitor_record(
            monitor,
            requested_percent,
            requested_percent,
            ApplyStatus::SkippedDisabled,
            String::from("monitor is disabled"),
        ),
        SkipReason::Contested { until_epoch_s } => {
            cancel_active_fade(sinks, &monitor.logical_id);
            gate::contested_record(monitor, requested_percent, Some(until_epoch_s))
        }
        SkipReason::Hold { until_epoch_s } => {
            cancel_active_fade(sinks, &monitor.logical_id);
            let remaining = until_epoch_s.saturating_sub(now_epoch_s);
            ApplyRecord {
                backoff_until_epoch_s: Some(until_epoch_s),
                ..monitor_record(
                    monitor,
                    requested_percent,
                    hold_observed_percent.unwrap_or(requested_percent),
                    ApplyStatus::SkippedHysteresis,
                    format!("external brightness hold active for {remaining}s"),
                )
            }
        }
        SkipReason::Backoff {
            remaining_s,
            suppress_until_epoch_s,
            failure_kind,
            consecutive_failures,
        } => ApplyRecord {
            failure_kind: Some(failure_kind),
            consecutive_failures: Some(consecutive_failures),
            backoff_until_epoch_s: Some(suppress_until_epoch_s),
            ..monitor_record(
                monitor,
                requested_percent,
                applied_percent,
                ApplyStatus::SkippedBackoff,
                format!("{failure_kind:?} failure backoff active for {remaining_s}s"),
            )
        },
        SkipReason::MinInterval {
            elapsed_s,
            min_interval_s,
        } => monitor_record(
            monitor,
            requested_percent,
            applied_percent,
            ApplyStatus::SkippedMinimumInterval,
            format!("last apply was {elapsed_s}s ago; minimum apply interval is {min_interval_s}s"),
        ),
        SkipReason::Budget { used, .. } => {
            tracing::warn!(logical_id = %monitor.logical_id, used, "write_budget_exhausted; deferring_to_latest_target");
            monitor_record(
                monitor,
                requested_percent,
                applied_percent,
                ApplyStatus::SkippedBudget,
                format!("write budget exhausted ({used} writes in {WRITE_BUDGET_WINDOW_SECONDS}s); deferred"),
            )
        }
        SkipReason::ManualCap { retry_at_epoch_s } => {
            tracing::debug!(logical_id = %monitor.logical_id, retry_at = retry_at_epoch_s, "forced_write_deferred; latest_value_applies_later");
            ApplyRecord {
                backoff_until_epoch_s: Some(retry_at_epoch_s),
                ..monitor_record(
                    monitor,
                    requested_percent,
                    applied_percent,
                    ApplyStatus::SkippedManualCap,
                    format!(
                        "forced write rate cap ({MANUAL_MAX_WRITES_PER_WINDOW} per {MANUAL_WINDOW_SECONDS}s, \
                         {MANUAL_MIN_GAP_SECONDS}s apart); deferred to {retry_at_epoch_s}"
                    ),
                )
            }
        }
        SkipReason::CorrectionInterval | SkipReason::AlreadyMatches => monitor_record(
            monitor,
            requested_percent,
            applied_percent,
            ApplyStatus::SkippedHysteresis,
            String::from("skipped"),
        ),
    }
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

/// Invariant: a delta below `min_write_delta_pct` is skipped unless an
/// integrity reassert is due.
fn hysteresis_gate(
    facts: &TargetFacts<'_>,
    settings: &ApplySettings,
    reassert_is_due: bool,
) -> ControlFlow<ApplyRecord> {
    let last_applied_percent = facts.monitor_state.last_applied_percent;
    let learned_rounding = facts
        .monitor_state
        .ledger
        .accepted_quantization
        .is_some_and(|(requested, observed)| {
            requested == facts.requested_percent && Some(observed) == last_applied_percent
        });
    let skip = learned_rounding
        || state::should_skip_hysteresis(
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
            engine.maybe_enqueue_with(
                &monitor.logical_id,
                start_percent,
                applied_percent,
                facts.forced,
            )
        });
    if !fade_queued {
        return ControlFlow::Continue(());
    }
    // No budget event here: every fade step is counted when it is written.
    if facts.forced {
        health::record_manual_write(&monitor.logical_id, facts.now_epoch_s);
    }
    health::acknowledge_readback(&monitor.logical_id);
    ControlFlow::Break(monitor_record(
        monitor,
        facts.requested_percent,
        applied_percent,
        ApplyStatus::Succeeded,
        format!("fading from {start_percent} to {applied_percent}"),
    ))
}
