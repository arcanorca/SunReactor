//! Probe schedule: when to read each monitor's brightness back.
//!
//! Brightness can be changed behind the daemon's back. Desktop compositors
//! (KWin, for example) keep their own saved value and write it when a display
//! wakes; users press monitor buttons; a monitor that has just powered on may
//! also come back at its own stored level.
//!
//! Wake signals (DRM hotplug, logind resume, idle resume) are not available on
//! every distribution or desktop, and the ones that exist can be silent for a
//! whole session. So the schedule never depends on them: it reads the monitors
//! on a steady cadence and treats the hardware itself as the source of truth.
//! A monitor that stops answering is asleep; when it answers again that *is*
//! the wake, and probes speed up to catch whatever the desktop writes
//! afterwards. Signals, when they arrive, only make the same window open
//! sooner.
//!
//! Reads are cheap: a verified-bus DDC read is about 40 ms, and a sleeping
//! monitor fails in a few milliseconds.

use std::time::{Duration, Instant};

/// How long fast probes continue after a wake, a signal, or a correction.
pub const FAST_WINDOW: Duration = Duration::from_mins(1);
/// Time between probes while the fast window is open, and while waiting for a
/// sleeping monitor to answer again.
pub const FAST_INTERVAL: Duration = Duration::from_secs(2);
/// Short pause before the first probe after a signal, so a monitor that is
/// still powering on gets a moment.
pub const FIRST_PROBE_DELAY: Duration = Duration::from_millis(500);

/// Why probes are running fast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeReason {
    WaylandIdleResume,
    SystemResume,
    DrmHotplug,
    DrmConnectorChange,
    /// A monitor that was not answering started answering again. This is the
    /// signal-free wake detection and works on every desktop.
    MonitorAnswered,
    /// Something else changed a monitor's brightness and it was corrected.
    BrightnessChanged,
}

/// What the last probe observed, as far as scheduling cares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeOutcome {
    /// Monitors that answered a read.
    pub answered: usize,
    /// Monitors that were expected to answer but did not: asleep or still
    /// waking.
    pub unreachable: usize,
    /// Monitors whose brightness had to be rewritten.
    pub corrected: usize,
}

#[derive(Debug, Clone)]
pub struct ProbeSchedule {
    /// Cadence when every monitor answers and matches its target. Zero turns
    /// steady probing off, leaving signal-driven windows only.
    steady: Option<Duration>,
    next: Option<Instant>,
    fast_until: Option<Instant>,
    /// A monitor did not answer the last probe, so the next answer is a wake.
    waiting_for_wake: bool,
    reason: Option<WakeReason>,
    probes: u64,
}

impl ProbeSchedule {
    #[must_use]
    pub fn new(steady: Duration, now: Instant) -> Self {
        let steady = (!steady.is_zero()).then_some(steady);
        Self {
            steady,
            next: steady.map(|interval| now + interval),
            fast_until: None,
            waiting_for_wake: false,
            reason: None,
            probes: 0,
        }
    }

    /// A wake signal: probe shortly and keep the fast cadence for a while.
    /// Returns `true` when this opened a new fast window.
    pub fn signal(&mut self, reason: WakeReason, now: Instant) -> bool {
        let opened = !self.is_fast(now);
        self.fast_until = Some(now + FAST_WINDOW);
        self.reason = Some(reason);
        let due = now + FIRST_PROBE_DELAY;
        if self.next.is_none_or(|next| due < next) {
            self.next = Some(due);
        }
        opened
    }

    #[must_use]
    pub fn is_fast(&self, now: Instant) -> bool {
        self.fast_until.is_some_and(|until| now < until)
    }

    /// Whether a probe is due now. The next one is scheduled from the current
    /// cadence and refined once the outcome is known.
    pub fn take_due_probe(&mut self, now: Instant) -> bool {
        match self.next {
            Some(due) if now >= due => {
                self.probes = self.probes.saturating_add(1);
                self.next = self.schedule_from(now);
                true
            }
            _ => false,
        }
    }

    /// Feeds the probe result back into the cadence.
    pub fn note_outcome(&mut self, outcome: ProbeOutcome, now: Instant) {
        if outcome.unreachable > 0 {
            // Asleep, or still waking: keep a light pulse so the moment it
            // answers is caught within one fast interval.
            self.waiting_for_wake = true;
        } else if self.waiting_for_wake && outcome.answered > 0 {
            // It answered again: that is the wake, whatever the desktop is.
            self.waiting_for_wake = false;
            self.signal(WakeReason::MonitorAnswered, now);
        }
        if outcome.corrected > 0 {
            // Someone else is writing to the monitor; stay fast in case they
            // write again.
            self.signal(WakeReason::BrightnessChanged, now);
        }
        self.next = self.schedule_from(now);
    }

