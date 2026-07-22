use crate::backends::{BackendKind, FailureKind};
use crate::state::types::{
    EffectiveControlState, FailureBackoffState, ManualOverrideState, MonitorRuntimeState,
    RuntimeState, STATE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const BACKOFF_BASE_SECONDS: u64 = 5;
pub(crate) const BACKOFF_MAX_SECONDS: u64 = 300;

impl RuntimeState {
    #[must_use]
    pub fn monitor(&self, logical_id: &str) -> Option<&MonitorRuntimeState> {
        self.monitors.get(logical_id)
    }

    pub fn monitor_mut(&mut self, logical_id: &str) -> &mut MonitorRuntimeState {
        self.monitors.entry(logical_id.to_owned()).or_default()
    }

    #[must_use]
    pub fn is_suspended(&self, now_epoch_s: u64) -> bool {
        self.suspend_indefinite
            || self
                .suspend_until_epoch_s
                .is_some_and(|until| until > now_epoch_s)
    }

    #[must_use]
    pub fn manual_override_active(&self, now_epoch_s: u64) -> bool {
        self.effective_control(now_epoch_s).manual_override_active
    }

    #[must_use]
    pub fn effective_control(&self, now_epoch_s: u64) -> EffectiveControlState {
        let manual_override = self.manual_override.as_ref();
        let global_override_percent =
            manual_override.and_then(|manual_override| manual_override.global_percent(now_epoch_s));
        let global_override_until_epoch_s = manual_override
            .filter(|manual_override| manual_override.global_active(now_epoch_s))
            .and_then(|manual_override| manual_override.global_expires_at_epoch_s);
        let per_monitor_override_until_epoch_s = manual_override
            .filter(|manual_override| manual_override.targets_active(now_epoch_s))
            .and_then(|manual_override| manual_override.expires_at_epoch_s);
        let per_monitor_overrides: BTreeMap<String, u8> = manual_override
            .filter(|manual_override| manual_override.targets_active(now_epoch_s))
            .map(|manual_override| {
                manual_override
                    .targets
                    .iter()
                    .map(|(logical_id, percent)| (logical_id.clone(), (*percent).min(100)))
                    .collect()
            })
            .unwrap_or_default();

        EffectiveControlState {
            suspended: self.is_suspended(now_epoch_s),
            suspend_indefinite: self.suspend_indefinite,
            suspend_until_epoch_s: self
                .suspend_until_epoch_s
                .filter(|until| *until > now_epoch_s),
            desktop_idle_dimmed: self.desktop_idle_dimmed,
            manual_override_active: global_override_percent.is_some()
                || !per_monitor_overrides.is_empty(),
            per_monitor_override_until_epoch_s,
            global_override_percent,
            global_override_until_epoch_s,
            per_monitor_overrides,
        }
    }

    pub fn refresh_effective_control<'a, I>(
        &mut self,
        now_epoch_s: u64,
        automation_targets: I,
    ) -> EffectiveControlState
    where
        I: IntoIterator<Item = (&'a str, u8)>,
    {
        self.expire_transient_flags(now_epoch_s);
        self.clear_auto_matched_monitor_overrides(now_epoch_s, automation_targets);
        self.effective_control(now_epoch_s)
    }

    pub fn prune_to_configured_monitors<'a, I>(&mut self, logical_ids: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        let valid_ids = logical_ids
            .into_iter()
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>();
        let mut changed = false;

        self.monitors.retain(|logical_id, _| {
            let keep = valid_ids.contains(logical_id);
            if !keep {
                changed = true;
            }
            keep
        });

        if let Some(manual_override) = self.manual_override.as_mut() {
            let original_len = manual_override.targets.len();
            manual_override
                .targets
                .retain(|logical_id, _| valid_ids.contains(logical_id));
            if manual_override.targets.len() != original_len {
                changed = true;
            }
            if manual_override.targets.is_empty() {
                changed |= manual_override.expires_at_epoch_s.take().is_some();
            }
            if manual_override.is_empty() {
                self.manual_override = None;
                changed = true;
            }
        }

        changed
    }

    pub fn set_monitor_override(
        &mut self,
        logical_id: &str,
        percent: u8,
        expires_at_epoch_s: Option<u64>,
    ) {
        let manual_override = self
            .manual_override
            .get_or_insert_with(ManualOverrideState::default);
        manual_override
            .targets
            .insert(logical_id.trim().to_owned(), percent.min(100));
        manual_override.expires_at_epoch_s = expires_at_epoch_s;
    }

    pub fn set_global_override(&mut self, percent: u8, expires_at_epoch_s: Option<u64>) {
        let manual_override = self
            .manual_override
            .get_or_insert_with(ManualOverrideState::default);
        manual_override.global_percent = Some(percent.min(100));
        manual_override.global_expires_at_epoch_s = expires_at_epoch_s;
    }

    pub fn clear_override(&mut self) -> bool {
        if self.manual_override.is_some() {
            self.manual_override = None;
            true
        } else {
            false
        }
    }

    pub fn clear_global_override(&mut self) -> bool {
        let Some(manual_override) = self.manual_override.as_mut() else {
            return false;
        };

        let removed_percent = manual_override.global_percent.take().is_some();
        let removed_expiry = manual_override.global_expires_at_epoch_s.take().is_some();
        let changed = removed_percent || removed_expiry;
        if manual_override.is_empty() {
            self.manual_override = None;
        }
        changed
    }

    pub fn clear_monitor_override(&mut self, logical_id: &str) -> bool {
        let Some(manual_override) = self.manual_override.as_mut() else {
            return false;
        };

        let changed = manual_override.targets.remove(logical_id).is_some();
        if manual_override.targets.is_empty() {
            manual_override.expires_at_epoch_s = None;
        }
        if manual_override.is_empty() {
            self.manual_override = None;
        }
        changed
    }

    pub fn suspend_for_minutes(&mut self, now_epoch_s: u64, minutes: u64) -> u64 {
        let until_epoch_s = now_epoch_s.saturating_add(minutes.saturating_mul(60));
        self.suspend_indefinite = false;
        self.suspend_until_epoch_s = Some(until_epoch_s);
        until_epoch_s
    }

    pub fn suspend_until_resume(&mut self) {
        self.suspend_indefinite = true;
        self.suspend_until_epoch_s = None;
    }

    pub fn clear_suspend(&mut self) {
        self.suspend_indefinite = false;
        self.suspend_until_epoch_s = None;
    }

    #[must_use]
    pub fn backoff_remaining(
        &self,
        logical_id: &str,
        backend: BackendKind,
        now_epoch_s: u64,
    ) -> Option<Duration> {
        backoff_remaining(
            self.monitor(logical_id)
                .and_then(|monitor| monitor.backoff.as_ref()),
            backend,
            now_epoch_s,
        )
    }

    pub fn expire_transient_flags(&mut self, now_epoch_s: u64) -> bool {
        let mut changed = false;

        if self
            .suspend_until_epoch_s
            .is_some_and(|until| until <= now_epoch_s)
        {
            self.suspend_until_epoch_s = None;
            changed = true;
        }

        if let Some(manual_override) = self.manual_override.as_mut() {
            changed |= manual_override.clear_expired(now_epoch_s);
            if manual_override.is_empty() {
                self.manual_override = None;
                changed = true;
            }
        }

        changed
    }

    pub fn clear_auto_matched_monitor_overrides<'a, I>(
        &mut self,
        now_epoch_s: u64,
        automation_targets: I,
    ) -> bool
    where
        I: IntoIterator<Item = (&'a str, u8)>,
    {
        let Some(manual_override) = self.manual_override.as_mut() else {
            return false;
        };
        if !manual_override.targets_active(now_epoch_s) {
            return false;
        }

        let mut changed = false;
        for (logical_id, target_percent) in automation_targets {
            if manual_override
                .target_percent(logical_id, now_epoch_s)
                .is_some_and(|override_percent| override_percent == target_percent)
            {
                changed |= manual_override.targets.remove(logical_id).is_some();
            }
        }

        if manual_override.targets.is_empty() {
            changed |= manual_override.expires_at_epoch_s.take().is_some();
        }
        if manual_override.is_empty() {
            self.manual_override = None;
            changed = true;
        }

        changed
    }

    pub fn record_apply_success(
        &mut self,
        logical_id: &str,
        applied_percent: u8,
        now_epoch_s: u64,
    ) {
        let monitor = self.monitor_mut(logical_id);
        monitor.last_applied_percent = Some(applied_percent.min(100));
        monitor.last_applied_at_epoch_s = Some(now_epoch_s);
        monitor.backoff = None;
    }

    pub fn record_apply_failure(
        &mut self,
        logical_id: &str,
        backend: BackendKind,
        failure_kind: FailureKind,
        now_epoch_s: u64,
    ) -> FailureBackoffState {
        let monitor = self.monitor_mut(logical_id);
        let next =
            next_failure_backoff(monitor.backoff.as_ref(), backend, failure_kind, now_epoch_s);
        monitor.backoff = Some(next.clone());
        next
    }

    pub(crate) fn normalized(mut self) -> Self {
        self.schema_version = STATE_SCHEMA_VERSION;
        if self.suspend_indefinite {
            self.suspend_until_epoch_s = None;
        }
        self.desktop_idle_dimmed = false;

        self.monitors.retain(|logical_id, monitor| {
            if logical_id.trim().is_empty() {
                return false;
            }

            monitor.last_applied_percent =
                monitor.last_applied_percent.map(|percent| percent.min(100));
            normalize_monitor_backoff(monitor);
            monitor.last_applied_percent.is_some()
                || monitor.last_applied_at_epoch_s.is_some()
                || monitor.backoff.is_some()
        });

        if let Some(manual_override) = self.manual_override.as_mut() {
            manual_override.global_percent = manual_override
                .global_percent
                .map(|percent| percent.min(100));
            if manual_override.global_percent.is_none() {
                manual_override.global_expires_at_epoch_s = None;
            }
            manual_override
                .targets
                .retain(|logical_id, _| !logical_id.trim().is_empty());
            for percent in manual_override.targets.values_mut() {
                *percent = (*percent).min(100);
            }
            if manual_override.targets.is_empty() {
                manual_override.expires_at_epoch_s = None;
            }
            if manual_override.is_empty() {
                self.manual_override = None;
            }
        }

        if let Some(weather) = self.weather.as_mut() {
            weather.provider = weather.provider.trim().to_owned();
            weather.cloud_cover_percent = weather.cloud_cover_percent.map(|value| value.min(100));
            weather.smoothed_cloud_cover_percent = weather
                .smoothed_cloud_cover_percent
                .map(|value| value.min(100));
            if weather.provider.is_empty() && weather.observed_at_epoch_s == 0 {
                self.weather = None;
            }
        }

        self
    }
}

