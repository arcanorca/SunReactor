//! The apply pipeline: turns one [`PolicyOutput`] into backend writes.
//!
//! One run has four phases, always in this order:
//!
//! 1. **resolve** ([`plan::ResolvedSettings`]) — pick the effective settings
//!    and cancel fades when smooth transitions are off;
//! 2. **plan** ([`plan::plan_targets`]) — walk policy targets sequentially, in
//!    policy order, through the gate chain; gates may read hardware and update
//!    runtime state, holds and fades;
//! 3. **write** ([`write::dispatch_all`]) — run every planned write
//!    concurrently, one scoped thread per target;
//! 4. **commit** ([`summary::commit`]) — fold the outcomes into runtime state
//!    and an [`ApplySummary`], again in policy order.
//!
//! The wake probe ([`probe_and_correct_with_external_state`]) is a separate,
//! cheaper reconciliation loop that shares the external-change rules in
//! [`gate`].

mod gate;
mod plan;
mod probe;
mod summary;
#[cfg(test)]
mod tests;
mod write;

#[cfg(test)]
use probe::probe_and_correct_with;
pub(crate) use probe::{probe_and_correct_with_external_state, ProbeInputs, ProbeSinks};

use super::types::{ApplySettings, ApplySummary, ExternalHold};
use crate::backends::{ProcessRunner, RealProcessRunner};
use crate::config::{Config, MonitorConfig};
use crate::policy::PolicyOutput;
use crate::runtime::fade::FadeEngine;
use crate::runtime::topology::{CapabilitySnapshot, ReconcileAction};
use crate::state::{self, RuntimeState};
use std::collections::BTreeMap;

pub fn apply_policy(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
) -> ApplySummary {
    apply_policy_at(config, policy, state, state::current_epoch_s())
}

pub fn apply_policy_at(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    now_epoch_s: u64,
) -> ApplySummary {
    apply_policy_with_runner(config, policy, state, &RealProcessRunner, now_epoch_s)
}

pub(crate) fn apply_policy_with_runner<R: ProcessRunner + Sync>(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    runner: &R,
    now_epoch_s: u64,
) -> ApplySummary {
    apply_policy_with_runner_settings(config, policy, state, None, runner, now_epoch_s, None)
}

/// Applies only configured monitors classified as present by a fresh runtime
/// capability snapshot. The snapshot is not persisted and does not mutate
/// user config; it gates this effective apply set only.
#[cfg(test)]
pub(crate) fn apply_policy_with_runner_reconciled<R: ProcessRunner + Sync>(
    monitors: &[MonitorConfig],
    settings: ApplySettings,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    runner: &R,
    now_epoch_s: u64,
    capabilities: &CapabilitySnapshot,
    fade_engine: Option<&mut FadeEngine>,
    settings_override: Option<ApplySettings>,
) -> ApplySummary {
    apply_policy_with_runner_reconciled_external(
        monitors,
        settings,
        policy,
        state,
        runner,
        now_epoch_s,
        capabilities,
        fade_engine,
        settings_override,
        None,
        None,
    )
}

/// Reconciles `monitors` against `capabilities`, then applies `policy` to the
/// monitors the topology allows, honouring external-change holds.
pub(crate) fn apply_policy_with_runner_reconciled_external<R: ProcessRunner + Sync>(
    monitors: &[MonitorConfig],
    settings: ApplySettings,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    runner: &R,
    now_epoch_s: u64,
    capabilities: &CapabilitySnapshot,
    fade_engine: Option<&mut FadeEngine>,
    settings_override: Option<ApplySettings>,
    external_holds: Option<&mut BTreeMap<String, ExternalHold>>,
    recovery_until_epoch_s: Option<u64>,
) -> ApplySummary {
    let actions = crate::runtime::topology::reconcile(monitors, capabilities);
    let inputs = ApplyInputs {
        monitors,
        policy,
        runner,
        now_epoch_s,
        default_settings: settings,
        settings_override,
        reconcile_actions: &actions,
        recovery_until_epoch_s,
    };
    run_apply(
        &inputs,
        ApplySinks {
            state,
            fade_engine,
            external_holds,
        },
    )
}

