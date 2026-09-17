//! Wake watch: after a monitor or the session wakes up, probe brightness
//! briefly and often.
//!
//! Desktop compositors (KWin, for example) may restore their own saved
//! brightness a few seconds after a display comes back, and monitors answer
//! DDC/CI only once they have finished waking. Instead of a fixed number of
//! retries, any wake signal opens a short window in which the daemon reads
//! each monitor every couple of seconds and rewrites any wrong value. A new
//! signal during the window extends it.

use std::time::{Duration, Instant};

/// How long probes continue after the latest wake signal.
pub const WATCH_WINDOW: Duration = Duration::from_mins(1);
/// Time between probes while the window is open.
pub const PROBE_INTERVAL: Duration = Duration::from_secs(2);
/// Short pause before the first probe so a monitor can start answering.
pub const FIRST_PROBE_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeReason {
    WaylandIdleResume,
    SystemResume,
    DrmHotplug,
    DrmConnectorChange,
}

#[derive(Debug, Clone, Default)]
pub struct WakeWatch {
    until: Option<Instant>,
    next_probe: Option<Instant>,
    reason: Option<WakeReason>,
    probes: u32,
}

impl WakeWatch {
    /// Opens the watch window, or extends it when one is already open.
    /// Returns `true` when this signal opened a new window.
    pub fn start(&mut self, reason: WakeReason, now: Instant) -> bool {
        let already_open = self.is_active(now);
        self.until = Some(now + WATCH_WINDOW);
        self.reason = Some(reason);
        if !already_open {
            self.next_probe = Some(now + FIRST_PROBE_DELAY);
            self.probes = 0;
        }
        !already_open
    }

    #[must_use]
    pub fn is_active(&self, now: Instant) -> bool {
        self.until.is_some_and(|until| now < until)
    }

    /// Whether a probe should run now; schedules the following one.
    pub fn take_due_probe(&mut self, now: Instant) -> bool {
        if !self.is_active(now) {
            self.until = None;
            self.next_probe = None;
            return false;
        }
        match self.next_probe {
            Some(due) if now >= due => {
                self.next_probe = Some(now + PROBE_INTERVAL);
                self.probes = self.probes.saturating_add(1);
                true
            }
            _ => false,
        }
    }

    /// When the loop should next wake up for a probe.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.until.and(self.next_probe)
    }

    #[must_use]
    pub fn reason(&self) -> Option<WakeReason> {
        self.reason
    }

    #[must_use]
    pub fn probes(&self) -> u32 {
        self.probes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wake_signal_probes_after_a_short_delay_then_every_interval() {
        let start = Instant::now();
        let mut watch = WakeWatch::default();
        assert!(!watch.take_due_probe(start));
        assert!(watch.start(WakeReason::DrmHotplug, start));
        assert!(!watch.take_due_probe(start));
        assert_eq!(watch.next_deadline(), Some(start + FIRST_PROBE_DELAY));

        let first = start + FIRST_PROBE_DELAY;
        assert!(watch.take_due_probe(first));
        assert!(!watch.take_due_probe(first + Duration::from_millis(100)));
        assert!(watch.take_due_probe(first + PROBE_INTERVAL));
        assert_eq!(watch.probes(), 2);
    }

    #[test]
    fn a_new_signal_extends_the_window_without_resetting_the_cadence() {
        let start = Instant::now();
        let mut watch = WakeWatch::default();
        watch.start(WakeReason::DrmHotplug, start);
        let later = start + Duration::from_secs(50);
        assert!(watch.take_due_probe(later));
        assert!(!watch.start(WakeReason::WaylandIdleResume, later));
        assert_eq!(watch.reason(), Some(WakeReason::WaylandIdleResume));
        // The first window would have closed at 60 s; the extension keeps it open.
        assert!(watch.is_active(start + Duration::from_secs(100)));
        assert!(!watch.take_due_probe(later + Duration::from_secs(1)));
    }

    #[test]
    fn the_window_closes_and_stops_probing() {
        let start = Instant::now();
        let mut watch = WakeWatch::default();
        watch.start(WakeReason::SystemResume, start);
        let after = start + WATCH_WINDOW;
        assert!(!watch.take_due_probe(after));
        assert!(!watch.is_active(after));
        assert_eq!(watch.next_deadline(), None);
    }
}
