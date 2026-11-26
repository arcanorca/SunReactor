use crate::backends::{BackendKind, FailureKind};
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// An in-memory quarantine for a monitor whose brightness was changed by
/// another controller. This is deliberately not part of persisted runtime
/// state: restarting the daemon starts a fresh ownership observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalHold {
    pub since_epoch_s: u64,
    pub until_epoch_s: u64,
    pub observed_percent: u8,
}

pub const EXTERNAL_CHANGE_THRESHOLD_PCT: u8 = 2;
pub const OWN_WRITE_GRACE_SECONDS: u64 = 5;
pub const RECOVERY_WINDOW_SECONDS: u64 = 120;

/// Write economy. Every DDC `setvcp` can wear the panel's EEPROM and ties up
/// the I2C bus, so every non-user write path is budgeted per monitor.
///
/// Minimum gap between two probe corrections of one monitor. One probe cadence
/// (15 s) plus slack: a correction that did not take is retried at most every
/// other probe, never back to back.
pub const MIN_CORRECTION_INTERVAL_SECONDS: u64 = 20;
/// Rolling window for [`MAX_WRITES_PER_WINDOW`].
pub const WRITE_BUDGET_WINDOW_SECONDS: u64 = 600;
/// Hard ceiling on budgeted write events (a fade counts as one) per monitor
/// per [`WRITE_BUDGET_WINDOW_SECONDS`], i.e. 72 per hour. Normal operation
/// needs at most one per 60 s tick (10 per window); the ceiling only bites in
/// a misbehaving loop. Over budget the write is deferred, not dropped: the
/// next tick or probe recomputes the latest target.
pub const MAX_WRITES_PER_WINDOW: usize = 12;
/// A monitor whose brightness cannot be read back is reasserted blind at most
/// this often (instead of every `apply_reassert_minutes`).
pub const BLIND_REASSERT_INTERVAL_SECONDS: u64 = 900;
/// The same (target, readback) mismatch seen again within this window means
/// the monitor snaps to its own step grid: the readback is accepted instead of
/// rewritten forever.
pub const QUANTIZATION_LEARN_WINDOW_SECONDS: u64 = 600;

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
    pub external_change_hold: Duration,
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
            apply_reassert_interval: Duration::from_secs(
                config.daemon.apply_reassert_minutes.saturating_mul(60),
            ),
            external_change_hold: Duration::from_secs(
                u64::from(config.daemon.external_change_hold_minutes).saturating_mul(60),
            ),
            ddc_timeout: Duration::from_secs(config.daemon.ddc_timeout_seconds),
            backlight_timeout: Duration::from_secs(config.daemon.backlight_timeout_seconds),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyStatus {
    SkippedDisabled,
    SkippedDryRun,
    SkippedHysteresis,
    SkippedMinimumInterval,
    SkippedBudget,
    SkippedManualCap,
    SkippedContested,
    SkippedBackoff,
    SkippedTopology,
    Succeeded,
    Failed,
    #[serde(other)]
    Unknown,
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
            | ApplyStatus::SkippedMinimumInterval
            | ApplyStatus::SkippedBudget
            | ApplyStatus::SkippedManualCap
            | ApplyStatus::SkippedContested
            | ApplyStatus::SkippedTopology
            | ApplyStatus::Unknown => {
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

/// Largest gap between target and readback that can be a monitor rounding to
/// its own step grid. Bigger gaps (0, 50, 100 after a standby) are resets or
/// ignored writes and are never "learned".
pub const QUANTIZATION_LEARN_MAX_DELTA_PCT: u8 = 3;

/// Consecutive failed reads (see [`DISCONNECT_MIN_SPAN_SECONDS`]) that count as
/// evidence of a real disconnection. A bare transient error never does.
pub const DISCONNECT_MIN_FAILED_READS: u32 = 3;
/// The failed reads must span at least this long.
pub const DISCONNECT_MIN_SPAN_SECONDS: u64 = 30;

/// A deviation found this soon after our own write looks like another
/// controller undoing it.
pub const CONTEST_QUICK_REVERT_SECONDS: u64 = 180;
/// This many such events within [`CONTEST_WINDOW_SECONDS`] make a monitor
/// "contested".
pub const CONTEST_THRESHOLD_EVENTS: usize = 3;
pub const CONTEST_WINDOW_SECONDS: u64 = 600;
/// How long automatic correction stays paused for a contested monitor.
pub const CONTESTED_BACKOFF_SECONDS: u64 = 6 * 3600;

/// User-forced writes (IPC overrides, limits, idle dim/wake) are not part of
/// the automatic budget but have their own generous cap per monitor: one write
/// per [`MANUAL_MIN_GAP_SECONDS`] and [`MANUAL_MAX_WRITES_PER_WINDOW`] per
/// [`MANUAL_WINDOW_SECONDS`]. Over the cap the write is deferred and the
/// latest value is applied later, never dropped.
pub const MANUAL_MIN_GAP_SECONDS: u64 = 1;
pub const MANUAL_MAX_WRITES_PER_WINDOW: usize = 60;
pub const MANUAL_WINDOW_SECONDS: u64 = 600;