    fn schedule_from(&self, now: Instant) -> Option<Instant> {
        if self.is_fast(now) || self.waiting_for_wake {
            return Some(now + FAST_INTERVAL);
        }
        self.steady.map(|interval| now + interval)
    }

    /// When the loop should next wake up for a probe.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.next
    }

    #[must_use]
    pub fn reason(&self) -> Option<WakeReason> {
        self.reason
    }

    #[must_use]
    pub fn probes(&self) -> u64 {
        self.probes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEADY: Duration = Duration::from_secs(15);

    fn schedule(start: Instant) -> ProbeSchedule {
        ProbeSchedule::new(STEADY, start)
    }

    /// Runs the probe that is due at `now`, if any, and feeds back `outcome`.
    fn probe(schedule: &mut ProbeSchedule, now: Instant, outcome: ProbeOutcome) -> bool {
        let due = schedule.take_due_probe(now);
        if due {
            schedule.note_outcome(outcome, now);
        }
        due
    }

    fn answered(count: usize) -> ProbeOutcome {
        ProbeOutcome {
            answered: count,
            ..ProbeOutcome::default()
        }
    }

    #[test]
    fn steady_cadence_runs_without_any_wake_signal() {
        let start = Instant::now();
        let mut schedule = schedule(start);
        assert!(!probe(&mut schedule, start, answered(2)));
        assert!(probe(&mut schedule, start + STEADY, answered(2)));
        assert!(!probe(
            &mut schedule,
            start + STEADY + FAST_INTERVAL,
            answered(2)
        ));
        assert!(probe(&mut schedule, start + STEADY * 2, answered(2)));
        assert_eq!(schedule.probes(), 2);
    }

    #[test]
    fn a_sleeping_monitor_is_polled_often_and_answering_again_counts_as_a_wake() {
        let start = Instant::now();
        let mut schedule = schedule(start);
        let asleep = ProbeOutcome {
            answered: 1,
            unreachable: 1,
            corrected: 0,
        };
        probe(&mut schedule, start + STEADY, asleep);
        // Waiting for the monitor: short pulses, not the steady cadence.
        assert_eq!(
            schedule.next_deadline(),
            Some(start + STEADY + FAST_INTERVAL)
        );

        let wake = start + STEADY + FAST_INTERVAL;
        assert!(probe(&mut schedule, wake, answered(2)));
        assert_eq!(schedule.reason(), Some(WakeReason::MonitorAnswered));
        // The wake opens a fast window so a late desktop write is caught.
        assert!(schedule.is_fast(wake + Duration::from_secs(30)));
        assert_eq!(schedule.next_deadline(), Some(wake + FAST_INTERVAL));
    }

    #[test]
    fn a_correction_keeps_probes_fast_then_they_settle_again() {
        let start = Instant::now();
        let mut schedule = schedule(start);
        let corrected = ProbeOutcome {
            answered: 2,
            unreachable: 0,
            corrected: 1,
        };
        probe(&mut schedule, start + STEADY, corrected);
        assert_eq!(schedule.reason(), Some(WakeReason::BrightnessChanged));
        assert_eq!(
            schedule.next_deadline(),
            Some(start + STEADY + FAST_INTERVAL)
        );

        // Once the window closes, the steady cadence returns.
        let settled = start + STEADY + FAST_WINDOW;
        probe(&mut schedule, settled, answered(2));
        assert!(!schedule.is_fast(settled));
        assert_eq!(schedule.next_deadline(), Some(settled + STEADY));
    }

    #[test]
    fn a_signal_probes_sooner_without_disturbing_the_cadence_afterwards() {
        let start = Instant::now();
        let mut schedule = schedule(start);
        assert!(schedule.signal(WakeReason::DrmHotplug, start));
        assert_eq!(schedule.next_deadline(), Some(start + FIRST_PROBE_DELAY));
        assert!(!schedule.signal(WakeReason::SystemResume, start));

        let first = start + FIRST_PROBE_DELAY;
        assert!(probe(&mut schedule, first, answered(2)));
        assert_eq!(schedule.next_deadline(), Some(first + FAST_INTERVAL));
    }

    #[test]
    fn steady_probing_can_be_turned_off_and_signals_still_work() {
        let start = Instant::now();
        let mut schedule = ProbeSchedule::new(Duration::ZERO, start);
        assert_eq!(schedule.next_deadline(), None);
        assert!(!probe(
            &mut schedule,
            start + Duration::from_mins(5),
            answered(2)
        ));

        schedule.signal(WakeReason::SystemResume, start);
        assert!(probe(&mut schedule, start + FIRST_PROBE_DELAY, answered(2)));
        // The window keeps it fast, and it stops once the window closes.
        let after = start + FIRST_PROBE_DELAY + FAST_WINDOW;
        probe(&mut schedule, after, answered(2));
        assert_eq!(schedule.next_deadline(), None);
    }
}
