use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::backends::ProcessRunner;
use crate::runtime::topology::CapabilitySnapshot;

/// Slow safety net: a full `ddcutil detect` + capabilities pass is heavy bus
/// traffic, so without any event (hotplug, resume, reload, explicit request)
/// the topology is only re-observed this often.
pub(super) const CAPABILITY_SAFETY_REFRESH_INTERVAL: Duration = Duration::from_mins(30);
/// Minimum gap between two observation starts, so event bursts coalesce.
pub(super) const CAPABILITY_REFRESH_MIN_GAP: Duration = Duration::from_secs(10);

#[derive(Debug)]
enum CapabilityRefreshResult {
    Completed {
        generation: u64,
        snapshot: CapabilitySnapshot,
    },
    Failed,
}

#[derive(Debug)]
pub(super) enum CapabilityRefreshPublication {
    Published(CapabilitySnapshot),
    Discarded,
    Failed,
}

/// Bounded asynchronous capability observation for the daemon control loop.
#[derive(Debug)]
pub(super) struct CapabilityRefreshState {
    tx: mpsc::SyncSender<CapabilityRefreshResult>,
    rx: mpsc::Receiver<CapabilityRefreshResult>,
    active: Arc<AtomicBool>,
    pending: bool,
    handle: Option<std::thread::JoinHandle<()>>,
    generation: Arc<AtomicU64>,
    /// An event asked for a fresh observation (hotplug, resume, reload, ...).
    requested: bool,
    last_started: Option<Instant>,
}

impl Default for CapabilityRefreshState {
    fn default() -> Self {
        let (tx, rx) = mpsc::sync_channel(1);
        Self {
            tx,
            rx,
            active: Arc::new(AtomicBool::new(false)),
            pending: false,
            handle: None,
            generation: Arc::new(AtomicU64::new(0)),
            requested: false,
            last_started: None,
        }
    }
}

impl CapabilityRefreshState {
    pub(super) fn begin_with_runner<R>(&mut self, runner: R)
    where
        R: ProcessRunner + Send + Sync + 'static,
    {
        if self.active.load(Ordering::Acquire) {
            return;
        }
        let generation = self
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);

        if let Some(handle) = self.handle.take() {
            if !handle.is_finished() {
                self.handle = Some(handle);
                return;
            }
            let _ = handle.join();
        }

        if self
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let sender = self.tx.clone();
        let active = Arc::clone(&self.active);
        self.handle = Some(std::thread::spawn(move || {
            let _active_guard = CapabilityRefreshGuard(active);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let report = crate::discovery::discover_with_runner(
                    &runner,
                    Path::new("/sys/class/backlight"),
                );
                CapabilityRefreshResult::Completed {
                    generation,
                    snapshot: CapabilitySnapshot::from_discovery(&report),
                }
            }))
            .unwrap_or(CapabilityRefreshResult::Failed);
            let _ = sender.try_send(result);
        }));
    }

    pub(super) fn publish_completed(&mut self) -> Option<CapabilityRefreshPublication> {
        match self.rx.try_recv() {
            Ok(CapabilityRefreshResult::Completed {
                generation,
                snapshot,
            }) => {
                if generation == self.generation.load(Ordering::Acquire) {
                    Some(CapabilityRefreshPublication::Published(snapshot))
                } else {
                    tracing::warn!(generation, "capability_refresh_superseded");
                    Some(CapabilityRefreshPublication::Discarded)
                }
            }
            Ok(CapabilityRefreshResult::Failed) => Some(CapabilityRefreshPublication::Failed),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(CapabilityRefreshPublication::Failed),
        }
    }

    /// Asks for a fresh observation; honoured by [`Self::due`].
    pub(super) fn request(&mut self) {
        self.requested = true;
    }

    /// Whether a new observation should start now: the very first one (no
    /// snapshot yet), an explicit request, or the slow safety net. Never while
    /// one is in flight, and never closer than the minimum gap.
    pub(super) fn due(&self, now: Instant, has_snapshot: bool) -> bool {
        if self.pending || self.active.load(Ordering::Acquire) {
            return false;
        }
        let Some(started) = self.last_started else {
            return true;
        };
        let since = now.saturating_duration_since(started);
        if since < CAPABILITY_REFRESH_MIN_GAP {
            return false;
        }
        !has_snapshot || self.requested || since >= CAPABILITY_SAFETY_REFRESH_INTERVAL
    }

    pub(super) fn note_started(&mut self, now: Instant) {
        self.requested = false;
        self.last_started = Some(now);
    }

    pub(super) fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    pub(super) fn set_pending(&mut self) {
        self.pending = true;
    }

    pub(super) fn take_pending(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }

    pub(super) fn is_pending(&self) -> bool {
        self.pending
    }
}

impl Drop for CapabilityRefreshState {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct CapabilityRefreshGuard(Arc<AtomicBool>);

impl Drop for CapabilityRefreshGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_observation_is_due_then_only_on_request_or_safety_net() {
        let mut state = CapabilityRefreshState::default();
        let t0 = Instant::now();
        assert!(state.due(t0, false), "startup observation is due");
        state.note_started(t0);

        // Ordinary ticks must not trigger detect again.
        for minutes in [1_u64, 2, 5, 14] {
            assert!(!state.due(t0 + Duration::from_mins(minutes), true));
        }
        // An event does, but not inside the coalescing gap.
        state.request();
        assert!(!state.due(t0 + Duration::from_secs(2), true));
        assert!(state.due(t0 + Duration::from_secs(11), true));
        state.note_started(t0 + Duration::from_secs(11));
        assert!(!state.due(t0 + Duration::from_mins(1), true));
        // Safety net.
        assert!(state.due(
            t0 + Duration::from_secs(11) + CAPABILITY_SAFETY_REFRESH_INTERVAL,
            true
        ));
    }

    #[test]
    fn safety_net_is_at_least_fifteen_minutes() {
        assert!(CAPABILITY_SAFETY_REFRESH_INTERVAL >= Duration::from_mins(15));
    }

    #[test]
    fn nothing_is_due_while_pending() {
        let mut state = CapabilityRefreshState::default();
        state.set_pending();
        state.request();
        assert!(!state.due(Instant::now(), false));
    }
}