impl ManualOverrideState {
    #[must_use]
    pub fn is_active(&self, now_epoch_s: u64) -> bool {
        self.global_percent(now_epoch_s).is_some() || self.targets_active(now_epoch_s)
    }

    #[must_use]
    pub fn global_percent(&self, now_epoch_s: u64) -> Option<u8> {
        if self.global_active(now_epoch_s) {
            self.global_percent.map(|percent| percent.min(100))
        } else {
            None
        }
    }

    #[must_use]
    pub fn target_percent(&self, logical_id: &str, now_epoch_s: u64) -> Option<u8> {
        if self.targets_active(now_epoch_s) {
            self.targets
                .get(logical_id)
                .copied()
                .map(|percent| percent.min(100))
        } else {
            None
        }
    }

    #[must_use]
    pub fn targets_active(&self, now_epoch_s: u64) -> bool {
        !self.targets.is_empty()
            && self
                .expires_at_epoch_s
                .is_none_or(|until| until > now_epoch_s)
    }

    #[must_use]
    pub fn global_active(&self, now_epoch_s: u64) -> bool {
        self.global_percent.is_some()
            && self
                .global_expires_at_epoch_s
                .is_none_or(|until| until > now_epoch_s)
    }

    pub fn clear_expired(&mut self, now_epoch_s: u64) -> bool {
        let mut changed = false;

        if self
            .global_expires_at_epoch_s
            .is_some_and(|until| until <= now_epoch_s)
        {
            self.global_percent = None;
            self.global_expires_at_epoch_s = None;
            changed = true;
        }

        if self
            .expires_at_epoch_s
            .is_some_and(|until| until <= now_epoch_s)
        {
            self.targets.clear();
            self.expires_at_epoch_s = None;
            changed = true;
        }

        if self.targets.is_empty() && self.expires_at_epoch_s.is_some() {
            self.expires_at_epoch_s = None;
            changed = true;
        }

        if self.global_percent.is_none() && self.global_expires_at_epoch_s.is_some() {
            self.global_expires_at_epoch_s = None;
            changed = true;
        }

        changed
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.global_percent.is_none() && self.targets.is_empty()
    }
}

