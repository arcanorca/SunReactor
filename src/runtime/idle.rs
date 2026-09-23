use crate::backends::ProcessRunner;
use crate::runtime::orchestrator::DaemonRuntime;
use crate::runtime::r#loop::LoopCadence;
use chrono::{DateTime, Utc};
#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::{Child, Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(all(target_os = "linux", feature = "wayland"))]
use wayland_client::globals::registry_queue_init;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use wayland_client::protocol::{wl_registry, wl_seat::WlSeat};
#[cfg(all(target_os = "linux", feature = "wayland"))]
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
#[cfg(all(target_os = "linux", feature = "wayland"))]
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{Event as IdleNotificationEvent, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

/// Raised by the Wayland idle watcher when input resumes after idling; the
/// daemon loop takes it to open a wake watch. The watcher runs inside the
/// daemon, so it signals directly instead of spawning `sunreactorctl`.
#[cfg(target_os = "linux")]
static INPUT_RESUMED: AtomicBool = AtomicBool::new(false);

// --- From desktop_idle.rs ---
const STARTUP_WAKE_SYNC_DELAY: Duration = Duration::from_secs(8);
const FOLLOWUP_WAKE_SYNC_DELAY: Duration = Duration::from_secs(15);
const SUSPEND_DRIFT_THRESHOLD_SECONDS: u64 = 5;

#[cfg(target_os = "linux")]
const XPRINTIDLE_POLL_INTERVAL: Duration = Duration::from_secs(30);

#[cfg(target_os = "linux")]
const XPRINTIDLE_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(all(target_os = "linux", feature = "wayland"))]
const WAYLAND_WAKE_HINT_IDLE_TIMEOUT_MS: u32 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdleSyncAction {
    DelayedSync,
    DriftRecovery { drift_seconds: u64 },
}

pub(super) struct DesktopIdleSync {
    enabled: bool,
    #[cfg(target_os = "linux")]
    watcher: Option<IdleWatcher>,
    delayed_sync_at: Option<Instant>,
    last_tick_utc: DateTime<Utc>,
    #[cfg(target_os = "linux")]
    last_spawn_attempt: Option<Instant>,
    timeout_minutes: u64,
}

impl DesktopIdleSync {
    pub(super) fn new(
        enabled: bool,
        socket_path: PathBuf,
        tick_seconds: u64,
        timeout_minutes: u64,
    ) -> Self {
        #[cfg(target_os = "linux")]
        let watcher: Option<IdleWatcher> = {
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
                spawn_wayland_idle()
            } else if enabled {
                spawn(socket_path, timeout_minutes)
            } else {
                None
            }
            #[cfg(not(feature = "wayland"))]
            if enabled {
                spawn(socket_path, timeout_minutes)
            } else {
                None
            }
        };
        #[cfg(not(target_os = "linux"))]
        let _ = (socket_path, timeout_minutes);

