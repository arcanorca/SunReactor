//! In-memory, per-monitor evidence the apply pipeline needs but the persisted
//! runtime state does not carry:
//!
//! * read-failure streaks, so one transient DDC error is never mistaken for a
//!   disconnection;
//! * connector power transitions (DPMS / status), so a value the monitor came
//!   back with after standby is never blamed on the user;
//! * corrections that another DDC controller (KDE PowerDevil, ...) undid
//!   shortly after we wrote them, which put a monitor into a hands-off
//!   "contested" back-off;
//! * the rate cap of user-forced writes.
//!
//! Nothing here is persisted: a daemon restart starts a fresh observation.
//! Production code shares one registry; unit tests get a registry per thread
//! so parallel tests cannot see each other.

use super::types::{
    CONTESTED_BACKOFF_SECONDS, CONTEST_QUICK_REVERT_SECONDS, CONTEST_THRESHOLD_EVENTS,
    CONTEST_WINDOW_SECONDS, DISCONNECT_MIN_FAILED_READS, DISCONNECT_MIN_SPAN_SECONDS,
    EXTERNAL_CHANGE_THRESHOLD_PCT, MANUAL_MAX_WRITES_PER_WINDOW, MANUAL_MIN_GAP_SECONDS,
    MANUAL_WINDOW_SECONDS, OWN_WRITE_GRACE_SECONDS, RECOVERY_WINDOW_SECONDS,
};
use crate::config::MonitorConfig;
use crate::state::CLOCK_SKEW_TOLERANCE_S;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
struct MonitorHealth {
    failed_reads: u32,
    first_failure_epoch_s: Option<u64>,
    /// The connector was seen powered off, or came back on, and no successful
    /// readback has judged the monitor since.
    power_transition_pending: bool,
    last_seen_powered_off: bool,
    /// End of the latest recovery window seen for this monitor.
    recovery_until_epoch_s: Option<u64>,
    contest_events: Vec<u64>,
    /// `last_applied_at` of the write the latest contest event was about.
    last_contest_write_at: Option<u64>,
    contested_until_epoch_s: Option<u64>,
    manual_writes: Vec<u64>,
    manual_deferred: bool,
}

type Registry = BTreeMap<String, MonitorHealth>;

#[cfg(not(test))]
static REGISTRY: std::sync::Mutex<Registry> = std::sync::Mutex::new(BTreeMap::new());

#[cfg(test)]
thread_local! {
    static REGISTRY: std::cell::RefCell<Registry> = const { std::cell::RefCell::new(BTreeMap::new()) };
    static POWERED_OFF_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

fn with_registry<T>(f: impl FnOnce(&mut Registry) -> T) -> T {
    #[cfg(not(test))]
    {
        f(&mut REGISTRY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    }
    #[cfg(test)]
    {
        REGISTRY.with(|registry| f(&mut registry.borrow_mut()))
    }
}

fn with_monitor<T>(logical_id: &str, f: impl FnOnce(&mut MonitorHealth) -> T) -> T {
    with_registry(|registry| f(registry.entry(logical_id.to_owned()).or_default()))
}

/// Test hook: forces what the connector power probe reports on this thread.
#[cfg(test)]
pub(crate) fn set_powered_off_for_test(value: Option<bool>) {
    POWERED_OFF_OVERRIDE.with(|cell| cell.set(value));
}

/// Test hook: forgets everything on this thread.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    with_registry(BTreeMap::clear);
    set_powered_off_for_test(None);
}

/// Whether the kernel reports the monitor's connector disconnected (legacy
/// `dpms` is ignored, see `backends::ddc::connector_is_powered_off`).
/// Reads sysfs only, never DDC.
fn connector_powered_off(monitor: &MonitorConfig) -> bool {
    #[cfg(test)]
    if let Some(value) = POWERED_OFF_OVERRIDE.with(std::cell::Cell::get) {
        return value;
    }
    #[cfg(target_os = "linux")]
    {
        crate::backends::ddc::connector_powered_off(&monitor.selector)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = monitor;
        false
    }
}

// ---------------------------------------------------------------- reads ----

/// Records a failed brightness read and reports whether it is now evidence of
/// a real disconnection: the connector is powered off or disconnected, or at
/// least [`DISCONNECT_MIN_FAILED_READS`] consecutive reads failed over
/// [`DISCONNECT_MIN_SPAN_SECONDS`]. A bare transient error is never evidence.
pub(crate) fn record_read_failure(monitor: &MonitorConfig, now_epoch_s: u64) -> bool {
    let off = connector_powered_off(monitor);
    with_monitor(&monitor.logical_id, |health| {
        if off {
            health.last_seen_powered_off = true;
            health.power_transition_pending = true;
        }
        health.failed_reads = health.failed_reads.saturating_add(1);
        let first = *health.first_failure_epoch_s.get_or_insert(now_epoch_s);
        off || (health.failed_reads >= DISCONNECT_MIN_FAILED_READS
            && now_epoch_s.saturating_sub(first) >= DISCONNECT_MIN_SPAN_SECONDS)
    })
}

pub(crate) fn record_read_success(logical_id: &str) {
    with_monitor(logical_id, |health| {
        health.failed_reads = 0;
        health.first_failure_epoch_s = None;
    });
}

/// A genuine reconnect is also a power event: whatever the monitor shows now
/// may be a factory value.
pub(crate) fn mark_power_event(logical_id: &str) {
    with_monitor(logical_id, |health| health.power_transition_pending = true);
}

// ------------------------------------------------------- power evidence ----

/// What hardware evidence says about a readback, gathered once per check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HardwareEvidence {
    /// A connector power transition happened since the last judged readback.
    pub(crate) power_transition: bool,
    /// Within [`RECOVERY_WINDOW_SECONDS`] after a recovery window: a reset
    /// looking value (0, 50, 100) is still a reset, not a choice.
    pub(crate) reset_grace: bool,
}

