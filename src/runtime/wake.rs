use std::time::{Duration, Instant};

const STABILIZATION_DELAY: Duration = Duration::from_secs(2);
const RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeReassertReason {
    WaylandIdleResume,
    KdeDpmsResume,
    SystemResume,
    ManualWake,
    TopologyRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeReassertAction {
    Observe { attempt: u8 },
    Wait,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Stabilizing,
    Observing,
    Retry,
    Completed,
}

#[derive(Debug, Clone)]
pub struct WakeReassertCoordinator {
    phase: Phase,
    due_at: Option<Instant>,
    attempts: u8,
    last_reason: Option<WakeReassertReason>,
}

impl Default for WakeReassertCoordinator {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            due_at: None,
            attempts: 0,
            last_reason: None,
        }
    }
}

impl WakeReassertCoordinator {
    #[must_use]
    pub fn request(&mut self, reason: WakeReassertReason, now: Instant) -> bool {
        self.last_reason = Some(reason);
        if !matches!(self.phase, Phase::Idle | Phase::Completed) {
            return false;
        }
        self.phase = Phase::Stabilizing;
        self.attempts = 0;
        self.due_at = Some(now + STABILIZATION_DELAY);
        true
    }

    #[must_use]
    pub fn poll(&mut self, now: Instant) -> WakeReassertAction {
        if !matches!(self.phase, Phase::Stabilizing | Phase::Retry) {
            return WakeReassertAction::Done;
        }
        if self.due_at.is_some_and(|due| now < due) {
            return WakeReassertAction::Wait;
        }
        self.attempts = self.attempts.saturating_add(1);
        self.phase = Phase::Observing;
        WakeReassertAction::Observe {
            attempt: self.attempts,
        }
    }

    #[must_use]
    pub fn observation_completed(&mut self, present: bool, now: Instant) -> bool {
        if !matches!(self.phase, Phase::Observing) {
            return false;
        }
        if present {
            self.finish();
            true
        } else {
            self.observation_unavailable(now)
        }
    }

    #[must_use]
    pub fn observation_unavailable(&mut self, now: Instant) -> bool {
        if self.attempts >= 2 {
            self.finish();
            return false;
        }
        self.phase = Phase::Retry;
        self.due_at = Some(now + RETRY_DELAY);
        true
    }

    pub fn finish(&mut self) {
        self.phase = Phase::Completed;
        self.due_at = None;
    }

    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(
            self.phase,
            Phase::Stabilizing | Phase::Observing | Phase::Retry
        )
    }

    #[must_use]
    pub fn attempts(&self) -> u8 {
        self.attempts
    }

    #[must_use]
    pub fn last_reason(&self) -> Option<WakeReassertReason> {
        self.last_reason
    }
}

#[cfg(test)]
mod tests {
    use super::{WakeReassertAction, WakeReassertCoordinator, WakeReassertReason};
    use std::time::{Duration, Instant};

    #[test]
    fn hint_waits_then_allows_one_observation() {
        let start = Instant::now();
        let mut coordinator = WakeReassertCoordinator::default();
        assert!(coordinator.request(WakeReassertReason::ManualWake, start));
        assert_eq!(coordinator.poll(start), WakeReassertAction::Wait);
        assert_eq!(
            coordinator.poll(start + Duration::from_secs(2)),
            WakeReassertAction::Observe { attempt: 1 }
        );
        assert_eq!(
            coordinator.poll(start + Duration::from_secs(2)),
            WakeReassertAction::Done
        );
        assert!(coordinator.observation_completed(true, start));
        assert_eq!(
            coordinator.poll(start + Duration::from_secs(4)),
            WakeReassertAction::Done
        );
    }

    #[test]
    fn duplicate_hints_are_coalesced_until_episode_finishes() {
        let start = Instant::now();
        let mut coordinator = WakeReassertCoordinator::default();
        assert!(coordinator.request(WakeReassertReason::KdeDpmsResume, start));
        assert!(!coordinator.request(WakeReassertReason::WaylandIdleResume, start));
        coordinator.finish();
        assert!(coordinator.request(WakeReassertReason::SystemResume, start));
    }

    #[test]
    fn unavailable_observation_has_one_bounded_retry_then_stops() {
        let start = Instant::now();
        let mut coordinator = WakeReassertCoordinator::default();
        assert!(coordinator.request(WakeReassertReason::TopologyRecovery, start));
        assert_eq!(
            coordinator.poll(start + Duration::from_secs(2)),
            WakeReassertAction::Observe { attempt: 1 }
        );
        assert!(coordinator.observation_unavailable(start));
        assert_eq!(
            coordinator.poll(start + Duration::from_secs(4)),
            WakeReassertAction::Observe { attempt: 2 }
        );
        assert!(!coordinator.observation_completed(false, start));
        assert!(!coordinator.is_pending());
    }
}