/// Apply policy with an optional settings override. When `settings_override`
/// is `Some`, the provided settings are used instead of deriving them from
/// config. This lets IPC `RunOnce` bypass throttles (hysteresis, step limit,
/// minimum interval, and failure backoff) so manual brightness overrides apply
/// immediately.
pub(crate) fn apply_policy_with_runner_settings<R: ProcessRunner + Sync>(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    fade_engine: Option<&mut FadeEngine>,
    runner: &R,
    now_epoch_s: u64,
    settings_override: Option<ApplySettings>,
) -> ApplySummary {
    apply_policy_with_runner_monitors(
        &config.monitors,
        ApplySettings::from_config(config),
        policy,
        state,
        fade_engine,
        runner,
        now_epoch_s,
        settings_override,
    )
}

/// Applies `policy` to `monitors` without topology gating or external holds.
pub(crate) fn apply_policy_with_runner_monitors<R: ProcessRunner + Sync>(
    monitors: &[MonitorConfig],
    default_settings: ApplySettings,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    fade_engine: Option<&mut FadeEngine>,
    runner: &R,
    now_epoch_s: u64,
    settings_override: Option<ApplySettings>,
) -> ApplySummary {
    let inputs = ApplyInputs {
        monitors,
        policy,
        runner,
        now_epoch_s,
        default_settings,
        settings_override,
        reconcile_actions: &[],
        recovery_until_epoch_s: None,
    };
    run_apply(
        &inputs,
        ApplySinks {
            state,
            fade_engine,
            external_holds: None,
        },
    )
}

#[cfg(test)]
pub(crate) fn apply_policy_with_runner_monitors_external<R: ProcessRunner + Sync>(
    monitors: &[MonitorConfig],
    default_settings: ApplySettings,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    fade_engine: Option<&mut FadeEngine>,
    runner: &R,
    now_epoch_s: u64,
    settings_override: Option<ApplySettings>,
    external_holds: &mut BTreeMap<String, ExternalHold>,
    recovery_until_epoch_s: Option<u64>,
) -> ApplySummary {
    let inputs = ApplyInputs {
        monitors,
        policy,
        runner,
        now_epoch_s,
        default_settings,
        settings_override,
        reconcile_actions: &[],
        recovery_until_epoch_s,
    };
    run_apply(
        &inputs,
        ApplySinks {
            state,
            fade_engine,
            external_holds: Some(external_holds),
        },
    )
}

/// Read-only inputs of one apply run. Everything is borrowed; nothing is
/// cloned.
struct ApplyInputs<'a, R> {
    monitors: &'a [MonitorConfig],
    policy: &'a PolicyOutput,
    runner: &'a R,
    now_epoch_s: u64,
    /// Settings derived from config; replaced wholesale by `settings_override`.
    default_settings: ApplySettings,
    /// Manual (IPC) settings. `Some` also bypasses failure backoff.
    settings_override: Option<ApplySettings>,
    /// Topology verdicts indexed like `monitors`. Empty disables topology
    /// gating; a monitor without a verdict is not gated.
    reconcile_actions: &'a [ReconcileAction],
    /// While `now_epoch_s` is before this deadline, drift is corrected instead
    /// of being treated as an external change.
    recovery_until_epoch_s: Option<u64>,
}

/// The mutable runtime collaborators one apply run updates.
struct ApplySinks<'a> {
    state: &'a mut RuntimeState,
    fade_engine: Option<&'a mut FadeEngine>,
    /// `None` disables external-change detection entirely.
    external_holds: Option<&'a mut BTreeMap<String, ExternalHold>>,
}

/// Runs the resolve → plan → write → commit pipeline described in the module
/// docs.
fn run_apply<R: ProcessRunner + Sync>(
    inputs: &ApplyInputs<'_, R>,
    mut sinks: ApplySinks<'_>,
) -> ApplySummary {
    let resolved = plan::ResolvedSettings::resolve(inputs, &mut sinks);
    let work = plan::plan_targets(inputs, &resolved, &mut sinks);
    let outcomes = write::dispatch_all(inputs.runner, &resolved.settings, work);
    summary::commit(outcomes, sinks.state, inputs.now_epoch_s)
}