/// Observes the connector (sysfs) and returns the evidence for this check.
pub(crate) fn observe_hardware(
    monitor: &MonitorConfig,
    recovery_until_epoch_s: Option<u64>,
    now_epoch_s: u64,
) -> HardwareEvidence {
    let off = connector_powered_off(monitor);
    with_monitor(&monitor.logical_id, |health| {
        if off || health.last_seen_powered_off != off {
            health.power_transition_pending = true;
        }
        health.last_seen_powered_off = off;
        if let Some(until) = recovery_until_epoch_s {
            health.recovery_until_epoch_s = Some(
                health
                    .recovery_until_epoch_s
                    .map_or(until, |c| c.max(until)),
            );
        }
        HardwareEvidence {
            power_transition: health.power_transition_pending,
            reset_grace: health
                .recovery_until_epoch_s
                .is_some_and(|until| now_epoch_s < until.saturating_add(RECOVERY_WINDOW_SECONDS)),
        }
    })
}

/// A successful readback has been judged: the pending power transition is
/// consumed, unless the connector is still off (the transition is not over).
pub(crate) fn acknowledge_readback(logical_id: &str) {
    with_monitor(logical_id, |health| {
        if !health.last_seen_powered_off {
            health.power_transition_pending = false;
        }
    });
}

// ------------------------------------------------------------ contested ----

/// A monitor in the hands-off back-off, as the runtime/status reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestedMonitor {
    pub logical_id: String,
    pub until_epoch_s: u64,
}

/// Whether another DDC controller is fighting us over `logical_id` and
/// automatic corrections are paused. Expires on its own.
#[must_use]
pub fn is_contested(logical_id: &str, now_epoch_s: u64) -> bool {
    contested_until(logical_id, now_epoch_s).is_some()
}

/// End of the back-off while `logical_id` is contested.
#[must_use]
pub fn contested_until(logical_id: &str, now_epoch_s: u64) -> Option<u64> {
    with_registry(|registry| {
        registry
            .get(logical_id)
            .and_then(|health| health.contested_until_epoch_s)
            .filter(|until| now_epoch_s < *until)
    })
}

/// Every monitor currently in the contested back-off.
#[must_use]
pub fn contested_monitors(now_epoch_s: u64) -> Vec<ContestedMonitor> {
    with_registry(|registry| {
        registry
            .iter()
            .filter_map(|(id, health)| {
                health
                    .contested_until_epoch_s
                    .filter(|until| now_epoch_s < *until)
                    .map(|until_epoch_s| ContestedMonitor {
                        logical_id: id.clone(),
                        until_epoch_s,
                    })
            })
            .collect()
    })
}