impl EffectiveControlState {
    #[must_use]
    pub fn monitor_override_percent(&self, logical_id: &str) -> Option<u8> {
        self.per_monitor_overrides.get(logical_id).copied()
    }

    #[must_use]
    pub fn effective_percent_for(&self, logical_id: &str, automated_percent: u8) -> u8 {
        if self.desktop_idle_dimmed {
            return 0;
        }
        self.monitor_override_percent(logical_id)
            .or(self.global_override_percent)
            .unwrap_or(automated_percent)
    }
}

pub(crate) fn current_epoch_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn should_skip_hysteresis(
    last_applied_percent: Option<u8>,
    requested_percent: u8,
    min_write_delta_pct: u8,
) -> bool {
    if min_write_delta_pct == 0 {
        return false;
    }

    match last_applied_percent {
        Some(last_applied_percent) => {
            requested_percent
                .min(100)
                .abs_diff(last_applied_percent.min(100))
                < min_write_delta_pct
        }
        None => false,
    }
}

pub(crate) fn limit_step_size(
    last_applied_percent: Option<u8>,
    requested_percent: u8,
    max_step_pct_per_tick: u8,
) -> u8 {
    let requested_percent = requested_percent.min(100);
    let Some(last_applied_percent) = last_applied_percent.map(|value| value.min(100)) else {
        return requested_percent;
    };

    if max_step_pct_per_tick == 0 {
        return requested_percent;
    }

    if requested_percent > last_applied_percent {
        last_applied_percent
            .saturating_add(max_step_pct_per_tick)
            .min(requested_percent)
            .min(100)
    } else {
        last_applied_percent
            .saturating_sub(max_step_pct_per_tick)
            .max(requested_percent)
    }
}

