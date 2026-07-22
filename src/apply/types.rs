use crate::backends::{BackendKind, FailureKind};
use crate::config::Config;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplySettings {
    pub min_write_delta_pct: u8,
    pub max_step_pct_per_tick: u8,
    /// When false, apply the requested target in a single backend dispatch
    /// instead of scheduling rapid intermediate writes.
    pub smooth_transition: bool,
    pub min_apply_interval: Duration,
    pub dry_run: bool,
    pub apply_reassert_interval: Duration,
    pub ddc_timeout: Duration,
    pub backlight_timeout: Duration,
}

impl ApplySettings {
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            min_write_delta_pct: config.solar_policy.min_write_delta_pct.min(100),
            max_step_pct_per_tick: config.solar_policy.max_step_pct_per_tick.min(100),
            smooth_transition: config.daemon.smooth_transition,
            min_apply_interval: Duration::from_secs(config.daemon.tick_seconds),
            dry_run: config.daemon.dry_run,
            apply_reassert_interval: Duration::from_secs(config.daemon.apply_reassert_minutes * 60),
            ddc_timeout: Duration::from_secs(config.daemon.ddc_timeout_seconds),
            backlight_timeout: Duration::from_secs(config.daemon.backlight_timeout_seconds),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyStatus {
    SkippedDisabled,
    SkippedDryRun,
    SkippedHysteresis,
    SkippedMinimumInterval,
    SkippedBackoff,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyRecord {
    pub logical_id: String,
    pub backend: Option<BackendKind>,
    pub requested_percent: u8,
    pub applied_percent: u8,
    pub attempts: u8,
    pub failure_kind: Option<FailureKind>,
    pub consecutive_failures: Option<u32>,
    pub backoff_until_epoch_s: Option<u64>,
    pub status: ApplyStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplySummary {
    pub attempted: usize,
    pub skipped: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub backoff_skips: usize,
    pub transient_failures: usize,
    pub persistent_failures: usize,
    pub records: Vec<ApplyRecord>,
}

impl ApplySummary {
    pub fn push(&mut self, record: ApplyRecord) {
        match record.status {
            ApplyStatus::Succeeded => {
                self.attempted += 1;
                self.succeeded += 1;
            }
            ApplyStatus::Failed => {
                if record.attempts > 0 {
                    self.attempted += 1;
                }
                self.failed += 1;
                match record.failure_kind {
                    Some(FailureKind::Transient) => self.transient_failures += 1,
                    Some(FailureKind::Persistent) => self.persistent_failures += 1,
                    None => {}
                }
            }
            ApplyStatus::SkippedDisabled
            | ApplyStatus::SkippedDryRun
            | ApplyStatus::SkippedHysteresis
            | ApplyStatus::SkippedMinimumInterval => {
                self.skipped += 1;
            }
            ApplyStatus::SkippedBackoff => {
                self.skipped += 1;
                self.backoff_skips += 1;
            }
        }

        self.records.push(record);
    }
}