        Self::new_with_watcher(
            enabled,
            #[cfg(target_os = "linux")]
            watcher,
            tick_seconds,
            timeout_minutes,
        )
    }

    fn new_with_watcher(
        enabled: bool,
        #[cfg(target_os = "linux")] watcher: Option<IdleWatcher>,
        tick_seconds: u64,
        timeout_minutes: u64,
    ) -> Self {
        Self {
            enabled,
            #[cfg(target_os = "linux")]
            watcher,
            delayed_sync_at: enabled.then_some(Instant::now() + STARTUP_WAKE_SYNC_DELAY),
            last_tick_utc: Utc::now()
                .checked_sub_signed(chrono::Duration::seconds(tick_seconds as i64))
                .unwrap_or_else(Utc::now),
            #[cfg(target_os = "linux")]
            last_spawn_attempt: enabled.then_some(Instant::now()),
            timeout_minutes,
        }
    }

    pub(super) fn update_config(&mut self, enabled: bool, timeout_minutes: u64) {
        if self.enabled != enabled || self.timeout_minutes != timeout_minutes {
            self.enabled = enabled;
            self.timeout_minutes = timeout_minutes;
            #[cfg(target_os = "linux")]
            {
                self.watcher = None; // Force respawn in maintain_watcher if enabled
            }
        }
    }

    pub(super) fn note_tick_attempt(&mut self, now_utc: DateTime<Utc>) {
        self.last_tick_utc = now_utc;
    }

    pub(super) fn maintain_watcher(&mut self, socket_path: &std::path::Path) {
        if !self.enabled {
            return;
        }

        #[cfg(target_os = "linux")]
        {
            let needs_restart = match &self.watcher {
                Some(watcher) => !watcher.is_alive(),
                None => self
                    .last_spawn_attempt
                    .is_none_or(|last| last.elapsed() > Duration::from_mins(1)),
            };

            if needs_restart {
                if self.watcher.is_some() {
                    tracing::info!("idle_watcher_died_restarting");
                }
                self.last_spawn_attempt = Some(Instant::now());
                self.watcher = spawn(socket_path.to_path_buf(), self.timeout_minutes);
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = socket_path;
        }
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.delayed_sync_at
    }

    /// Whether user input resumed after idling since the last call.
    pub(super) fn take_input_resume() -> bool {
        #[cfg(target_os = "linux")]
        {
            INPUT_RESUMED.swap(false, Ordering::AcqRel)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub(super) fn perform_due_action<R: ProcessRunner + Sync>(
        &mut self,
        runtime: &mut DaemonRuntime,
        now_utc: DateTime<Utc>,
        runner: &R,
        cadence: &mut LoopCadence,
    ) -> bool {
        let Some(action) = self.due_action(now_utc, cadence.elapsed_since_tick()) else {
            return false;
        };

        match action {
            IdleSyncAction::DelayedSync => {
                tracing::info!("delayed_wake_sync_triggered");
                runtime.execute_resync_tick(now_utc, runner, cadence, self, false);
            }
            IdleSyncAction::DriftRecovery { drift_seconds } => {
                tracing::info!(drift_secs = %drift_seconds, "time_jump_detected");
                runtime.execute_resync_tick(now_utc, runner, cadence, self, true);
                // Monitors may still be waking after a suspend.
                let _ =
                    runtime.request_wake_reassert(crate::runtime::wake::WakeReason::SystemResume);
            }
        }
        true
    }

    fn due_action(
        &mut self,
        now_utc: DateTime<Utc>,
        elapsed_since_tick: Duration,
    ) -> Option<IdleSyncAction> {
        if self.delayed_sync_due() {
            self.delayed_sync_at = None;
            return Some(IdleSyncAction::DelayedSync);
        }

        let elapsed_real = now_utc
            .signed_duration_since(self.last_tick_utc)
            .num_seconds()
            .max(0) as u64;
        let drift_seconds = elapsed_real.saturating_sub(elapsed_since_tick.as_secs());

        if drift_seconds > SUSPEND_DRIFT_THRESHOLD_SECONDS {
            self.delayed_sync_at = Some(Instant::now() + FOLLOWUP_WAKE_SYNC_DELAY);
            return Some(IdleSyncAction::DriftRecovery { drift_seconds });
        }

        None
    }

    fn delayed_sync_due(&self) -> bool {
        self.delayed_sync_at
            .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

// ---------------------------------------------------------------------------
// Idle watcher: multi-strategy backend
// ---------------------------------------------------------------------------

/// Wraps the active idle-detection backend. Supports both long-lived child
/// processes (swayidle) and polling threads (xprintidle).
///
/// Dropping the watcher cleans up the underlying resource (kills the child
/// process or signals the polling thread to shut down).
#[cfg(target_os = "linux")]
pub struct IdleWatcher {
    inner: IdleWatcherInner,
}

#[cfg(target_os = "linux")]
enum IdleWatcherInner {
    /// A long-lived child process (e.g. swayidle) that directly executes
    /// sunreactorctl idle-dim / idle-wake on timeout / resume events.
    Subprocess(Arc<Mutex<Child>>),

    /// A polling thread that periodically invokes xprintidle to measure
    /// user idle time and dispatches idle-dim / idle-wake accordingly.
    PollingThread {
        shutdown: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    },
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    WaylandThread {
        shutdown: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    },
}

#[cfg(target_os = "linux")]
impl IdleWatcher {
    pub(super) fn is_alive(&self) -> bool {
        match &self.inner {
            IdleWatcherInner::Subprocess(child) => {
                if let Ok(mut child) = child.lock() {
                    matches!(child.try_wait(), Ok(None))
                } else {
                    false
                }
            }
            IdleWatcherInner::PollingThread { shutdown, handle } => {
                !shutdown.load(Ordering::Relaxed)
                    && handle.as_ref().is_some_and(|h| !h.is_finished())
            }
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            IdleWatcherInner::WaylandThread { shutdown, handle } => {
                !shutdown.load(Ordering::Relaxed)
                    && handle.as_ref().is_some_and(|h| !h.is_finished())
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for IdleWatcher {
    fn drop(&mut self) {
        match &mut self.inner {
            IdleWatcherInner::Subprocess(child) => {
                if let Ok(mut child) = child.lock() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            IdleWatcherInner::PollingThread { shutdown, handle } => {
                shutdown.store(true, Ordering::Relaxed);
                if let Some(handle) = handle.take() {
                    let _ = handle.join();
                }
            }
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            IdleWatcherInner::WaylandThread { shutdown, handle } => {
                shutdown.store(true, Ordering::Relaxed);
                if let Some(handle) = handle.take() {
                    let _ = handle.join();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Strategy selection
// ---------------------------------------------------------------------------

/// Tries idle detection strategies in priority order:
///
/// 1. **swayidle** — Wayland compositors with wlr-idle support (Sway, Hyprland, …)
/// 2. **xprintidle polling** — X11 sessions (DISPLAY set, WAYLAND_DISPLAY absent)
/// 3. **None** — drift-only fallback (suspend/resume still detected by
///    `DesktopIdleSync::due_action`)
#[cfg(target_os = "linux")]
pub(super) fn spawn(_socket_path: PathBuf, timeout_minutes: u64) -> Option<IdleWatcher> {
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
        if let Some(watcher) = spawn_wayland_idle() {
            return Some(watcher);
        }
    }

    // Strategy 1: swayidle (Wayland wlr-idle)
    if let Some(watcher) = spawn_swayidle(timeout_minutes) {
        return Some(watcher);
    }

    // Strategy 2: xprintidle polling (X11)
    if let Some(watcher) = spawn_xprintidle_poll(timeout_minutes) {
        return Some(watcher);
    }

    // No suitable backend — drift-only fallback is still active via
    // DesktopIdleSync::due_action, but user-idle-timeout detection is
    // not available.
    tracing::info!(
        reason = "no suitable backend (swayidle and xprintidle unavailable)",
        "idle_watcher_disabled"
    );
    None
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
struct WaylandIdleState {
    shutdown: Arc<AtomicBool>,
    _notification: Option<ExtIdleNotificationV1>,
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
impl Dispatch<wl_registry::WlRegistry, wayland_client::globals::GlobalListContents>
    for WaylandIdleState
{
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &wayland_client::globals::GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
#[allow(clippy::ignored_unit_patterns)]
impl Dispatch<WlSeat, ()> for WaylandIdleState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
#[allow(clippy::ignored_unit_patterns)]
impl Dispatch<ExtIdleNotifierV1, ()> for WaylandIdleState {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: <ExtIdleNotifierV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
#[allow(clippy::ignored_unit_patterns)]
impl Dispatch<ExtIdleNotificationV1, ()> for WaylandIdleState {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotificationV1,
        event: IdleNotificationEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, IdleNotificationEvent::Resumed) {
            INPUT_RESUMED.store(true, Ordering::Release);
        }
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
fn spawn_wayland_idle() -> Option<IdleWatcher> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let thread_shutdown = Arc::clone(&shutdown);
    let timeout_ms = WAYLAND_WAKE_HINT_IDLE_TIMEOUT_MS;
    let handle = std::thread::Builder::new()
        .name(String::from("wayland-idle-notify"))
        .spawn(move || {
            let Ok(connection) = Connection::connect_to_env() else {
                tracing::info!(reason = "connection failed", "wayland_idle_unavailable");
                return;
            };
            let Ok((globals, mut queue)) = registry_queue_init::<WaylandIdleState>(&connection)
            else {
                tracing::info!(reason = "registry failed", "wayland_idle_unavailable");
                return;
            };
            let qh = queue.handle();
            let Ok(seat) = globals.bind::<WlSeat, _, _>(&qh, 1..=9, ()) else {
                tracing::info!(reason = "seat unavailable", "wayland_idle_unavailable");
                return;
            };
            let Ok(notifier) = globals.bind::<ExtIdleNotifierV1, _, _>(&qh, 1..=2, ()) else {
                tracing::info!(
                    reason = "ext-idle-notify unavailable",
                    "wayland_idle_unavailable"
                );
                return;
            };
            let notification = notifier.get_input_idle_notification(timeout_ms, &seat, &qh, ());
            let mut state = WaylandIdleState {
                shutdown: thread_shutdown,
                _notification: Some(notification),
            };
            while !state.shutdown.load(Ordering::Relaxed) {
                if !dispatch_wayland_once(&mut queue, &mut state) {
                    break;
                }
            }
        })
        .ok()?;
    tracing::info!(strategy = "wayland_ext_idle_notify", "idle_watcher_started");
    Some(IdleWatcher {
        inner: IdleWatcherInner::WaylandThread {
            shutdown,
            handle: Some(handle),
        },
    })
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
fn dispatch_wayland_once(
    queue: &mut wayland_client::EventQueue<WaylandIdleState>,
    state: &mut WaylandIdleState,
) -> bool {
    let _ = queue.dispatch_pending(state);
    let Some(read_guard) = queue.prepare_read() else {
        return true;
    };
    let mut pollfd = libc::pollfd {
        fd: read_guard.connection_fd().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&raw mut pollfd, 1, 100) };
    if state.shutdown.load(Ordering::Relaxed) {
        drop(read_guard);
        return false;
    }
    if result < 0 {
        drop(read_guard);
        return false;
    }
    if result == 0 {
        drop(read_guard);
        return true;
    }
    if read_guard.read().is_err() {
        return false;
    }
    queue.dispatch_pending(state).is_ok()
}

// ---------------------------------------------------------------------------
// Strategy 1: swayidle (Wayland wlr-idle)
// ---------------------------------------------------------------------------

/// Spawns `swayidle` as a long-lived child process. swayidle natively
/// monitors Wayland idle events and executes `sunreactorctl idle-dim`
/// on timeout and `sunreactorctl idle-wake` on resume.
///
/// Returns `None` if swayidle is not installed or fails to start.
#[cfg(target_os = "linux")]
fn spawn_swayidle(timeout_minutes: u64) -> Option<IdleWatcher> {
    let timeout_seconds = timeout_minutes * 60;

    // SAFETY: pre_exec sets PR_SET_PDEATHSIG so the child is killed when the
    // daemon exits. This is called between fork() and exec() where only
    // async-signal-safe operations are permitted; prctl is safe here.
    let child = match unsafe {
        Command::new("swayidle")
            .arg("-w")
            .arg("timeout")
            .arg(timeout_seconds.to_string())
            .arg(format!("{} idle-dim", cli_binary().display()))
            .arg("resume")
            .arg(format!("{} idle-wake", cli_binary().display()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            })
            .spawn()
    } {
        Ok(child) => child,
        Err(error) => {
            tracing::info!(
                reason = %format!("failed to start swayidle: {error}"),
                "idle_watcher_swayidle_skipped"
            );
            return None;
        }
    };

    tracing::info!(strategy = "swayidle", "idle_watcher_started");

    Some(IdleWatcher {
        inner: IdleWatcherInner::Subprocess(Arc::new(Mutex::new(child))),
    })
}

// ---------------------------------------------------------------------------
// Strategy 2: xprintidle polling (X11)
// ---------------------------------------------------------------------------

/// Spawns a polling thread that periodically calls `xprintidle` to read the
/// X11 idle time. When the user has been idle longer than `timeout_minutes`,
/// the thread runs `sunreactorctl idle-dim`. When activity resumes it runs
/// `sunreactorctl idle-wake`.
///
/// Only attempted when `DISPLAY` is set and `WAYLAND_DISPLAY` is absent,
/// indicating a pure X11 session. Returns `None` if xprintidle is not
/// installed or the session is not X11.
#[cfg(target_os = "linux")]
fn spawn_xprintidle_poll(timeout_minutes: u64) -> Option<IdleWatcher> {
    // Only try on X11 sessions: DISPLAY must be set, WAYLAND_DISPLAY must not.
    let has_display = std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty());
    let has_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());

    if !has_display || has_wayland {
        tracing::info!(
            reason = "not an X11-only session",
            "idle_watcher_xprintidle_skipped"
        );
        return None;
    }

    // Probe xprintidle availability with a single bounded invocation before
    // committing to the polling thread.
    if !is_xprintidle_available() {
        tracing::info!(
            reason = "xprintidle not installed or not functional",
            "idle_watcher_xprintidle_skipped"
        );
        return None;
    }

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    let timeout_ms = timeout_minutes * 60 * 1000;

    let handle = std::thread::Builder::new()
        .name("xprintidle-poll".into())
        .spawn(move || {
            xprintidle_poll_loop(&shutdown_clone, timeout_ms);
        })
        .ok()?;

    tracing::info!(strategy = "xprintidle_poll", "idle_watcher_started");

    Some(IdleWatcher {
        inner: IdleWatcherInner::PollingThread {
            shutdown,
            handle: Some(handle),
        },
    })
}

/// Checks whether `xprintidle` is installed and returns a plausible idle-time
/// value. A single bounded invocation; output is treated as untrusted.
#[cfg(target_os = "linux")]
fn is_xprintidle_available() -> bool {
    let output = Command::new("xprintidle")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let Ok(mut child) = output else {
        return false;
    };

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return false;
                }
                // Verify stdout contains a parseable number (untrusted output).
                if let Some(stdout) = child.stdout.take() {
                    let mut buf = Vec::new();
                    if std::io::Read::read_to_end(&mut std::io::BufReader::new(stdout), &mut buf)
                        .is_ok()
                    {
                        let text = String::from_utf8_lossy(&buf);
                        return text.trim().parse::<u64>().is_ok();
                    }
                }
                return false;
            }
            Ok(None) => {
                if start.elapsed() >= XPRINTIDLE_COMMAND_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

/// Core polling loop for the xprintidle strategy. Runs on a dedicated thread.
///
/// Calls `xprintidle` every `XPRINTIDLE_POLL_INTERVAL` seconds, compares the
/// reported idle time against `timeout_ms`, and dispatches the appropriate
/// `sunreactorctl` command when state transitions occur.
#[cfg(target_os = "linux")]
fn xprintidle_poll_loop(shutdown: &Arc<AtomicBool>, timeout_ms: u64) {
    let mut is_idle = false;

    while !shutdown.load(Ordering::Relaxed) {
        if let Some(idle_ms) = read_xprintidle() {
            let was_idle = is_idle;

            if idle_ms >= timeout_ms && !was_idle {
                is_idle = true;
                fire_sunreactorctl("idle-dim");
            } else if idle_ms < timeout_ms && was_idle {
                is_idle = false;
                fire_sunreactorctl("idle-wake");
            }
        }
        // else: xprintidle invocation failed this cycle — skip and retry next
        // interval rather than flapping state.

        // Sleep in short increments so shutdown is responsive.
        let deadline = Instant::now() + XPRINTIDLE_POLL_INTERVAL;
        while Instant::now() < deadline {
            if shutdown.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

/// Invokes `xprintidle` with a bounded timeout and parses the idle-time output
/// (milliseconds). Returns `None` on any failure (missing binary, timeout,
/// non-numeric output). Output is treated as untrusted.
#[cfg(target_os = "linux")]
fn read_xprintidle() -> Option<u64> {
    let mut child = Command::new("xprintidle")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let stdout = child.stdout.take()?;
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut std::io::BufReader::new(stdout), &mut buf).ok()?;
                let text = String::from_utf8_lossy(&buf);
                return text.trim().parse::<u64>().ok();
            }
            Ok(None) => {
                if start.elapsed() >= XPRINTIDLE_COMMAND_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// `sunreactorctl` installed next to the running daemon, falling back to
/// `PATH`. User services often run without `~/.local/bin` on `PATH`.
#[cfg(target_os = "linux")]
fn cli_binary() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(crate::CLI_BINARY)))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(crate::CLI_BINARY))
}

/// Fires a `sunreactorctl` subcommand (`idle-dim` or `idle-wake`) as a
/// short-lived child process. Failures are logged but do not stop the
/// polling loop.
#[cfg(target_os = "linux")]
fn fire_sunreactorctl(subcommand: &str) {
    let result = Command::new(cli_binary())
        .arg(subcommand)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match result {
        Ok(mut child) => {
            // Wait up to 10 s for the command to complete, then abandon.
            let start = Instant::now();
            let deadline = Duration::from_secs(10);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if start.elapsed() >= deadline {
                            tracing::info!(
                                cmd = subcommand,
                                "xprintidle_poll_sunreactorctl_timeout"
                            );
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(error) => {
                        tracing::info!(
                            cmd = subcommand,
                            error = %error,
                            "xprintidle_poll_sunreactorctl_error"
                        );
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }
        Err(error) => {
            tracing::info!(
                cmd = subcommand,
                error = %error,
                "xprintidle_poll_sunreactorctl_spawn_failed"
            );
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::backends::{BackendKind, FailureKind};
    use crate::config::{
        Config, ConfigReport, ConfigSource, DaemonConfig, LocationConfig, LogLevel, MonitorConfig,
        MonitorSelector, SolarPolicyConfig, WeatherConfig,
    };
    use crate::process::{CommandError, CommandOutput};
    use crate::state::FailureBackoffState;
    use chrono::TimeZone;

    fn test_idle_sync(enabled: bool, tick_seconds: u64) -> DesktopIdleSync {
        DesktopIdleSync::new_with_watcher(enabled, None, tick_seconds, 15)
    }

    struct RecordingRunner {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl ProcessRunner for RecordingRunner {
        fn run(
            &self,
            program: &str,
            args: &[String],
            _timeout: Duration,
        ) -> Result<CommandOutput, CommandError> {
            let mut call = String::from(program);
            for arg in args {
                call.push('|');
                call.push_str(arg);
            }
            self.calls.lock().unwrap().push(call);
            Ok(CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
            })
        }
    }

    #[test]
    fn disabled_idle_sync_performs_drift_recovery_but_no_idle_deadlines() {
        let mut idle_sync = test_idle_sync(false, 60);
        let base_time = Utc.timestamp_opt(1_800_000_000, 0).single().expect("valid");
        idle_sync.last_tick_utc = base_time;

        assert!(idle_sync.next_deadline().is_none());

        // A small normal tick shouldn't trigger drift
        let small_advance = base_time + Duration::from_mins(1);
        assert_eq!(
            idle_sync.due_action(small_advance, std::time::Duration::from_mins(1)),
            None
        );

        // A massive jump (e.g., waking from sleep) MUST trigger drift recovery,
        // even if idle sync is disabled.
        let massive_jump = base_time + Duration::from_hours(1);
        let action = idle_sync.due_action(massive_jump, std::time::Duration::from_mins(1));
        assert!(matches!(action, Some(IdleSyncAction::DriftRecovery { .. })));
    }

    #[test]
    fn enabled_idle_sync_starts_with_delayed_sync_deadline() {
        let mut enabled = test_idle_sync(true, 60);
        enabled.delayed_sync_at = Some(Instant::now());

        let now = Utc.timestamp_opt(1_800_000_000, 0).single().expect("valid");
        assert_eq!(
            enabled.due_action(now, std::time::Duration::from_mins(1)),
            Some(IdleSyncAction::DelayedSync)
        );
    }

    #[test]
    fn drift_recovery_schedules_followup_sync() {
        let mut idle_sync = test_idle_sync(true, 60);
        idle_sync.delayed_sync_at = None;
        idle_sync.last_tick_utc = Utc.timestamp_opt(1_800_000_000, 0).single().expect("valid");

        let action = idle_sync.due_action(
            Utc.timestamp_opt(1_800_000_030, 0).single().expect("valid"),
            std::time::Duration::from_secs(10),
        );

        assert_eq!(
            action,
            Some(IdleSyncAction::DriftRecovery { drift_seconds: 20 })
        );
        assert!(idle_sync.next_deadline().is_some());
    }

    #[test]
    fn delayed_sync_forces_reapply_even_with_active_backoff() {
        let temp = std::env::temp_dir().join(format!(
            "sunreactor-desktop-idle-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&temp).expect("temp dir should be created");
        let state_path = temp.join("state/runtime-state.json");
        let socket_path = temp.join("run/control.sock");
        let mut runtime =
            DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
                .expect("runtime should bootstrap");
        runtime.last_capabilities = Some(crate::runtime::topology::CapabilitySnapshot {
            ddc_present: Vec::new(),
            backlights_present: vec![crate::discovery::BacklightDeviceDiscovery {
                stable_id: String::from("test:backlight"),
                device_name: String::from("intel_backlight"),
                class: String::from("backlight"),
                max_brightness: Some(100),
                backlight_type: Some(String::from("raw")),
                probe_source: String::from("test-fixture"),
                sysfs_path: String::from("/sys/class/backlight/intel_backlight"),
                ddcci_connector: None,
                backend_viable: true,
                note: None,
            }],
        });
        runtime.state.monitor_mut("internal").backoff = Some(FailureBackoffState {
            backend: BackendKind::Backlight,
            failure_kind: FailureKind::Transient,
            consecutive_failures: 3,
            suppress_until_epoch_s: Some(1_800_000_500),
        });

        let mut idle_sync = test_idle_sync(true, 60);
        idle_sync.delayed_sync_at = Some(Instant::now());
        let mut cadence = LoopCadence::new(60);
        let runner = RecordingRunner::new();
        let now = Utc.timestamp_opt(1_800_000_000, 0).single().expect("valid");

        assert!(idle_sync.perform_due_action(&mut runtime, now, &runner, &mut cadence));
        assert_eq!(runner.calls().len(), 1);
        assert!(runner.calls()[0]
            .starts_with("brightnessctl|--quiet|--class|backlight|--device|intel_backlight|set|"));
        assert_eq!(
            runtime
                .state
                .monitor("internal")
                .and_then(|monitor| monitor.backoff.as_ref()),
            None
        );

        std::fs::remove_dir_all(&temp).ok();
    }

    fn test_config_report() -> ConfigReport {
        ConfigReport {
            path: std::path::PathBuf::from("/tmp/sunreactor-config.toml"),
            source: ConfigSource::Defaults,
            config: Config {
                daemon: DaemonConfig {
                    tick_seconds: 60,
                    dry_run: false,
                    desktop_idle_sync: true,
                    desktop_idle_timeout_minutes: 0,
                    log_level: LogLevel::Info,
                    apply_reassert_minutes: 2,
                    probe_seconds: 15,
                    ddc_timeout_seconds: 4,
                    backlight_timeout_seconds: 2,
                },
                location: LocationConfig {
                    city: String::new(),
                    latitude: 41.0082,
                    longitude: 28.9784,
                    timezone: String::from("Europe/Istanbul"),
                },
                solar_policy: SolarPolicyConfig {
                    use_adaptive_zenith: true,
                    twilight_elevation_start: -6.0,
                    day_elevation_full: 3.0,
                    min_write_delta_pct: 2,
                    max_step_pct_per_tick: 6,
                },
                monitors: vec![MonitorConfig {
                    logical_id: String::from("internal"),
                    backend: BackendKind::Backlight,
                    enabled: true,
                    allow_topology_retargeting: false,
                    min_pct: 10,
                    max_pct: 100,
                    gain: 1.0,
                    transition_gamma: 1.4,
                    milestone_adjustments: Vec::new(),
                    selector: MonitorSelector {
                        connector: None,
                        serial: None,
                        model: None,
                        edid: None,
                        sysfs_path: Some(String::from("/sys/class/backlight/intel_backlight")),
                        ddc_bus: None,
                        ddc_address: None,
                    },
                }],
                weather: WeatherConfig::default(),
                tui: crate::config::TuiConfig::default(),
            },
            warnings: Vec::new(),
        }
    }
}
