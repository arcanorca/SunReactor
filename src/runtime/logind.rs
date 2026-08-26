//! Optional systemd-logind resume hint source.
//!
//! This source is deliberately non-authoritative: it only reports the end of
//! the logind sleep lifecycle. The runtime still owns observation,
//! reconciliation, and all hardware-write authorization.

use futures_lite::{future, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use zbus::{Connection, Proxy};

const SIGNAL_POLL_TIMEOUT: Duration = Duration::from_millis(250);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const HINT_CHANNEL_CAPACITY: usize = 1;
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogindEvent {
    PrepareForSleep(bool),
    SystemResume { correlation_id: u64 },
}

static NEXT_CORRELATION_ID: AtomicU64 = AtomicU64::new(1);

#[must_use]
pub fn resume_edge(sleeping: &mut bool, start: bool) -> bool {
    if start {
        *sleeping = true;
        false
    } else if *sleeping {
        *sleeping = false;
        true
    } else {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalPoll {
    Event(LogindEvent),
    Malformed,
    Disconnected,
    Timeout,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionOutcome {
    Shutdown,
    Disconnected,
}

trait SignalAdapter {
    fn next<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = SignalPoll> + 'a>>;
}

struct ZbusSignalAdapter<'a, S> {
    stream: &'a mut S,
    shutdown: &'a AtomicBool,
}

impl<S> SignalAdapter for ZbusSignalAdapter<'_, S>
where
    S: StreamExt<Item = zbus::Message> + Unpin,
{
    fn next<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = SignalPoll> + 'a>> {
        Box::pin(async move {
            if self.shutdown.load(Ordering::Relaxed) {
                return SignalPoll::Shutdown;
            }
            let next = async {
                match self.stream.next().await {
                    None => SignalPoll::Disconnected,
                    Some(message) => match message.body().deserialize::<(bool,)>() {
                        Ok((start,)) => SignalPoll::Event(LogindEvent::PrepareForSleep(start)),
                        Err(error) => {
                            tracing::warn!(reason = %error, "logind_signal_decode_failed");
                            SignalPoll::Malformed
                        }
                    },
                }
            };
            let timeout = async {
                async_io::Timer::after(SIGNAL_POLL_TIMEOUT).await;
                SignalPoll::Timeout
            };
            future::or(next, timeout).await
        })
    }
}

#[derive(Debug)]
pub struct LogindWatcher {
    shutdown: Arc<AtomicBool>,
    receiver: Receiver<LogindEvent>,
    handle: Option<JoinHandle<()>>,
}

impl LogindWatcher {
    pub fn spawn() -> Option<Self> {
        let (sender, receiver) = mpsc::sync_channel(HINT_CHANNEL_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let handle = thread::Builder::new()
            .name(String::from("systemd-logind-resume"))
            .spawn(move || run_listener(sender, thread_shutdown))
            .ok()?;
        Some(Self {
            shutdown,
            receiver,
            handle: Some(handle),
        })
    }

    pub fn try_recv(&self) -> Option<LogindEvent> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for LogindWatcher {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_listener(sender: SyncSender<LogindEvent>, shutdown: Arc<AtomicBool>) {
    zbus::block_on(run_listener_async(sender, shutdown));
}

async fn run_listener_async(sender: SyncSender<LogindEvent>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        listen_once(&sender, &shutdown).await;
        wait_for_reconnect(&shutdown).await;
    }
}

async fn wait_for_reconnect(shutdown: &AtomicBool) {
    wait_for_reconnect_with(shutdown, |interval| async move {
        async_io::Timer::after(interval).await;
    })
    .await;
}

async fn wait_for_reconnect_with<F, Fut>(shutdown: &AtomicBool, mut wait: F)
where
    F: FnMut(Duration) -> Fut,
    Fut: Future<Output = ()>,
{
    let mut waited = Duration::ZERO;
    while !shutdown.load(Ordering::Relaxed) && waited < RECONNECT_DELAY {
        let interval = RECONNECT_DELAY
            .checked_sub(waited)
            .unwrap_or(Duration::ZERO)
            .min(SHUTDOWN_POLL_INTERVAL);
        wait(interval).await;
        waited += interval;
    }
}

async fn listen_once(sender: &SyncSender<LogindEvent>, shutdown: &AtomicBool) {
    // Each setup await is cancellable by a bounded timeout. This prevents
    // Drop from depending on a stalled bus daemon during clean shutdown.
    let connection = match bounded_setup(Connection::system()).await {
        Some(Ok(connection)) => connection,
        Some(Err(error)) => {
            tracing::info!(reason = %error, "logind_unavailable");
            return;
        }
        None => return,
    };

    let proxy = match bounded_setup(Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    ))
    .await
    {
        Some(Ok(proxy)) => proxy,
        Some(Err(error)) => {
            tracing::info!(reason = %error, "logind_unavailable");
            return;
        }
        None => return,
    };

    let mut stream = match bounded_setup(proxy.receive_signal("PrepareForSleep")).await {
        Some(Ok(stream)) => stream,
        Some(Err(error)) => {
            tracing::info!(reason = %error, "logind_signal_unavailable");
            return;
        }
        None => return,
    };

    let mut adapter = ZbusSignalAdapter {
        stream: &mut stream,
        shutdown,
    };
    let _ = run_signal_session(&mut adapter, sender, shutdown).await;
}

async fn bounded_setup<F, T, E>(operation: F) -> Option<Result<T, E>>
where
    F: Future<Output = Result<T, E>>,
{
    future::or(async { Some(operation.await) }, async {
        async_io::Timer::after(SIGNAL_POLL_TIMEOUT).await;
        None
    })
    .await
}

/// Shared production/test listener state machine. The adapter is injected as
/// a future-producing closure so tests drive this exact reconnect session
/// logic rather than a copy of its edge handling.
async fn run_signal_session<A: SignalAdapter + ?Sized>(
    adapter: &mut A,
    sender: &SyncSender<LogindEvent>,
    shutdown: &AtomicBool,
) -> SessionOutcome {
    let mut sleeping = false;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return SessionOutcome::Shutdown;
        }
        match adapter.next().await {
            SignalPoll::Event(LogindEvent::PrepareForSleep(start)) => {
                if resume_edge(&mut sleeping, start) {
                    let correlation_id = NEXT_CORRELATION_ID.fetch_add(1, Ordering::Relaxed);
                    tracing::info!(correlation_id, "logind_system_resume_hint");
                    let _ = sender.try_send(LogindEvent::SystemResume { correlation_id });
                }
            }
            SignalPoll::Event(LogindEvent::SystemResume { .. }) => {
                // This is an internal channel event, never a bus payload.
                return SessionOutcome::Disconnected;
            }
            SignalPoll::Malformed => return SessionOutcome::Disconnected,
            SignalPoll::Timeout => {}
            SignalPoll::Disconnected => return SessionOutcome::Disconnected,
            SignalPoll::Shutdown => return SessionOutcome::Shutdown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::future;

    fn shutdown_flag() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    struct ScriptedAdapter {
        events: Vec<SignalPoll>,
    }

    impl SignalAdapter for ScriptedAdapter {
        fn next<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = SignalPoll> + 'a>> {
            Box::pin(future::ready(self.events.remove(0)))
        }
    }

    #[test]
    fn missing_registration_is_terminal_and_emits_no_hint() {
        let shutdown = shutdown_flag();
        let (sender, receiver) = mpsc::sync_channel(HINT_CHANNEL_CAPACITY);
        let mut adapter = ScriptedAdapter {
            events: vec![SignalPoll::Disconnected],
        };
        let outcome = future::block_on(run_signal_session(&mut adapter, &sender, &shutdown));
        assert_eq!(outcome, SessionOutcome::Disconnected);
        assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    #[test]
    fn production_session_emits_only_true_to_false_and_stops_on_disconnect() {
        let shutdown = shutdown_flag();
        let (sender, receiver) = mpsc::sync_channel(HINT_CHANNEL_CAPACITY);
        let mut adapter = ScriptedAdapter {
            events: vec![
                SignalPoll::Event(LogindEvent::PrepareForSleep(false)),
                SignalPoll::Event(LogindEvent::PrepareForSleep(true)),
                SignalPoll::Event(LogindEvent::PrepareForSleep(false)),
                SignalPoll::Disconnected,
            ],
        };
        let outcome = future::block_on(run_signal_session(&mut adapter, &sender, &shutdown));
        assert_eq!(outcome, SessionOutcome::Disconnected);
        assert!(matches!(
            receiver.try_recv(),
            Ok(LogindEvent::SystemResume { correlation_id: _ })
        ));
        assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    #[test]
    fn malformed_signal_is_terminal_and_does_not_reconnect_spin() {
        let shutdown = shutdown_flag();
        let (sender, receiver) = mpsc::sync_channel(HINT_CHANNEL_CAPACITY);
        let mut adapter = ScriptedAdapter {
            events: vec![SignalPoll::Malformed],
        };
        let outcome = future::block_on(run_signal_session(&mut adapter, &sender, &shutdown));
        assert_eq!(outcome, SessionOutcome::Disconnected);
        assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    #[test]
    fn shutdown_terminates_injected_session_without_reconnect_or_hint() {
        let shutdown = shutdown_flag();
        shutdown.store(true, Ordering::Relaxed);
        let (sender, receiver) = mpsc::sync_channel(HINT_CHANNEL_CAPACITY);
        let mut adapter = ScriptedAdapter {
            events: vec![SignalPoll::Event(LogindEvent::PrepareForSleep(true))],
        };
        let outcome = future::block_on(run_signal_session(&mut adapter, &sender, &shutdown));
        assert_eq!(outcome, SessionOutcome::Shutdown);
        assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    #[test]
    fn reconnect_wait_is_interruptible_in_fixed_bounded_slices() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let elapsed = Arc::new(std::sync::Mutex::new(Duration::ZERO));
        let clock = Arc::clone(&elapsed);
        future::block_on(wait_for_reconnect_with(&shutdown, move |interval| {
            let clock = Arc::clone(&clock);
            async move {
                *clock.lock().expect("fake clock lock") += interval;
            }
        }));
        assert_eq!(*elapsed.lock().expect("fake clock lock"), RECONNECT_DELAY);
        shutdown.store(true, Ordering::Relaxed);
        let elapsed = Arc::new(std::sync::Mutex::new(Duration::ZERO));
        let clock = Arc::clone(&elapsed);
        future::block_on(wait_for_reconnect_with(&shutdown, move |interval| {
            let clock = Arc::clone(&clock);
            async move {
                *clock.lock().expect("fake clock lock") += interval;
            }
        }));
        assert_eq!(*elapsed.lock().expect("fake clock lock"), Duration::ZERO);
    }

    #[test]
    fn channel_is_bounded_and_coalesces_burst() {
        let (sender, receiver) = mpsc::sync_channel(HINT_CHANNEL_CAPACITY);
        assert!(sender
            .try_send(LogindEvent::SystemResume { correlation_id: 1 })
            .is_ok());
        assert!(sender
            .try_send(LogindEvent::SystemResume { correlation_id: 2 })
            .is_err());
        assert_eq!(
            receiver.try_recv(),
            Ok(LogindEvent::SystemResume { correlation_id: 1 })
        );
    }
}
