//! Wall-clock step detection and normalization of in-memory windows.
//!
//! Holds, write-budget events, backoff and minimum intervals are stored as
//! epoch seconds. A wall clock that steps (NTP, manual change, resume) would
//! freeze or prematurely end those windows. The daemon compares wall-clock
//! progress against a suspend-aware monotonic clock (`CLOCK_BOOTTIME`) and,
//! when they disagree by more than [`CLOCK_JUMP_THRESHOLD_MS`], shifts every
//! in-memory window by the jump so each keeps its real remaining length.
//! Timestamps still in the future beyond the skew tolerance afterwards are
//! clamped to "now" (see `RuntimeState::clamp_future_timestamps`).

use std::collections::BTreeMap;

use crate::apply::ExternalHold;
use crate::state::CLOCK_SKEW_TOLERANCE_S;

/// Wall-vs-monotonic disagreement that counts as a clock step.
pub(super) const CLOCK_JUMP_THRESHOLD_MS: i64 = 60_000;

/// Milliseconds on a monotonic clock that keeps counting through suspend.
#[cfg(target_os = "linux")]
pub(super) fn boottime_ms() -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, exclusively borrowed timespec.
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &raw mut ts) };
    if rc != 0 {
        return fallback_monotonic_ms();
    }
    ts.tv_sec * 1000 + ts.tv_nsec / 1_000_000
}

#[cfg(not(target_os = "linux"))]
pub(super) fn boottime_ms() -> i64 {
    fallback_monotonic_ms()
}

fn fallback_monotonic_ms() -> i64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static BASE: OnceLock<Instant> = OnceLock::new();
    BASE.get_or_init(Instant::now).elapsed().as_millis() as i64
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ClockWatch {
    last_wall_ms: i64,
    last_mono_ms: i64,
}

impl ClockWatch {
    pub(super) fn new(wall_ms: i64, mono_ms: i64) -> Self {
        Self {
            last_wall_ms: wall_ms,
            last_mono_ms: mono_ms,
        }
    }

    /// Returns the wall-clock jump in whole seconds (positive: the clock
    /// stepped forward) when it exceeds [`CLOCK_JUMP_THRESHOLD_MS`].
    pub(super) fn observe(&mut self, wall_ms: i64, mono_ms: i64) -> Option<i64> {
        let wall_delta = wall_ms - self.last_wall_ms;
        let mono_delta = mono_ms - self.last_mono_ms;
        self.last_wall_ms = wall_ms;
        self.last_mono_ms = mono_ms;
        let jump = wall_delta - mono_delta;
        (jump.abs() > CLOCK_JUMP_THRESHOLD_MS).then_some(jump / 1000)
    }
}

/// Shifts holds by a wall-clock jump.
pub(super) fn shift_holds(holds: &mut BTreeMap<String, ExternalHold>, delta_s: i64) {
    let shift = |value: u64| {
        if delta_s >= 0 {
            value.saturating_add(delta_s.unsigned_abs())
        } else {
            value.saturating_sub(delta_s.unsigned_abs())
        }
    };
    for hold in holds.values_mut() {
        hold.since_epoch_s = shift(hold.since_epoch_s);
        hold.until_epoch_s = shift(hold.until_epoch_s);
    }
}

/// Clamps implausible holds: a start in the future counts as now, and an end
/// further away than the configured hold (plus skew) is cut to `now + hold`.
/// Returns how many holds were changed.
pub(super) fn clamp_holds(
    holds: &mut BTreeMap<String, ExternalHold>,
    now_epoch_s: u64,
    max_hold_s: u64,
) -> usize {
    let limit = now_epoch_s.saturating_add(CLOCK_SKEW_TOLERANCE_S);
    let max_until = limit.saturating_add(max_hold_s);
    let mut changed = 0;
    for hold in holds.values_mut() {
        let mut touched = false;
        if hold.since_epoch_s > limit {
            hold.since_epoch_s = now_epoch_s;
            touched = true;
        }
        if hold.until_epoch_s > max_until {
            hold.until_epoch_s = now_epoch_s.saturating_add(max_hold_s);
            touched = true;
        }
        changed += usize::from(touched);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold(since: u64, until: u64) -> ExternalHold {
        ExternalHold {
            since_epoch_s: since,
            until_epoch_s: until,
            observed_percent: 40,
        }
    }

    #[test]
    fn steady_clock_reports_no_jump() {
        let mut watch = ClockWatch::new(1_000_000, 5_000);
        assert_eq!(watch.observe(1_016_000, 21_000), None);
        assert_eq!(watch.observe(1_076_000, 81_000), None);
    }

    #[test]
    fn backward_step_of_three_hours_is_detected() {
        let mut watch = ClockWatch::new(100_000_000, 5_000);
        let jump = watch.observe(100_000_000 - 3 * 3_600_000 + 15_000, 20_000);
        assert_eq!(jump, Some(-3 * 3_600));
    }

    #[test]
    fn forward_step_is_detected() {
        let mut watch = ClockWatch::new(0, 0);
        assert_eq!(watch.observe(2 * 3_600_000 + 10_000, 10_000), Some(7_200));
    }

    #[test]
    fn shift_keeps_the_remaining_window_length() {
        let mut holds = BTreeMap::from([(String::from("a"), hold(1_000, 4_600))]);
        shift_holds(&mut holds, -10_800);
        assert_eq!(holds["a"].since_epoch_s, 0);
        assert_eq!(holds["a"].until_epoch_s, 0);
        let mut holds = BTreeMap::from([(String::from("a"), hold(20_000, 23_600))]);
        shift_holds(&mut holds, -10_800);
        assert_eq!(holds["a"].until_epoch_s - holds["a"].since_epoch_s, 3_600);
        shift_holds(&mut holds, 7_200);
        assert_eq!(holds["a"].since_epoch_s, 16_400);
    }

    #[test]
    fn clamp_cuts_future_holds_and_leaves_plausible_ones() {
        let now = 100_000;
        let mut holds = BTreeMap::from([
            (
                String::from("bad"),
                hold(now + 10_800, now + 10_800 + 3_600),
            ),
            (String::from("ok"), hold(now - 60, now + 1_800)),
        ]);
        assert_eq!(clamp_holds(&mut holds, now, 3_600), 1);
        assert_eq!(holds["bad"].since_epoch_s, now);
        assert_eq!(holds["bad"].until_epoch_s, now + 3_600);
        assert_eq!(holds["ok"], hold(now - 60, now + 1_800));
    }
}
