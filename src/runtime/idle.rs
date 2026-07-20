use crate::backends::ProcessRunner;
use crate::runtime::orchestrator::{DaemonRuntime, LoopCadence};
use chrono::{DateTime, Utc};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// --- From desktop_idle.rs ---
const STARTUP_WAKE_SYNC_DELAY: Duration = Duration::from_secs(8);
const FOLLOWUP_WAKE_SYNC_DELAY: Duration = Duration::from_secs(15);
const SUSPEND_DRIFT_THRESHOLD_SECONDS: u64 = 5;

/// How often the xprintidle polling thread checks user idle time.
const XPRINTIDLE_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Timeout for each individual `xprintidle` subprocess invocation.
const XPRINTIDLE_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdleSyncAction {
    DelayedSync,
    DriftRecovery { drift_seconds: u64 },
}

pub(super) struct DesktopIdleSync {
    enabled: bool,
    _watcher: Option<IdleWatcher>,
    delayed_sync_at: Option<Instant>,
    last_tick_utc: DateTime<Utc>,
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
        let watcher = if enabled {
            spawn(socket_path, timeout_minutes)
        } else {
            None
        };

        Self::new_with_watcher(enabled, watcher, tick_seconds, timeout_minutes)
    }

    fn new_with_watcher(
        enabled: bool,
        watcher: Option<IdleWatcher>,
        tick_seconds: u64,
        timeout_minutes: u64,
    ) -> Self {
        Self {
            enabled,
            _watcher: watcher,
            delayed_sync_at: enabled.then_some(Instant::now() + STARTUP_WAKE_SYNC_DELAY),
            last_tick_utc: Utc::now()
                .checked_sub_signed(chrono::Duration::seconds(tick_seconds as i64))
                .unwrap_or_else(Utc::now),
            last_spawn_attempt: enabled.then_some(Instant::now()),
            timeout_minutes,
        }
    }

    pub(super) fn update_config(&mut self, enabled: bool, timeout_minutes: u64) {
        if self.enabled != enabled || self.timeout_minutes != timeout_minutes {
            self.enabled = enabled;
            self.timeout_minutes = timeout_minutes;
            self._watcher = None; // Force respawn in maintain_watcher if enabled
        }
    }

    pub(super) fn note_tick_attempt(&mut self, now_utc: DateTime<Utc>) {
        self.last_tick_utc = now_utc;
    }

    pub(super) fn maintain_watcher(&mut self, socket_path: &std::path::Path) {
        if !self.enabled {
            return;
        }

        let needs_restart = match &self._watcher {
            Some(watcher) => !watcher.is_alive(),
            None => self
                .last_spawn_attempt
                .is_none_or(|last| last.elapsed() > Duration::from_mins(1)),
        };

        if needs_restart {
            if self._watcher.is_some() {
                tracing::info!("idle_watcher_died_restarting");
            }
            self.last_spawn_attempt = Some(Instant::now());
            self._watcher = spawn(socket_path.to_path_buf(), self.timeout_minutes);
        }
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.delayed_sync_at
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

        if !self.enabled {
            return None;
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

#[cfg(test)]
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
    fn disabled_idle_sync_has_no_deadlines_or_drift_actions() {
        let mut idle_sync = test_idle_sync(false, 60);
        let now = Utc.timestamp_opt(1_800_000_000, 0).single().expect("valid");

        assert!(idle_sync.next_deadline().is_none());
        assert_eq!(
            idle_sync.due_action(now, std::time::Duration::from_secs(60)),
            None
        );
    }

    #[test]
    fn enabled_idle_sync_starts_with_delayed_sync_deadline() {
        let mut enabled = test_idle_sync(true, 60);
        enabled.delayed_sync_at = Some(Instant::now());

        let now = Utc.timestamp_opt(1_800_000_000, 0).single().expect("valid");
        assert_eq!(
            enabled.due_action(now, std::time::Duration::from_secs(60)),
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

// ---------------------------------------------------------------------------
// Idle watcher: multi-strategy backend
// ---------------------------------------------------------------------------

/// Wraps the active idle-detection backend. Supports both long-lived child
/// processes (swayidle) and polling threads (xprintidle).
///
/// Dropping the watcher cleans up the underlying resource (kills the child
/// process or signals the polling thread to shut down).
pub struct IdleWatcher {
    inner: IdleWatcherInner,
}

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
}

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
        }
    }
}

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
pub(super) fn spawn(_socket_path: PathBuf, timeout_minutes: u64) -> Option<IdleWatcher> {
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

// ---------------------------------------------------------------------------
// Strategy 1: swayidle (Wayland wlr-idle)
// ---------------------------------------------------------------------------

/// Spawns `swayidle` as a long-lived child process. swayidle natively
/// monitors Wayland idle events and executes `sunreactorctl idle-dim`
/// on timeout and `sunreactorctl idle-wake` on resume.
///
/// Returns `None` if swayidle is not installed or fails to start.
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
            .arg("sunreactorctl idle-dim")
            .arg("resume")
            .arg("sunreactorctl idle-wake")
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
            xprintidle_poll_loop(shutdown_clone, timeout_ms);
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
fn is_xprintidle_available() -> bool {
    let output = Command::new("xprintidle")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match output {
        Ok(c) => c,
        Err(_) => return false,
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
fn xprintidle_poll_loop(shutdown: Arc<AtomicBool>, timeout_ms: u64) {
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

/// Fires a `sunreactorctl` subcommand (`idle-dim` or `idle-wake`) as a
/// short-lived child process. Failures are logged but do not stop the
/// polling loop.
fn fire_sunreactorctl(subcommand: &str) {
    let result = Command::new(crate::CLI_BINARY)
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