pub(crate) fn write_interval_active(
    last_applied_at_epoch_s: Option<u64>,
    now_epoch_s: u64,
    min_apply_interval: Duration,
) -> bool {
    let min_apply_interval_s = min_apply_interval.as_secs();
    if min_apply_interval_s == 0 {
        return false;
    }

    match last_applied_at_epoch_s {
        Some(last_applied_at_epoch_s) => {
            now_epoch_s < last_applied_at_epoch_s.saturating_add(min_apply_interval_s)
        }
        None => false,
    }
}

pub(crate) fn backoff_remaining(
    backoff: Option<&FailureBackoffState>,
    backend: BackendKind,
    now_epoch_s: u64,
) -> Option<Duration> {
    let backoff = backoff?;
    if backoff.backend != backend {
        return None;
    }

    let suppress_until_epoch_s = backoff.suppress_until_epoch_s?;
    if suppress_until_epoch_s <= now_epoch_s {
        return None;
    }

    Some(Duration::from_secs(
        suppress_until_epoch_s.saturating_sub(now_epoch_s),
    ))
}

pub(crate) fn next_failure_backoff(
    previous: Option<&FailureBackoffState>,
    backend: BackendKind,
    failure_kind: FailureKind,
    now_epoch_s: u64,
) -> FailureBackoffState {
    let consecutive_failures = match previous {
        Some(previous) if previous.backend == backend && previous.failure_kind == failure_kind => {
            previous.consecutive_failures.saturating_add(1)
        }
        _ => 1,
    };
    let delay = backoff_delay(consecutive_failures);

    FailureBackoffState {
        backend,
        failure_kind,
        consecutive_failures,
        suppress_until_epoch_s: Some(now_epoch_s.saturating_add(delay.as_secs())),
    }
}

pub(crate) fn backoff_delay(consecutive_failures: u32) -> Duration {
    let shift = consecutive_failures.saturating_sub(1).min(16);
    let multiplier = 1u64 << shift;
    Duration::from_secs(
        BACKOFF_BASE_SECONDS
            .saturating_mul(multiplier)
            .min(BACKOFF_MAX_SECONDS),
    )
}