/// Ends the back-off early (user resumed automation, config reload).
pub fn clear_contested(logical_id: &str) {
    with_registry(|registry| {
        if let Some(health) = registry.get_mut(logical_id) {
            health.contested_until_epoch_s = None;
            health.contest_events.clear();
        }
    });
}

/// Notes that the monitor deviates from the value we last wrote it. Only a
/// deviation that appears shortly after our own write counts: that is the
/// signature of another controller undoing it. Returns true when this event
/// makes the monitor contested.
pub(crate) fn note_contest_event(
    logical_id: &str,
    now_epoch_s: u64,
    last_applied_percent: Option<u8>,
    last_applied_at_epoch_s: Option<u64>,
    observed: u8,
) -> bool {
    let (Some(written), Some(written_at)) = (last_applied_percent, last_applied_at_epoch_s) else {
        return false;
    };
    let after_grace = now_epoch_s >= written_at.saturating_add(OWN_WRITE_GRACE_SECONDS);
    let quick = now_epoch_s < written_at.saturating_add(CONTEST_QUICK_REVERT_SECONDS);
    if !after_grace || !quick || observed.abs_diff(written) <= EXTERNAL_CHANGE_THRESHOLD_PCT {
        return false;
    }
    let newly = with_monitor(logical_id, |health| {
        if health.last_contest_write_at == Some(written_at) {
            return false; // the same write, seen again by a later probe
        }
        health.last_contest_write_at = Some(written_at);
        health
            .contest_events
            .retain(|at| now_epoch_s < at.saturating_add(CONTEST_WINDOW_SECONDS));
        health.contest_events.push(now_epoch_s);
        if health.contest_events.len() < CONTEST_THRESHOLD_EVENTS
            || health
                .contested_until_epoch_s
                .is_some_and(|until| now_epoch_s < until)
        {
            return false;
        }
        health.contest_events.clear();
        health.contested_until_epoch_s =
            Some(now_epoch_s.saturating_add(CONTESTED_BACKOFF_SECONDS));
        true
    });
    if newly {
        tracing::warn!(
            logical_id = %logical_id,
            backoff_minutes = CONTESTED_BACKOFF_SECONDS / 60,
            "monitor_contested; another program, likely KDE PowerDevil, is changing this \
             monitor's brightness right after SunReactor sets it. Automatic correction is \
             paused. Disable its external-monitor brightness control (System Settings > \
             Power Management, or stop the other DDC tool) so only one controller remains"
        );
    }
    newly
}

// --------------------------------------------------------- forced writes ----

/// Checks the user-forced write cap. `Err(retry_at)` means the write must be
/// deferred (the latest value is applied at or after `retry_at`).
pub(crate) fn manual_slot(logical_id: &str, now_epoch_s: u64) -> Result<(), u64> {
    with_monitor(logical_id, |health| {
        health
            .manual_writes
            .retain(|at| now_epoch_s < at.saturating_add(MANUAL_WINDOW_SECONDS));
        let gap_end = health
            .manual_writes
            .last()
            .map(|last| last.saturating_add(MANUAL_MIN_GAP_SECONDS))
            .filter(|end| now_epoch_s < *end);
        let window_end = (health.manual_writes.len() >= MANUAL_MAX_WRITES_PER_WINDOW)
            .then(|| health.manual_writes[0].saturating_add(MANUAL_WINDOW_SECONDS));
        match gap_end.max(window_end) {
            Some(retry_at) => {
                health.manual_deferred = true;
                Err(retry_at)
            }
            None => Ok(()),
        }
    })
}

/// Counts one user-forced write (or fade start).
pub(crate) fn record_manual_write(logical_id: &str, now_epoch_s: u64) {
    with_monitor(logical_id, |health| {
        health.manual_writes.push(now_epoch_s);
        health.manual_deferred = false;
    });
}

