use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use crate::apply::{self, ApplySummary};
use crate::backends::ProcessRunner;
use crate::policy::{self, PolicyContext, PolicyOutput};
use crate::runtime::orchestrator::{
    skipped_apply_summary, DaemonRuntime, RuntimeError, TickReport,
};
use crate::solar::{self, Location, SolarSample};

#[derive(Debug, Clone)]
pub(super) struct CollectedTickInputs {
    pub(super) now_utc: DateTime<Utc>,
    pub(super) now_epoch_s: u64,
    location: Location,
    solar: SolarSample,
    weather_multiplier: Option<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct ComputedTick {
    pub(super) inputs: CollectedTickInputs,
    pub(super) policy: PolicyOutput,
    pub(super) suspended: bool,
    manual_override_active: bool,
}

#[derive(Debug, Clone)]
struct AppliedTick {
    computed: ComputedTick,
    apply_summary: ApplySummary,
}

impl DaemonRuntime {
    pub(crate) fn run_once_at_with_runner<R: ProcessRunner + Sync>(
        &mut self,
        now_utc: DateTime<Utc>,
        runner: &R,
        force_immediate: bool,
    ) -> Result<TickReport, RuntimeError> {
        let tick_started = Instant::now();
        let inputs = self.collect_tick_inputs(now_utc, force_immediate)?;
        let computed = self.compute_tick_policy(inputs)?;
        let applied = self.apply_tick_policy(computed, runner, force_immediate);

        self.finish_tick(applied, tick_started)
    }

    pub(super) fn collect_tick_inputs(
        &mut self,
        now_utc: DateTime<Utc>,
        force_weather_refresh: bool,
    ) -> Result<CollectedTickInputs, RuntimeError> {
        let now_epoch_s = now_utc.timestamp().max(0) as u64;
        let location = self.config.location.clone();
        let solar = solar::sample_at_utc(
            now_utc,
            &location,
            self.config.solar.twilight_elevation_start,
            self.config.solar.day_elevation_full,
        )?;
        let weather_multiplier =
            self.refresh_weather_modifier(&location, now_epoch_s, force_weather_refresh);

        Ok(CollectedTickInputs {
            now_utc,
            now_epoch_s,
            location,
            solar,
            weather_multiplier,
        })
    }

    pub(super) fn compute_tick_policy(
        &mut self,
        inputs: CollectedTickInputs,
    ) -> Result<ComputedTick, RuntimeError> {
        let mut policy = policy::compute_policy(&PolicyContext {
            now_utc: inputs.now_utc,
            location: &inputs.location,
            config: &self.config.solar,
            weather_multiplier: inputs.weather_multiplier,
            monitors: &self.config.monitors,
        })?;

        let control = self.state.refresh_effective_control(
            inputs.now_epoch_s,
            policy
                .targets
                .iter()
                .map(|target| (target.logical_id.as_str(), target.percent)),
        );
        for target in &mut policy.targets {
            target.percent = control.effective_percent_for(&target.logical_id, target.percent);
        }

        Ok(ComputedTick {
            inputs,
            policy,
            suspended: control.suspended,
            manual_override_active: control.manual_override_active,
        })
    }

    fn apply_tick_policy<R: ProcessRunner + Sync>(
        &mut self,
        computed: ComputedTick,
        runner: &R,
        force_immediate: bool,
    ) -> AppliedTick {
        let apply_summary = if computed.suspended {
            skipped_apply_summary(computed.policy.targets.len(), "suspend_until is active")
        } else {
            let settings_override = force_immediate.then(|| self.force_apply_settings());
            let monitors = self.config.monitors.as_slice();
            let settings = self.config.apply;
            if let Some(capabilities) = self.last_capabilities.as_ref() {
                apply::apply_policy_with_runner_reconciled_mode(
                    monitors,
                    settings,
                    &computed.policy,
                    &mut self.state,
                    runner,
                    computed.inputs.now_epoch_s,
                    capabilities,
                    Some(&mut self.fade_engine),
                    settings_override,
                )
            } else {
                skipped_apply_summary(
                    computed.policy.targets.len(),
                    "capability observation is not ready",
                )
            }
        };

        AppliedTick {
            computed,
            apply_summary,
        }
    }

    fn finish_tick(
        &mut self,
        applied: AppliedTick,
        tick_started: Instant,
    ) -> Result<TickReport, RuntimeError> {
        self.persist_state_if_changed()?;

        let solar_elevation = f64::from(applied.computed.inputs.solar.elevation_deg);
        self.last_solar_elevation = Some(solar_elevation);
        let weather_modifier_applied = applied.computed.policy.weather_multiplier < 1.0;

        Ok(TickReport {
            now_utc: applied.computed.inputs.now_utc,
            solar: applied.computed.inputs.solar,
            policy: applied.computed.policy,
            apply_summary: applied.apply_summary,
            tick_duration: tick_started.elapsed(),
            monitors_evaluated: self.config.monitors.len(),
            suspended: applied.computed.suspended,
            manual_override_active: applied.computed.manual_override_active,
            weather_modifier_applied,
        })
    }

    pub(super) fn force_apply_settings(&self) -> apply::ApplySettings {
        apply::ApplySettings {
            min_write_delta_pct: 0,
            max_step_pct_per_tick: 100,
            min_apply_interval: Duration::ZERO,
            dry_run: self.config.daemon.dry_run,
            apply_reassert_interval: Duration::from_secs(
                self.config.daemon.apply_reassert_minutes * 60,
            ),
            ddc_timeout: Duration::from_secs(self.config.daemon.ddc_timeout_seconds),
            backlight_timeout: Duration::from_secs(self.config.daemon.backlight_timeout_seconds),
        }
    }
}