pub(crate) fn normalize_monitor_backoff(monitor: &mut MonitorRuntimeState) {
    let Some(backoff) = monitor.backoff.as_mut() else {
        return;
    };

    if backoff.consecutive_failures == 0 {
        if backoff.suppress_until_epoch_s.is_some() {
            backoff.consecutive_failures = 1;
        } else {
            monitor.backoff = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn step_limiting_moves_toward_requested_target_without_overshoot() {
        assert_eq!(limit_step_size(Some(40), 70, 6), 46);
        assert_eq!(limit_step_size(Some(40), 10, 6), 34);
        assert_eq!(limit_step_size(Some(40), 43, 6), 43);
        assert_eq!(limit_step_size(None, 88, 6), 88);
    }

    #[test]
    fn hysteresis_skips_only_small_changes() {
        assert!(should_skip_hysteresis(Some(50), 51, 2));
        assert!(!should_skip_hysteresis(Some(50), 52, 2));
        assert!(!should_skip_hysteresis(None, 10, 2));
    }

    #[test]
    fn transient_override_expiry_clears_inactive_parts() {
        let mut state = RuntimeState {
            manual_override: Some(ManualOverrideState {
                global_percent: Some(55),
                global_expires_at_epoch_s: Some(90),
                targets: BTreeMap::from([(String::from("desk"), 40)]),
                expires_at_epoch_s: Some(110),
            }),
            ..RuntimeState::default()
        };

        assert!(state.expire_transient_flags(100));
        let manual_override = state
            .manual_override
            .as_ref()
            .expect("per-monitor override should still be active");
        assert_eq!(manual_override.global_percent(100), None);
        assert_eq!(manual_override.target_percent("desk", 100), Some(40));

        assert!(state.expire_transient_flags(120));
        assert_eq!(state.manual_override, None);
    }

    #[test]
    fn effective_control_hides_expired_suspend_and_overrides() {
        let state = RuntimeState {
            suspend_until_epoch_s: Some(90),
            manual_override: Some(ManualOverrideState {
                global_percent: Some(55),
                global_expires_at_epoch_s: Some(90),
                targets: BTreeMap::from([(String::from("desk"), 40)]),
                expires_at_epoch_s: Some(90),
            }),
            ..RuntimeState::default()
        };

        let control = state.effective_control(100);

        assert!(!control.suspended);
        assert_eq!(control.suspend_until_epoch_s, None);
        assert!(!control.manual_override_active);
        assert_eq!(control.global_override_percent, None);
        assert_eq!(control.global_override_until_epoch_s, None);
        assert_eq!(control.per_monitor_override_until_epoch_s, None);
        assert_eq!(control.monitor_override_percent("desk"), None);
        assert_eq!(control.effective_percent_for("desk", 27), 27);
    }

    #[test]
    fn effective_control_keeps_indefinite_suspend_active() {
        let state = RuntimeState {
            suspend_indefinite: true,
            ..RuntimeState::default()
        };

        let control = state.effective_control(100);

        assert!(control.suspended);
        assert!(control.suspend_indefinite);
        assert_eq!(control.suspend_until_epoch_s, None);
    }

    #[test]
    fn refresh_effective_control_clears_monitor_overrides_that_match_automation() {
        let mut state = RuntimeState {
            manual_override: Some(ManualOverrideState {
                global_percent: Some(55),
                global_expires_at_epoch_s: Some(150),
                targets: BTreeMap::from([
                    (String::from("desk"), 40),
                    (String::from("internal"), 25),
                ]),
                expires_at_epoch_s: Some(150),
            }),
            ..RuntimeState::default()
        };

        let control = state.refresh_effective_control(100, [("desk", 40), ("internal", 20)]);

        assert!(control.manual_override_active);
        assert_eq!(control.global_override_percent, Some(55));
        assert_eq!(control.monitor_override_percent("desk"), None);
        assert_eq!(control.monitor_override_percent("internal"), Some(25));
        assert_eq!(control.effective_percent_for("desk", 40), 55);
        assert_eq!(control.effective_percent_for("internal", 20), 25);
        assert_eq!(
            state
                .manual_override
                .as_ref()
                .and_then(|manual_override| manual_override.target_percent("desk", 100)),
            None
        );
        assert_eq!(
            state
                .manual_override
                .as_ref()
                .and_then(|manual_override| manual_override.target_percent("internal", 100)),
            Some(25)
        );
    }

    #[test]
    fn prune_to_configured_monitors_removes_stale_monitor_state() {
        let mut state = RuntimeState::default();
        state
            .monitors
            .insert(String::from("desk"), MonitorRuntimeState::default());
        state.monitors.insert(
            String::from("internal"),
            MonitorRuntimeState {
                last_applied_percent: Some(30),
                last_applied_at_epoch_s: Some(100),
                backoff: None,
            },
        );
        state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: BTreeMap::from([(String::from("desk"), 60), (String::from("internal"), 40)]),
            expires_at_epoch_s: Some(200),
        });

        assert!(state.prune_to_configured_monitors(["internal"].iter().copied()));
        assert!(state.monitor("desk").is_none());
        assert!(state.monitor("internal").is_some());
        assert_eq!(
            state
                .manual_override
                .as_ref()
                .and_then(|manual_override| manual_override.target_percent("desk", 150)),
            None
        );
        assert_eq!(
            state
                .manual_override
                .as_ref()
                .and_then(|manual_override| manual_override.target_percent("internal", 150)),
            Some(40)
        );
    }
}