/// Earliest epoch at which a deferred user-forced write for `logical_id` may
/// be retried. `None` when no manual write is currently deferred for it.
#[must_use]
pub fn deferred_manual_until(logical_id: &str, now_epoch_s: u64) -> Option<u64> {
    with_registry(|registry| {
        registry
            .get(logical_id)
            .filter(|health| health.manual_deferred)
            .and_then(|health| {
                let gap = health
                    .manual_writes
                    .last()
                    .map(|last| last.saturating_add(MANUAL_MIN_GAP_SECONDS));
                let window = (health.manual_writes.len() >= MANUAL_MAX_WRITES_PER_WINDOW)
                    .then(|| health.manual_writes[0].saturating_add(MANUAL_WINDOW_SECONDS));
                gap.max(window)
            })
            .map(|at| at.max(now_epoch_s))
    })
}

/// Earliest epoch at which a deferred user-forced write may be retried, for
/// the runtime to schedule one more forced apply. `None` when nothing is
/// deferred. The latest value is always recomputed from state at that time.
#[must_use]
pub fn next_deferred_manual_at(now_epoch_s: u64) -> Option<u64> {
    with_registry(|registry| {
        registry
            .values()
            .filter(|health| health.manual_deferred)
            .filter_map(|health| {
                let gap = health
                    .manual_writes
                    .last()
                    .map(|last| last.saturating_add(MANUAL_MIN_GAP_SECONDS));
                let window = (health.manual_writes.len() >= MANUAL_MAX_WRITES_PER_WINDOW)
                    .then(|| health.manual_writes[0].saturating_add(MANUAL_WINDOW_SECONDS));
                gap.max(window)
            })
            .min()
            .map(|at| at.max(now_epoch_s))
    })
}

// ------------------------------------------------------------ clock jumps ----

/// Shifts every in-memory window timestamp in the health registry by a
/// wall-clock jump (`delta_s` > 0: the clock stepped forward).
pub fn shift_windows(delta_s: i64) {
    let shift = |value: &mut u64| {
        *value = if delta_s >= 0 {
            value.saturating_add(delta_s.unsigned_abs())
        } else {
            value.saturating_sub(delta_s.unsigned_abs())
        };
    };
    with_registry(|registry| {
        for health in registry.values_mut() {
            health.first_failure_epoch_s.iter_mut().for_each(shift);
            health.recovery_until_epoch_s.iter_mut().for_each(shift);
            health.contest_events.iter_mut().for_each(shift);
            health.last_contest_write_at.iter_mut().for_each(shift);
            health.contested_until_epoch_s.iter_mut().for_each(shift);
            health.manual_writes.iter_mut().for_each(shift);
        }
    });
}

