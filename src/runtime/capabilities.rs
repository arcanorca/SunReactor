use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};

use crate::backends::ProcessRunner;
use crate::runtime::topology::CapabilitySnapshot;

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