/// Clamps any health-registry timestamp lying implausibly far in the future
/// (beyond [`CLOCK_SKEW_TOLERANCE_S`]) so a backward clock jump cannot freeze
/// contest detection, recovery, manual-cap tracking, or disconnect detection.
/// Returns the number of clamped timestamps.
pub fn clamp_future_windows(now_epoch_s: u64) -> usize {
    let limit = now_epoch_s.saturating_add(CLOCK_SKEW_TOLERANCE_S);
    let max_recovery = limit.saturating_add(RECOVERY_WINDOW_SECONDS);
    let max_contested = limit.saturating_add(CONTESTED_BACKOFF_SECONDS);
    let mut clamped = 0;
    let clamp_past = |value: &mut u64| -> usize {
        if *value > limit {
            *value = now_epoch_s;
            1
        } else {
            0
        }
    };
    with_registry(|registry| {
        for health in registry.values_mut() {
            if let Some(at) = health.first_failure_epoch_s.as_mut() {
                clamped += clamp_past(at);
            }
            if let Some(until) = health.recovery_until_epoch_s.as_mut() {
                if *until > max_recovery {
                    *until = now_epoch_s.saturating_add(RECOVERY_WINDOW_SECONDS);
                    clamped += 1;
                }
            }
            for at in &mut health.contest_events {
                clamped += clamp_past(at);
            }
            if let Some(at) = health.last_contest_write_at.as_mut() {
                clamped += clamp_past(at);
            }
            if let Some(until) = health.contested_until_epoch_s.as_mut() {
                if *until > max_contested {
                    *until = now_epoch_s.saturating_add(CONTESTED_BACKOFF_SECONDS);
                    clamped += 1;
                }
            }
            for at in &mut health.manual_writes {
                clamped += clamp_past(at);
            }
        }
    });
    clamped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_windows_preserves_durations_across_three_hour_backward_and_forward_jumps() {
        reset_for_test();
        let base = 100_000_u64;
        let monitor = MonitorConfig {
            logical_id: String::from("panel"),
            ..MonitorConfig::default()
        };
        let _ = record_read_failure(&monitor, base);
        let _ = observe_hardware(&monitor, Some(base + RECOVERY_WINDOW_SECONDS), base);
        note_contest_event("panel", base + 10, Some(50), Some(base), 80);
        note_contest_event("panel", base + 110, Some(50), Some(base + 100), 80);
        note_contest_event("panel", base + 210, Some(50), Some(base + 200), 80);
        record_manual_write("panel", base + 250);
        with_monitor("panel", |h| {
            h.contest_events = vec![base + 10, base + 20];
        });

        let three_hours = 3 * 3_600_i64;
        shift_windows(-three_hours);
        with_monitor("panel", |h| {
            assert_eq!(h.first_failure_epoch_s, Some(base - 10_800));
            assert_eq!(
                h.recovery_until_epoch_s,
                Some(base + RECOVERY_WINDOW_SECONDS - 10_800)
            );
            assert_eq!(
                h.contest_events,
                vec![base + 10 - 10_800, base + 20 - 10_800]
            );
            assert_eq!(h.last_contest_write_at, Some(base + 200 - 10_800));
            assert_eq!(
                h.contested_until_epoch_s,
                Some(base + 210 + CONTESTED_BACKOFF_SECONDS - 10_800)
            );
            assert_eq!(h.manual_writes, vec![base + 250 - 10_800]);
        });

        shift_windows(three_hours);
        with_monitor("panel", |h| {
            assert_eq!(h.first_failure_epoch_s, Some(base));
            assert_eq!(
                h.recovery_until_epoch_s,
                Some(base + RECOVERY_WINDOW_SECONDS)
            );
            assert_eq!(h.contest_events, vec![base + 10, base + 20]);
            assert_eq!(h.last_contest_write_at, Some(base + 200));
            assert_eq!(
                h.contested_until_epoch_s,
                Some(base + 210 + CONTESTED_BACKOFF_SECONDS)
            );
            assert_eq!(h.manual_writes, vec![base + 250]);
        });
        reset_for_test();
    }

    #[test]
    fn clamp_future_windows_clamps_three_hour_future_timestamps_and_keeps_plausible_ones() {
        reset_for_test();
        let now = 100_000_u64;
        let future = now + 3 * 3_600;
        with_monitor("bad", |h| {
            h.first_failure_epoch_s = Some(future);
            h.recovery_until_epoch_s = Some(future + RECOVERY_WINDOW_SECONDS);
            h.contest_events = vec![future, future + 10];
            h.last_contest_write_at = Some(future);
            h.contested_until_epoch_s = Some(future + CONTESTED_BACKOFF_SECONDS);
            h.manual_writes = vec![future];
        });
        with_monitor("ok", |h| {
            h.first_failure_epoch_s = Some(now - 5);
            h.recovery_until_epoch_s = Some(now + 60);
            h.contest_events = vec![now - 10];
            h.last_contest_write_at = Some(now - 20);
            h.contested_until_epoch_s = Some(now + 3_600);
            h.manual_writes = vec![now - 1];
        });

        assert_eq!(clamp_future_windows(now), 7);
        with_monitor("bad", |h| {
            assert_eq!(h.first_failure_epoch_s, Some(now));
            assert_eq!(
                h.recovery_until_epoch_s,
                Some(now + RECOVERY_WINDOW_SECONDS)
            );
            assert_eq!(h.contest_events, vec![now, now]);
            assert_eq!(h.last_contest_write_at, Some(now));
            assert_eq!(
                h.contested_until_epoch_s,
                Some(now + CONTESTED_BACKOFF_SECONDS)
            );
            assert_eq!(h.manual_writes, vec![now]);
        });
        with_monitor("ok", |h| {
            assert_eq!(h.first_failure_epoch_s, Some(now - 5));
            assert_eq!(h.recovery_until_epoch_s, Some(now + 60));
            assert_eq!(h.contest_events, vec![now - 10]);
            assert_eq!(h.last_contest_write_at, Some(now - 20));
            assert_eq!(h.contested_until_epoch_s, Some(now + 3_600));
            assert_eq!(h.manual_writes, vec![now - 1]);
        });
        reset_for_test();
    }
}
