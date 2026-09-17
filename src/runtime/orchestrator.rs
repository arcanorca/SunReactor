use crate::platform::{DisplayEventSources, DisplayRecoveryReason};
use crate::runtime::fade::{self, FadeEngine};
use crate::runtime::idle::DesktopIdleSync;
use chrono::{DateTime, Utc};
use std::fs;
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

#[cfg(target_os = "linux")]
pub(crate) type NativeIpcStream = std::os::unix::net::UnixStream;

#[cfg(target_os = "windows")]
pub(crate) type NativeIpcStream = crate::platform::windows::pipe::NamedPipeStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const IPC_DRAIN_MAX_REQUESTS: usize = 16;
const IPC_DRAIN_MAX_DURATION: Duration = Duration::from_millis(8);

use tracing::{error, info};

use crate::apply::{self, ApplyRecord, ApplyStatus, ApplySummary};
use crate::backends::{FailureKind, ProcessRunner, RealProcessRunner};
use crate::config::{self, ConfigError, ConfigReport, ConfigSource};
use crate::ipc::{self, BoundControlSocket, ControlSocket};
use crate::paths::{self, PathError};
use crate::policy::{PolicyError, PolicyOutput};
use crate::runtime::capabilities::{CapabilityRefreshPublication, CapabilityRefreshState};
use crate::runtime::runtime_config::RuntimeConfig;
use crate::runtime::topology::CapabilitySnapshot;
use crate::runtime::wake::{WakeReason, WakeWatch};
use crate::runtime::weather_refresh::WeatherRefreshState;
use crate::solar::{SolarError, SolarSample};
use crate::state::{RuntimeState, StateError};
use crate::weather;

// --- From mod.rs (Types & Impls) ---
#[cfg(target_os = "linux")]
const RUNTIME_DIR_MODE: u32 = 0o700;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Ipc(#[from] ipc::IpcError),
    #[error(transparent)]
    Paths(#[from] PathError),
    #[error(transparent)]
    Policy(#[from] PolicyError),
    #[error(transparent)]
    Solar(#[from] SolarError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error("failed to access {}: {}", path.display(), source)]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub struct DaemonRuntime {
    pub(super) config: RuntimeConfig,
    pub socket: ControlSocket,
    pub state_path: PathBuf,
    pub state: RuntimeState,
    pub weather_refresh: WeatherRefreshState,
    pub persisted_state_snapshot: RuntimeState,
    pub last_solar_elevation: Option<f64>,
    pub fade_engine: fade::FadeEngine,
    pub weather_engine: weather::WeatherEngine,
    /// Ephemeral capability observation from the last reconciliation.
    /// Not persisted; kept separate from user config.
    pub last_capabilities: Option<CapabilitySnapshot>,
    capability_refresh: CapabilityRefreshState,
    wake_watch: WakeWatch,
    display_events: Option<DisplayEventSources>,
}

#[derive(Debug, Clone)]
pub struct TickReport {
    pub now_utc: DateTime<Utc>,
    pub solar: SolarSample,
    pub policy: PolicyOutput,
    pub apply_summary: ApplySummary,
    pub tick_duration: Duration,
    pub monitors_evaluated: usize,
    pub suspended: bool,
    pub manual_override_active: bool,
    pub weather_modifier_applied: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IpcOutcome {
    pub tick_attempted: bool,
    pub config_reloaded: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IpcDrainOutcome {
    processed: usize,
}

impl DaemonRuntime {
    pub fn run_once(&mut self) -> Result<TickReport, RuntimeError> {
        self.run_once_at_with_runner(Utc::now(), &RealProcessRunner, false)
    }

    /// Perform one complete capability observation before a one-shot apply.
    /// A one-shot invocation has no daemon loop available to perform the
    /// normal asynchronous observation/reconciliation handshake.
    pub fn refresh_capabilities(&mut self) {
        let report = crate::discovery::discover_with_runner(
            &RealProcessRunner,
            std::path::Path::new("/sys/class/backlight"),
        );
        self.last_capabilities = Some(CapabilitySnapshot::from_discovery(&report));
    }

    /// Force-apply variant: bypasses throttles so brightness changes
    /// are applied immediately to the physical monitor.
    pub fn run_once_forced(&mut self) -> Result<TickReport, RuntimeError> {
        self.run_once_at_with_runner(Utc::now(), &RealProcessRunner, true)
    }

    pub(super) fn prepare_apply_resync(&mut self, clear_backoff: bool) {
        for monitor in self.state.monitors.values_mut() {
            monitor.last_applied_percent = None;
            monitor.last_applied_at_epoch_s = None;
            if clear_backoff {
                monitor.backoff = None;
            }
        }
    }

    pub(super) fn run_resync_at_with_runner<R: ProcessRunner + Sync>(
        &mut self,
        now_utc: DateTime<Utc>,
        runner: &R,
        clear_backoff: bool,
    ) -> Result<TickReport, RuntimeError> {
        self.prepare_apply_resync(clear_backoff);
        self.run_once_at_with_runner(now_utc, runner, true)
    }

    /// Starts at most one bounded capability observation away from the control
    /// loop. Only the completed snapshot is published back to the runtime.
    pub(crate) fn begin_capability_refresh_with_runner<R>(&mut self, runner: R)
    where
        R: ProcessRunner + Send + Sync + 'static,
    {
        self.capability_refresh.begin_with_runner(runner);
    }

    fn publish_completed_capability_snapshot(&mut self) -> Option<bool> {
        match self.capability_refresh.publish_completed() {
            Some(CapabilityRefreshPublication::Published(snapshot)) => {
                self.last_capabilities = Some(snapshot);
                Some(true)
            }
            Some(
                CapabilityRefreshPublication::Discarded | CapabilityRefreshPublication::Failed,
            ) => Some(false),
            None => None,
        }
    }

    /// Opens (or extends) the wake watch; probes then fix any brightness a
    /// waking monitor or the desktop restored on its own.
    pub(super) fn request_wake_reassert(&mut self, reason: WakeReason) -> bool {
        let opened = self.wake_watch.start(reason, Instant::now());
        if opened {
            tracing::info!(reason = ?reason, "wake_watch_started");
        } else {
            tracing::debug!(reason = ?reason, "wake_watch_extended");
        }
        opened
    }

    fn handle_display_event(&mut self, reason: DisplayRecoveryReason) {
        let wake_reason = match reason {
            DisplayRecoveryReason::DrmHotplug => WakeReason::DrmHotplug,
            DisplayRecoveryReason::DrmConnectorChange => WakeReason::DrmConnectorChange,
            DisplayRecoveryReason::Resume => WakeReason::SystemResume,
        };
        if self.request_wake_reassert(wake_reason) {
            // A capability observation that started before the change may
            // describe the old topology; make sure it is not published.
            self.capability_refresh.invalidate();
        }
    }

    /// One wake probe: compute the current targets and correct any monitor
    /// that answers with a different brightness.
    pub(super) fn run_wake_probe<R: ProcessRunner>(&mut self, now_utc: DateTime<Utc>, runner: &R) {
        if self.fade_engine.is_fading() {
            return;
        }
        let computed = match self
            .collect_tick_inputs(now_utc, false)
            .and_then(|inputs| self.compute_tick_policy(inputs))
        {
            Ok(computed) => computed,
            Err(error) => {
                tracing::debug!(error = %error, "wake_probe_policy_unavailable");
                return;
            }
        };
        if computed.suspended {
            return;
        }
        let settings = self.force_apply_settings();
        let summary = apply::probe_and_correct(
            &self.config.monitors,
            &settings,
            &computed.policy,
            &mut self.state,
            runner,
            computed.inputs.now_epoch_s,
        );
        tracing::debug!(
            probe = self.wake_watch.probes(),
            answered = summary.answered,
            corrected = summary.corrected,
            failed = summary.failed,
            "wake_probe_completed"
        );
        if summary.corrected > 0 {
            let _ = self.persist_state_if_changed();
        }
    }

    pub(super) fn persist_state_if_changed(&mut self) -> Result<bool, RuntimeError> {
        let candidate = self.state.normalized_for_persistence();
        if candidate == self.persisted_state_snapshot {
            return Ok(false);
        }

        self.state.save_to_path(&self.state_path)?;
        self.persisted_state_snapshot = candidate;
        Ok(true)
    }
}

// --- From loop_timing.rs ---
const SHUTDOWN_SLEEP_SLICE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LoopAction {
    RunTick,
    RunFadeStep,
    Sleep(Duration),
}

#[cfg(target_os = "linux")]
fn wait_for_ipc_or_sleep(listener: &BoundControlSocket, duration: Duration) {
    let mut pollfd = libc::pollfd {
        fd: listener.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let timeout_ms = duration.as_millis().try_into().unwrap_or(i32::MAX);
    unsafe {
        libc::poll(&raw mut pollfd, 1, timeout_ms);
    }
}

#[cfg(target_os = "windows")]
fn wait_for_ipc_or_sleep(listener: &BoundControlSocket, duration: Duration) {
    listener.wait_for_ipc_or_sleep(duration);
}

#[derive(Debug, Clone)]
pub(super) struct LoopCadence {
    tick_interval: Duration,
    last_tick_started: Instant,
}

impl LoopCadence {
    pub(super) fn new(tick_seconds: u64) -> Self {
        let tick_interval = Duration::from_secs(tick_seconds);
        Self {
            tick_interval,
            last_tick_started: Instant::now()
                .checked_sub(tick_interval)
                .unwrap_or_else(Instant::now),
        }
    }

    pub(super) fn note_tick_attempt(&mut self) {
        self.last_tick_started = Instant::now();
    }

    pub(super) fn elapsed_since_tick(&self) -> Duration {
        self.last_tick_started.elapsed()
    }

    pub(super) fn next_action(
        &self,
        extra_deadline: Option<Instant>,
        is_fading: bool,
    ) -> LoopAction {
        let elapsed = self.elapsed_since_tick();

        if is_fading {
            return LoopAction::RunFadeStep;
        }

        if elapsed >= self.tick_interval {
            return LoopAction::RunTick;
        }

        LoopAction::Sleep(self.next_sleep_duration(elapsed, extra_deadline))
    }

    fn next_sleep_duration(&self, elapsed: Duration, extra_deadline: Option<Instant>) -> Duration {
        let remaining = self.tick_interval.saturating_sub(elapsed);
        if let Some(deadline) = extra_deadline {
            let deadline_remaining = deadline.saturating_duration_since(Instant::now());
            remaining.min(deadline_remaining).min(SHUTDOWN_SLEEP_SLICE)
        } else {
            remaining.min(SHUTDOWN_SLEEP_SLICE)
        }
    }
}

fn drain_ready_connections<F>(
    listener: &BoundControlSocket,
    mut handle: F,
) -> Result<IpcDrainOutcome, RuntimeError>
where
    F: FnMut(NativeIpcStream),
{
    let started = Instant::now();
    let mut outcome = IpcDrainOutcome::default();
    while outcome.processed < IPC_DRAIN_MAX_REQUESTS && started.elapsed() < IPC_DRAIN_MAX_DURATION {
        match listener.accept() {
            Ok(Some(stream)) => {
                outcome.processed += 1;
                handle(stream);
            }
            Ok(None) => return Ok(outcome),
            Err(error) => {
                if is_transient_ipc_accept_error(&error) {
                    return Ok(outcome);
                }
                return Err(RuntimeError::from(error));
            }
        }
    }
    Ok(outcome)
}

impl DaemonRuntime {
    fn drain_ipc_requests_with_quantum_impl(
        &mut self,
        listener: &BoundControlSocket,
        cadence: &mut LoopCadence,
        desktop_idle: &mut DesktopIdleSync,
    ) -> Result<IpcDrainOutcome, RuntimeError> {
        let outcome = drain_ready_connections(listener, |stream| {
            let request_outcome = self.handle_ipc_stream(stream);
            if request_outcome.tick_attempted {
                let now_utc = Utc::now();
                cadence.note_tick_attempt();
                desktop_idle.note_tick_attempt(now_utc);
            }
            if request_outcome.config_reloaded {
                desktop_idle.update_config(
                    self.config.daemon.desktop_idle_sync,
                    self.config.daemon.desktop_idle_timeout_minutes,
                );
            }
        })?;
        Ok(outcome)
    }

    pub(super) fn perform_loop_action<R: ProcessRunner + Sync>(
        &mut self,
        action: LoopAction,
        now_utc: DateTime<Utc>,
        runner: &R,
        cadence: &mut LoopCadence,
        desktop_idle: &mut DesktopIdleSync,
        listener: &BoundControlSocket,
    ) {
        match action {
            LoopAction::RunTick => {
                self.execute_scheduled_tick(now_utc, runner, cadence, desktop_idle);
            }
            LoopAction::RunFadeStep => {
                // Fade steps are hardware writes too.  Do not let the high-
                // cadence path bypass topology authorization while the first
                // complete capability observation is still pending.
                if self.last_capabilities.is_none() {
                    wait_for_ipc_or_sleep(listener, Duration::from_millis(16));
                    return;
                }
                let step_start = Instant::now();
                let steps = self.fade_engine.process_tick();
                for (monitor_id, percent) in steps {
                    if let Some(monitor) = self
                        .config
                        .monitors
                        .iter()
                        .find(|m| m.logical_id == monitor_id)
                    {
                        let target = crate::policy::PerMonitorTarget {
                            logical_id: monitor_id,
                            percent,
                            solar_daylight_factor: 0.0,
                            effective_daylight_factor: 0.0,
                        };
                        let settings = self.config.apply;
                        match crate::apply::apply_monitor_target(
                            runner, monitor, &target, percent, &settings,
                        ) {
                            Ok(_) => {
                                self.state.record_apply_success(
                                    &target.logical_id,
                                    percent,
                                    now_utc.timestamp().max(0) as u64,
                                );
                            }
                            Err(error) => {
                                tracing::warn!(
                                    monitor = %target.logical_id,
                                    percent = %percent,
                                    error = %error,
                                    "fade_step_failed"
                                );
                                self.state.record_apply_failure(
                                    &target.logical_id,
                                    monitor.backend,
                                    error.failure_kind(),
                                    now_utc.timestamp().max(0) as u64,
                                );
                                // Stop fading this monitor if it's failing
                                self.fade_engine.active_fades.remove(&target.logical_id);
                            }
                        }
                    }
                }
                if let Some(remaining) = Duration::from_millis(16).checked_sub(step_start.elapsed())
                {
                    wait_for_ipc_or_sleep(listener, remaining);
                }
            }
            LoopAction::Sleep(duration) => wait_for_ipc_or_sleep(listener, duration),
        }
    }

    pub(super) fn execute_scheduled_tick<R: ProcessRunner + Sync>(
        &mut self,
        now_utc: DateTime<Utc>,
        runner: &R,
        cadence: &mut LoopCadence,
        desktop_idle: &mut DesktopIdleSync,
    ) {
        cadence.note_tick_attempt();
        desktop_idle.note_tick_attempt(now_utc);
        match self.run_once_at_with_runner(now_utc, runner, false) {
            Ok(report) => log_tick(&report),
            Err(error) => log_tick_error(&error),
        }
    }

    pub(super) fn execute_resync_tick<R: ProcessRunner + Sync>(
        &mut self,
        now_utc: DateTime<Utc>,
        runner: &R,
        cadence: &mut LoopCadence,
        desktop_idle: &mut DesktopIdleSync,
        clear_backoff: bool,
    ) {
        cadence.note_tick_attempt();
        desktop_idle.note_tick_attempt(now_utc);
        match self.run_resync_at_with_runner(now_utc, runner, clear_backoff) {
            Ok(report) => log_tick(&report),
            Err(error) => log_tick_error(&error),
        }
    }
}

#[cfg(test)]
mod loop_timing_tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use std::os::unix::net::UnixStream;
    #[cfg(target_os = "linux")]
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cadence_runs_tick_when_interval_elapsed() {
        let mut cadence = LoopCadence::new(60);
        cadence.last_tick_started = Instant::now()
            .checked_sub(Duration::from_mins(1))
            .expect("instant subtraction should work");

        assert_eq!(cadence.next_action(None, false), LoopAction::RunTick);
    }

    #[test]
    fn cadence_limits_sleep_to_earliest_deadline() {
        let mut cadence = LoopCadence::new(60);
        cadence.note_tick_attempt();
        let deadline = Some(Instant::now() + Duration::from_millis(10));

        match cadence.next_action(deadline, false) {
            LoopAction::Sleep(duration) => assert!(duration <= Duration::from_millis(10)),
            LoopAction::RunTick => panic!("unexpected tick"),
            LoopAction::RunFadeStep => panic!("unexpected fade step"),
        }
    }

    #[test]
    fn ipc_drain_quantum_is_bounded_by_request_count_and_time() {
        assert_eq!(IPC_DRAIN_MAX_REQUESTS, 16);
        assert!(IPC_DRAIN_MAX_DURATION <= Duration::from_millis(16));
    }

    #[test]
    fn cadence_still_selects_due_tick_after_ipc_quantum_yields() {
        let mut cadence = LoopCadence::new(60);
        cadence.last_tick_started = Instant::now()
            .checked_sub(Duration::from_mins(1))
            .expect("instant subtraction should work");
        assert_eq!(cadence.next_action(None, false), LoopAction::RunTick);
    }

    #[test]
    fn drain_quantum_is_small_enough_to_yield_to_fade_cadence() {
        assert_eq!(IPC_DRAIN_MAX_REQUESTS, 16);
        assert_eq!(IPC_DRAIN_MAX_DURATION, Duration::from_millis(8));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn real_listener_drain_respects_count_and_preserves_backlog() {
        let path = std::env::temp_dir().join(format!(
            "sunreactor-drain-test-{}-{}.sock",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        let socket = ControlSocket { path: path.clone() };
        let listener = socket.bind_listener().expect("listener should bind");
        let mut clients = Vec::new();
        for _ in 0..(IPC_DRAIN_MAX_REQUESTS + 2) {
            let mut client = UnixStream::connect(&path).expect("client should connect");
            ipc::write_json_message(
                &mut client,
                &ipc::RequestEnvelope::new(ipc::Request::Ping),
                &path.display().to_string(),
            )
            .expect("request should write");
            clients.push(client);
        }
        std::thread::sleep(Duration::from_millis(10));

        let handled = std::sync::atomic::AtomicUsize::new(0);
        let first = drain_ready_connections(&listener, |stream| {
            handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            drop(stream);
        })
        .expect("first drain should succeed");
        assert_eq!(first.processed, IPC_DRAIN_MAX_REQUESTS);

        let second = drain_ready_connections(&listener, |stream| {
            handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            drop(stream);
        })
        .expect("second drain should succeed");
        assert!(second.processed >= 1);
        assert_eq!(
            handled.load(std::sync::atomic::Ordering::Relaxed),
            IPC_DRAIN_MAX_REQUESTS + 2
        );
        drop(clients);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn malformed_connections_consume_drain_budget() {
        let path = std::env::temp_dir().join(format!(
            "sunreactor-malformed-drain-test-{}-{}.sock",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should work")
                .as_nanos()
        ));
        let socket = ControlSocket { path: path.clone() };
        let listener = socket.bind_listener().expect("listener should bind");
        let mut clients = Vec::new();
        for _ in 0..=IPC_DRAIN_MAX_REQUESTS {
            let mut client = UnixStream::connect(&path).expect("client should connect");
            std::io::Write::write_all(&mut client, b"not-json\n").expect("frame should write");
            clients.push(client);
        }
        std::thread::sleep(Duration::from_millis(10));
        let handled = std::sync::atomic::AtomicUsize::new(0);
        let result = drain_ready_connections(&listener, |stream| {
            // Production counts the connection before parsing its frame.
            handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            drop(stream);
        })
        .expect("drain should succeed");
        assert_eq!(result.processed, IPC_DRAIN_MAX_REQUESTS);
        assert_eq!(
            handled.load(std::sync::atomic::Ordering::Relaxed),
            IPC_DRAIN_MAX_REQUESTS
        );
        drop(clients);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn one_slow_handler_is_not_preempted_by_drain_budget() {
        let path = std::env::temp_dir().join(format!(
            "sunreactor-slow-drain-test-{}-{}.sock",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        let socket = ControlSocket { path: path.clone() };
        let listener = socket.bind_listener().expect("listener should bind");
        let mut client = UnixStream::connect(&path).expect("client should connect");
        ipc::write_json_message(
            &mut client,
            &ipc::RequestEnvelope::new(ipc::Request::Ping),
            &path.display().to_string(),
        )
        .expect("request should write");
        let started = Instant::now();
        let result = drain_ready_connections(&listener, |_stream| {
            std::thread::sleep(IPC_DRAIN_MAX_DURATION + Duration::from_millis(4));
        })
        .expect("drain should succeed");
        assert_eq!(result.processed, 1);
        assert!(started.elapsed() >= IPC_DRAIN_MAX_DURATION);
    }

    #[test]
    fn due_tick_is_selected_after_a_drained_quantum() {
        let mut cadence = LoopCadence::new(60);
        cadence.last_tick_started = Instant::now()
            .checked_sub(Duration::from_mins(1))
            .expect("instant subtraction should work");
        assert_eq!(cadence.next_action(None, false), LoopAction::RunTick);
    }
}

// --- From helpers.rs ---
/// Pure helper functions used by the runtime — no &mut self, no side effects.
pub(super) fn ipc_error_response(error: &RuntimeError) -> ipc::ResponseEnvelope {
    let code = match error {
        RuntimeError::Ipc(ipc::IpcError::Protocol { .. }) => ipc::ErrorCode::InvalidRequest,
        _ => ipc::ErrorCode::InternalError,
    };
    ipc::ResponseEnvelope::error(code, error.to_string())
}

pub(super) fn run_once_response(report: &TickReport) -> ipc::RunOnceResponse {
    ipc::RunOnceResponse {
        tick_duration_ms: report.tick_duration.as_millis().min(u128::from(u64::MAX)) as u64,
        monitors_evaluated: report.monitors_evaluated as u32,
        writes_attempted: report.apply_summary.attempted as u32,
        writes_skipped: report.apply_summary.skipped as u32,
        writes_succeeded: report.apply_summary.succeeded as u32,
        writes_failed: report.apply_summary.failed as u32,
    }
}

pub(super) fn immediate_apply_error_message(
    action: &str,
    summary: &ApplySummary,
) -> Option<String> {
    if summary.failed == 0 {
        return None;
    }

    let failures = summary
        .records
        .iter()
        .filter(|record| record.status == ApplyStatus::Failed)
        .map(|record| format!("{} ({})", record.logical_id, record.detail))
        .take(3)
        .collect::<Vec<_>>();

    let detail = if failures.is_empty() {
        format!("{} monitor(s)", summary.failed)
    } else {
        failures.join("; ")
    };

    Some(format!(
        "{action}, but immediate apply failed on {} monitor(s): {detail}",
        summary.failed
    ))
}

pub(super) fn skipped_apply_summary(monitors: usize, _reason: &str) -> ApplySummary {
    ApplySummary {
        attempted: 0,
        skipped: monitors,
        succeeded: 0,
        failed: 0,
        backoff_skips: 0,
        transient_failures: 0,
        persistent_failures: 0,
        records: Vec::new(),
    }
}

pub(super) fn create_runtime_dir(path: &Path) -> Result<(), RuntimeError> {
    #[cfg(target_os = "linux")]
    {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(RUNTIME_DIR_MODE);
        builder.create(path).map_err(|source| RuntimeError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(RUNTIME_DIR_MODE));
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        fs::create_dir_all(path).map_err(|source| RuntimeError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }
}

pub(super) fn is_transient_ipc_accept_error(error: &ipc::IpcError) -> bool {
    matches!(
        error,
        ipc::IpcError::Io { source, .. }
            if matches!(
                source.kind(),
                io::ErrorKind::Interrupted
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::TimedOut
            )
    )
}

// --- From logging.rs ---
pub(super) fn log_tick(report: &TickReport) {
    let degraded_monitors = report
        .apply_summary
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.status,
                ApplyStatus::Failed | ApplyStatus::SkippedBackoff
            )
        })
        .count();

    info!(
        tick_duration_ms = report.tick_duration.as_millis(),
        monitors_evaluated = report.monitors_evaluated,
        writes_attempted = report.apply_summary.attempted,
        writes_skipped = report.apply_summary.skipped,
        writes_succeeded = report.apply_summary.succeeded,
        failures = report.apply_summary.failed,
        transient_failures = report.apply_summary.transient_failures,
        persistent_failures = report.apply_summary.persistent_failures,
        backoff_skips = report.apply_summary.backoff_skips,
        degraded_monitors = degraded_monitors,
        solar_elevation_deg = format!("{:.2}", report.solar.elevation_deg),
        manual_override = report.manual_override_active,
        suspended = report.suspended,
        weather_modifier = report.weather_modifier_applied,
        "tick"
    );

    for record in &report.apply_summary.records {
        if record.status == ApplyStatus::Failed {
            log_apply_failure(record);
        }
    }
}

pub(super) fn log_tick_error(error: &RuntimeError) {
    error!(
        recoverable = true,
        error_kind = runtime_error_kind(error),
        error = %error,
        "tick_failed"
    );
}

fn log_apply_failure(record: &ApplyRecord) {
    let failure_kind = match record.failure_kind {
        Some(FailureKind::Transient) => "transient",
        Some(FailureKind::Persistent) => "persistent",
        None => "unknown",
    };
    let should_log = record
        .consecutive_failures
        .is_none_or(should_log_failure_count);

    if !should_log {
        return;
    }

    let backend = record.backend.map_or_else(
        || String::from("unknown"),
        |b| format!("{b:?}").to_ascii_lowercase(),
    );

    if let Some(until_epoch_s) = record.backoff_until_epoch_s {
        error!(
            logical_id = %record.logical_id,
            backend = %backend,
            failure_kind = failure_kind,
            requested_percent = record.requested_percent,
            applied_percent = record.applied_percent,
            attempts = record.attempts,
            consecutive_failures = record.consecutive_failures.unwrap_or(0),
            detail = %record.detail,
            backoff_until_epoch_s = until_epoch_s,
            "apply_failed"
        );
    } else {
        error!(
            logical_id = %record.logical_id,
            backend = %backend,
            failure_kind = failure_kind,
            requested_percent = record.requested_percent,
            applied_percent = record.applied_percent,
            attempts = record.attempts,
            consecutive_failures = record.consecutive_failures.unwrap_or(0),
            detail = %record.detail,
            "apply_failed"
        );
    }
}

fn should_log_failure_count(consecutive_failures: u32) -> bool {
    consecutive_failures <= 3 || consecutive_failures.is_power_of_two()
}

fn runtime_error_kind(error: &RuntimeError) -> &'static str {
    match error {
        RuntimeError::Config(_) => "config",
        RuntimeError::Ipc(_) => "ipc",
        RuntimeError::Paths(_) => "paths",
        RuntimeError::Policy(_) => "policy",
        RuntimeError::Solar(_) => "solar",
        RuntimeError::State(_) => "state",
        RuntimeError::Io { .. } => "io",
    }
}

// --- From bootstrap.rs ---
/// Initializes the daemon environment, configures runtime paths,
/// and prepares the initial runtime state.
impl DaemonRuntime {
    /// Bootstraps the daemon by loading configuration, establishing paths,
    /// and instantiating the runtime.
    pub fn bootstrap() -> Result<Self, RuntimeError> {
        let config = config::load()?;
        let state_path = paths::state_file()?;
        let socket_path = paths::runtime_socket_path()?;
        Self::bootstrap_with_paths(config, state_path, socket_path)
    }

    #[must_use]
    /// Generates a human-readable startup message summarizing the active
    /// configuration source, connected monitors, and runtime paths.
    pub fn startup_message(&self) -> String {
        let config_source = match self.config.source {
            ConfigSource::Defaults => "defaults",
            ConfigSource::FilePresent => "file",
        };

        format!(
            "config={config_source} monitors={} state_file={} socket={}",
            self.config.monitors.len(),
            self.state_path.display(),
            self.socket.display_target(),
        )
    }
    /// Performs the internal setup steps, including creating the runtime socket directory,
    /// loading the persistent state file, and normalizing monitor configurations.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn bootstrap_with_paths(
        config: ConfigReport,
        state_path: PathBuf,
        socket_path: PathBuf,
    ) -> Result<Self, RuntimeError> {
        for warning in &config.warnings {
            tracing::info!(warning = %warning, "config_compat_warning");
        }

        if let Some(state_dir) = state_path.parent() {
            create_runtime_dir(state_dir)?;
        }
        #[cfg(target_os = "linux")]
        if let Some(socket_dir) = socket_path.parent() {
            create_runtime_dir(socket_dir)?;
        }

        let state = RuntimeState::load_from_path(&state_path)?;

        // Force a physical hardware sync on daemon startup.
        // If we trust the persisted `last_applied_percent`, the first tick might
        // silently skip applying brightness due to hysteresis if the calculated
        // target matches the persisted value, even if the physical monitor was
        // manually changed while the daemon was offline.
        #[cfg(target_os = "linux")]
        let socket = ControlSocket { path: socket_path };
        #[cfg(target_os = "windows")]
        let socket = ControlSocket {
            endpoint: crate::paths::IpcEndpoint::NamedPipe(socket_path.display().to_string()),
        };
        let config = RuntimeConfig::from_report(config)?;
        let persisted_state_snapshot = state.normalized_for_persistence();
        let initial_weather = state.weather.clone();

        let weather_engine = weather::WeatherEngine::new(
            config.weather.clone(),
            config.location.clone(),
            initial_weather,
        );

        let mut runtime = Self {
            config,
            socket,
            state_path,
            state,
            weather_refresh: WeatherRefreshState::default(),
            persisted_state_snapshot,
            last_solar_elevation: None,
            fade_engine: FadeEngine::new(),
            weather_engine,
            last_capabilities: None,
            capability_refresh: CapabilityRefreshState::default(),
            wake_watch: WakeWatch::default(),
            display_events: None,
        };

        runtime.prepare_apply_resync(false);

        // Unit fixtures historically invoke ticks directly. Give those
        // fixtures an explicit safe observation; production bootstrap remains
        // fail-closed until the daemon loop publishes a real snapshot.
        #[cfg(test)]
        {
            if runtime.config.monitors.iter().any(|monitor| {
                monitor.backend == crate::backends::BackendKind::Backlight
                    && monitor.selector.sysfs_path.is_some()
            }) {
                runtime.last_capabilities = Some(CapabilitySnapshot {
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
            }
        }

        runtime.state.prune_to_configured_monitors(
            runtime
                .config
                .monitors
                .iter()
                .map(|monitor| monitor.logical_id.as_str()),
        );
        runtime.persist_state_if_changed()?;

        Ok(runtime)
    }
}

// --- From control.rs ---
impl DaemonRuntime {
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_ipc_request_with_runner<R: ProcessRunner + Sync>(
        &mut self,
        request: ipc::Request,
        runner: &R,
        now_utc: DateTime<Utc>,
    ) -> (ipc::ResponseEnvelope, IpcOutcome) {
        let now_epoch_s = now_utc.timestamp().max(0) as u64;

        match request {
            ipc::Request::Status => (
                ipc::ResponseEnvelope::status(self.status_response(now_epoch_s)),
                IpcOutcome::default(),
            ),
            ipc::Request::Suspend { minutes } => {
                if minutes == Some(0) {
                    return (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InvalidRequest,
                            "suspend minutes must be greater than zero",
                        ),
                        IpcOutcome::default(),
                    );
                }

                match self.suspend(now_epoch_s, minutes) {
                    Ok(message) => (ipc::ResponseEnvelope::ack(message), IpcOutcome::default()),
                    Err(error) => (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InternalError,
                            error.to_string(),
                        ),
                        IpcOutcome::default(),
                    ),
                }
            }
            ipc::Request::Resume => match self.resume() {
                Ok(()) => (
                    self.respond_after_resync(
                        "resume",
                        "resumed automatic apply and cleared manual overrides",
                        now_utc,
                        runner,
                        true,
                    ),
                    IpcOutcome {
                        tick_attempted: true,
                        config_reloaded: false,
                    },
                ),
                Err(error) => (
                    ipc::ResponseEnvelope::error(ipc::ErrorCode::InternalError, error.to_string()),
                    IpcOutcome::default(),
                ),
            },
            ipc::Request::SetOverride {
                monitor_id,
                percent,
                minutes,
            } => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                match self.set_override(now_epoch_s, monitor_id.as_deref(), percent, minutes) {
                    Ok(message) => (
                        self.respond_after_forced_apply("set_override", &message, now_utc, runner),
                        outcome,
                    ),
                    Err(error) => (ipc_error_response(&error), outcome),
                }
            }
            ipc::Request::ClearOverride { monitor_id, global } => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                match self.clear_override(monitor_id.as_deref(), global) {
                    Ok(message) => (
                        self.respond_after_forced_apply(
                            "clear_override",
                            &message,
                            now_utc,
                            runner,
                        ),
                        outcome,
                    ),
                    Err(error) => (ipc_error_response(&error), outcome),
                }
            }
            ipc::Request::ReloadConfig => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: true,
                };
                match self.reload_config() {
                    Ok(()) => {
                        let message =
                            format!("reloaded config from {}", self.config.path.display());
                        (
                            self.respond_after_forced_apply(
                                "reload_config",
                                &message,
                                now_utc,
                                runner,
                            ),
                            outcome,
                        )
                    }
                    Err(error) => (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InternalError,
                            error.to_string(),
                        ),
                        IpcOutcome::default(),
                    ),
                }
            }
            ipc::Request::RefreshWeather => {
                if !self.config.weather.enabled {
                    return (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InvalidRequest,
                            "weather is disabled in configuration",
                        ),
                        IpcOutcome::default(),
                    );
                }
                if self.weather_engine.request_refresh() {
                    (
                        ipc::ResponseEnvelope::ack("weather refresh requested"),
                        IpcOutcome::default(),
                    )
                } else {
                    (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InternalError,
                            "weather refresh could not be scheduled",
                        ),
                        IpcOutcome::default(),
                    )
                }
            }
            ipc::Request::Ping => (ipc::ResponseEnvelope::pong(), IpcOutcome::default()),
            ipc::Request::RunOnce { force } => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };

                match self.run_once_at_with_runner(now_utc, runner, force) {
                    Ok(report) => {
                        log_tick(&report);
                        (
                            ipc::ResponseEnvelope::run_once(
                                run_once_response(&report),
                                "completed one daemon tick",
                            ),
                            outcome,
                        )
                    }
                    Err(error) => (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InternalError,
                            error.to_string(),
                        ),
                        outcome,
                    ),
                }
            }
            ipc::Request::IdleDim => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                self.state.desktop_idle_dimmed = true;
                (
                    self.respond_after_forced_apply(
                        "idle_dim",
                        "desktop idle dimming active",
                        now_utc,
                        runner,
                    ),
                    outcome,
                )
            }
            ipc::Request::IdleWake => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                self.state.desktop_idle_dimmed = false;
                let _ = self.request_wake_reassert(WakeReason::WaylandIdleResume);
                (
                    ipc::ResponseEnvelope::ack("desktop idle wake queued for re-observation"),
                    outcome,
                )
            }
            ipc::Request::ExternalBrightnessChange => {
                tracing::info!(
                    "external_brightness_change_observed_without_identity; no blind reapply"
                );
                (
                    ipc::ResponseEnvelope::ack(
                        "external brightness change observed; current hardware value preserved",
                    ),
                    IpcOutcome::default(),
                )
            }
        }
    }

    pub(super) fn reload_config_with<F>(&mut self, loader: F) -> Result<(), RuntimeError>
    where
        F: FnOnce() -> Result<ConfigReport, ConfigError>,
    {
        let next = loader()?;
        self.apply_reloaded_config(next)?;
        Ok(())
    }

    fn respond_after_forced_apply<R: ProcessRunner + Sync>(
        &mut self,
        trigger: &'static str,
        success_message: &str,
        now_utc: DateTime<Utc>,
        runner: &R,
    ) -> ipc::ResponseEnvelope {
        self.run_once_at_with_runner(now_utc, runner, true)
            .map_or_else(
                |error| {
                    tracing::error!(trigger = %trigger, error = %error, "force_apply_failed");
                    ipc::ResponseEnvelope::error(
                        ipc::ErrorCode::InternalError,
                        format!("{success_message}, but immediate apply failed: {error}"),
                    )
                },
                |report| immediate_apply_response(success_message, &report),
            )
    }

    fn respond_after_resync<R: ProcessRunner + Sync>(
        &mut self,
        trigger: &'static str,
        success_message: &str,
        now_utc: DateTime<Utc>,
        runner: &R,
        clear_backoff: bool,
    ) -> ipc::ResponseEnvelope {
        self.run_resync_at_with_runner(now_utc, runner, clear_backoff)
            .map_or_else(
                |error| {
                    tracing::error!(trigger = %trigger, error = %error, "force_apply_failed");
                    ipc::ResponseEnvelope::error(
                        ipc::ErrorCode::InternalError,
                        format!("{success_message}, but immediate apply failed: {error}"),
                    )
                },
                |report| immediate_apply_response(success_message, &report),
            )
    }

    fn suspend(&mut self, now_epoch_s: u64, minutes: Option<u64>) -> Result<String, RuntimeError> {
        let previous = self.state.suspend_until_epoch_s;
        let previous_indefinite = self.state.suspend_indefinite;
        let message = if let Some(minutes) = minutes {
            let until_epoch_s = self.state.suspend_for_minutes(now_epoch_s, minutes);
            format!("suspended until epoch {until_epoch_s} ({minutes} minute(s))")
        } else {
            self.state.suspend_until_resume();
            String::from("suspended until resume")
        };
        if let Err(error) = self.persist_state_if_changed() {
            self.state.suspend_until_epoch_s = previous;
            self.state.suspend_indefinite = previous_indefinite;
            return Err(error);
        }
        Ok(message)
    }

    fn resume(&mut self) -> Result<(), RuntimeError> {
        let previous_suspend_until = self.state.suspend_until_epoch_s;
        let previous_suspend_indefinite = self.state.suspend_indefinite;
        let previous_override = self.state.manual_override.take();

        self.state.suspend_until_epoch_s = None;
        self.state.suspend_indefinite = false;

        if let Err(error) = self.persist_state_if_changed() {
            self.state.suspend_until_epoch_s = previous_suspend_until;
            self.state.suspend_indefinite = previous_suspend_indefinite;
            self.state.manual_override = previous_override;
            return Err(error);
        }
        Ok(())
    }

    fn set_override(
        &mut self,
        now_epoch_s: u64,
        monitor_id: Option<&str>,
        percent: u8,
        minutes: Option<u64>,
    ) -> Result<String, RuntimeError> {
        if percent > 100 {
            return Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                message: String::from("override percent must be in the range 0..=100"),
            }));
        }
        if minutes == Some(0) {
            return Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                message: String::from("override minutes must be greater than zero"),
            }));
        }

        let previous_override = self.state.manual_override.clone();
        let expires_at_epoch_s =
            minutes.map(|minutes| now_epoch_s.saturating_add(minutes.saturating_mul(60)));

        let message = if let Some(logical_id) = monitor_id {
            self.ensure_configured_monitor(logical_id)?;
            self.state
                .set_monitor_override(logical_id, percent, expires_at_epoch_s);
            match expires_at_epoch_s {
                Some(until_epoch_s) => format!(
                    "set manual override for {logical_id} to {}% until epoch {until_epoch_s}",
                    percent.min(100)
                ),
                None => format!(
                    "set manual override for {logical_id} to {}%",
                    percent.min(100)
                ),
            }
        } else {
            self.state.set_global_override(percent, expires_at_epoch_s);
            match expires_at_epoch_s {
                Some(until_epoch_s) => format!(
                    "set global manual override to {}% until epoch {until_epoch_s}",
                    percent.min(100)
                ),
                None => format!("set global manual override to {}%", percent.min(100)),
            }
        };

        if let Err(error) = self.persist_state_if_changed() {
            self.state.manual_override = previous_override;
            return Err(error);
        }

        Ok(message)
    }

    fn clear_override(
        &mut self,
        monitor_id: Option<&str>,
        global: bool,
    ) -> Result<String, RuntimeError> {
        if monitor_id.is_some() && global {
            return Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                message: String::from("clear-override accepts either --monitor-id or --global"),
            }));
        }

        let previous_override = self.state.manual_override.clone();
        let changed = if let Some(logical_id) = monitor_id {
            self.ensure_configured_monitor(logical_id)?;
            self.state.clear_monitor_override(logical_id)
        } else if global {
            self.state.clear_global_override()
        } else {
            self.state.clear_override()
        };

        let message = if let Some(logical_id) = monitor_id {
            if changed {
                format!("cleared manual override for {logical_id}")
            } else {
                format!("no manual override was active for {logical_id}")
            }
        } else if global {
            if changed {
                String::from("cleared global manual override")
            } else {
                String::from("no global manual override was active")
            }
        } else if changed {
            String::from("cleared all manual overrides")
        } else {
            String::from("no manual overrides were active")
        };

        if let Err(error) = self.persist_state_if_changed() {
            self.state.manual_override = previous_override;
            return Err(error);
        }

        Ok(message)
    }

    fn ensure_configured_monitor(&self, logical_id: &str) -> Result<(), RuntimeError> {
        if self
            .config
            .monitors
            .iter()
            .any(|monitor| monitor.logical_id == logical_id)
        {
            Ok(())
        } else {
            Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                message: format!("unknown monitor logical_id `{logical_id}`"),
            }))
        }
    }

    fn reload_config(&mut self) -> Result<(), RuntimeError> {
        self.reload_config_with(config::load)
    }

    fn apply_reloaded_config(&mut self, next: ConfigReport) -> Result<(), RuntimeError> {
        let new_config = RuntimeConfig::from_report(next)?;
        let location_changed = self.config.location != new_config.location;

        let previous_config = std::mem::replace(&mut self.config, new_config);
        let previous_monitors = self.state.monitors.clone();
        let previous_weather = self.state.weather.clone();
        let previous_weather_refresh = std::mem::take(&mut self.weather_refresh);

        let mut state_changed = self.state.prune_to_configured_monitors(
            self.config
                .monitors
                .iter()
                .map(|monitor| monitor.logical_id.as_str()),
        );

        if location_changed && self.state.weather.is_some() {
            self.state.weather = None;
            state_changed = true;
        }

        if state_changed {
            if let Err(error) = self.persist_state_if_changed() {
                self.config = previous_config;
                self.state.monitors = previous_monitors;
                self.state.weather = previous_weather;
                self.weather_refresh = previous_weather_refresh;
                return Err(error);
            }
        }

        self.weather_engine
            .sync_config(&self.config.weather, &self.config.location);

        Ok(())
    }
}

fn immediate_apply_response(success_message: &str, report: &TickReport) -> ipc::ResponseEnvelope {
    log_tick(report);

    if let Some(message) = immediate_apply_error_message(success_message, &report.apply_summary) {
        ipc::ResponseEnvelope::error(ipc::ErrorCode::InternalError, message)
    } else {
        ipc::ResponseEnvelope::ack(success_message)
    }
}

// --- From loop.rs ---
/// Implements the main non-blocking execution loop for the daemon.
impl DaemonRuntime {
    /// Starts the main daemon event loop.
    ///
    /// This function handles IPC requests, idle state transitions, and
    /// applies hardware brightness targets asynchronously until
    /// `shutdown_requested` returns true.
    #[allow(clippy::too_many_lines)]
    pub fn run_loop<F>(&mut self, mut shutdown_requested: F) -> Result<(), RuntimeError>
    where
        F: FnMut() -> bool,
    {
        let listener = self.socket.bind_listener()?;
        let desktop_idle_sync = self.config.daemon.desktop_idle_sync;
        let mut desktop_idle = DesktopIdleSync::new(
            desktop_idle_sync,
            PathBuf::from(self.socket.display_target()),
            self.config.daemon.tick_seconds,
            self.config.daemon.desktop_idle_timeout_minutes,
        );
        let mut cadence = LoopCadence::new(self.config.daemon.tick_seconds);
        self.display_events = Some(DisplayEventSources::spawn());

        tracing::info!(
            mode = "daemon",
            tick_seconds = %self.config.daemon.tick_seconds,
            desktop_idle_sync = %desktop_idle_sync,
            startup = %self.startup_message(),
            "startup"
        );

        if self.config.location.latitude == 0.0
            && self.config.location.longitude == 0.0
            && self.config.location.timezone_name == "UTC"
        {
            tracing::warn!(
                "using default equatorial location (0.0, 0.0, UTC) — \
                 set your coordinates in config.toml or via the TUI for accurate brightness"
            );
        }

        while !shutdown_requested() {
            let _ = self.drain_ipc_requests_with_quantum_impl(
                &listener,
                &mut cadence,
                &mut desktop_idle,
            )?;

            if shutdown_requested() {
                break;
            }

            let now_utc = Utc::now();
            let pending_event = self
                .display_events
                .as_ref()
                .and_then(DisplayEventSources::drain);
            if let Some(request) = pending_event {
                tracing::info!(reason = ?request.reason, "display_recovery_requested");
                self.handle_display_event(request.reason);
            }
            #[cfg(target_os = "linux")]
            desktop_idle.maintain_watcher(&self.socket.path);
            #[cfg(not(target_os = "linux"))]
            desktop_idle.maintain_watcher(Path::new(""));
            if desktop_idle.perform_due_action(self, now_utc, &RealProcessRunner, &mut cadence) {
                continue;
            }

            if DesktopIdleSync::take_input_resume() {
                let _ = self.request_wake_reassert(WakeReason::WaylandIdleResume);
            }
            if self.wake_watch.take_due_probe(Instant::now()) {
                self.run_wake_probe(now_utc, &RealProcessRunner);
                continue;
            }

            let next_deadline = [
                desktop_idle.next_deadline(),
                self.wake_watch.next_deadline(),
            ]
            .into_iter()
            .flatten()
            .min();
            let action = cadence.next_action(next_deadline, self.fade_engine.is_fading());
            // Phase 9 uses periodic observation only. Refresh immediately
            // before scheduled ticks, not on every idle loop iteration.
            if let Some(result) = self.publish_completed_capability_snapshot() {
                if self.capability_refresh.take_pending() {
                    if result {
                        self.execute_scheduled_tick(
                            now_utc,
                            &RealProcessRunner,
                            &mut cadence,
                            &mut desktop_idle,
                        );
                    }
                    continue;
                }
            }
            if self.capability_refresh.is_pending() {
                wait_for_ipc_or_sleep(&listener, Duration::from_millis(16));
                continue;
            }

            if matches!(action, LoopAction::RunTick) {
                self.capability_refresh.set_pending();
                self.begin_capability_refresh_with_runner(RealProcessRunner);
                wait_for_ipc_or_sleep(&listener, Duration::from_millis(16));
                continue;
            }

            self.perform_loop_action(
                action,
                now_utc,
                &RealProcessRunner,
                &mut cadence,
                &mut desktop_idle,
                &listener,
            );
        }

        let _ = self.persist_state_if_changed();
        drop(listener);

        tracing::info!(mode = "daemon", reason = "signal", "shutdown");
        Ok(())
    }

    pub(crate) fn handle_ipc_stream(&mut self, mut stream: NativeIpcStream) -> IpcOutcome {
        let target = self.socket.display_target();
        let envelope = match ipc::read_request(&mut stream, &target) {
            Ok(envelope) => envelope,
            Err(error) => {
                if let ipc::IpcError::Io { source, .. } = &error {
                    if source.kind() == std::io::ErrorKind::WouldBlock {
                        tracing::info!(reason = "would_block", "ipc_read_aborted");
                        return IpcOutcome::default();
                    }
                }
                tracing::error!(error = %error, "ipc_read_failed");
                let _ = ipc::write_response(
                    &mut stream,
                    &ipc::ResponseEnvelope::error(
                        ipc::ErrorCode::InvalidRequest,
                        error.to_string(),
                    ),
                    &target,
                );
                return IpcOutcome::default();
            }
        };

        let request = match envelope.validate() {
            Ok(request) => request,
            Err(response) => {
                if let Err(error) = ipc::write_response(&mut stream, response.as_ref(), &target) {
                    tracing::error!(error = %error, "ipc_write_failed");
                }
                return IpcOutcome::default();
            }
        };

        let request_name = request.name().to_owned();
        let (response, outcome) =
            self.handle_ipc_request_with_runner(request, &RealProcessRunner, Utc::now());
        let response_kind = response.kind_name().to_owned();
        let response_error = match &response.response {
            ipc::Response::Error { message, .. } => Some(message.clone()),
            _ => None,
        };

        if let Err(error) = ipc::write_response(&mut stream, &response, &target) {
            tracing::error!(
                request = %request_name,
                response = %response_kind,
                error = %error,
                "ipc_write_failed"
            );
            return outcome;
        }

        if let Some(error) = response_error {
            tracing::error!(
                request = %request_name,
                response = %response_kind,
                error = %error,
                "ipc_request_failed"
            );
        } else if response_kind != "status" && response_kind != "pong" {
            tracing::info!(
                request = %request_name,
                response = %response_kind,
                "ipc_request"
            );
        }

        outcome
    }
}

// --- From tests.rs ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use chrono::{TimeZone, Utc};

    use crate::backends::{testutil::FakeRunner, BackendKind, FailureKind};
    use crate::config::{
        Config, ConfigError, ConfigReport, ConfigSource, DaemonConfig, LocationConfig, LogLevel,
        MonitorConfig, MonitorSelector, SolarPolicyConfig, WeatherConfig,
    };
    use crate::ipc::{Request, Response};
    use crate::process::{CommandError, CommandOutput, ProcessRunner};
    use crate::state::{
        FailureBackoffState, ManualOverrideState, RuntimeState, WeatherSnapshotMetadata,
    };

    struct RecordingRunner {
        calls: std::sync::Mutex<Vec<String>>,
    }

    #[derive(Clone)]
    struct SlowObservationRunner {
        delay: Duration,
        started: Arc<AtomicBool>,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ProcessRunner for SlowObservationRunner {
        fn run(
            &self,
            program: &str,
            _args: &[String],
            _timeout: Duration,
        ) -> Result<CommandOutput, CommandError> {
            self.started.store(true, Ordering::Release);
            self.calls.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(self.delay);
            Err(CommandError::Missing {
                program: program.to_owned(),
            })
        }
    }

    struct PanicObservationRunner;

    impl ProcessRunner for PanicObservationRunner {
        fn run(
            &self,
            _program: &str,
            _args: &[String],
            _timeout: Duration,
        ) -> Result<CommandOutput, CommandError> {
            panic!("synthetic observation panic");
        }
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
    fn bootstrap_creates_runtime_dirs_and_loads_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");

        let runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path.clone(),
        )
        .expect("runtime should bootstrap");

        assert!(state_path.parent().expect("state dir").exists());
        assert!(socket_path.parent().expect("socket dir").exists());
        assert_eq!(runtime.state, RuntimeState::default());
    }

    #[test]
    fn slow_capability_refresh_does_not_block_ipc_or_publish_partial_snapshot() {
        let temp = TempDir::new();
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            temp.path().join("state/runtime-state.json"),
            temp.path().join("run/control.sock"),
        )
        .expect("runtime should bootstrap");
        runtime.last_capabilities = None;
        let runner = SlowObservationRunner {
            delay: Duration::from_millis(150),
            started: Arc::new(AtomicBool::new(false)),
            calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        let calls = Arc::clone(&runner.calls);
        let started = Arc::clone(&runner.started);

        runtime.begin_capability_refresh_with_runner(runner.clone());
        let started_deadline = Instant::now() + Duration::from_millis(100);
        while !started.load(Ordering::Acquire) && Instant::now() < started_deadline {
            std::thread::yield_now();
        }
        runtime.begin_capability_refresh_with_runner(runner);
        assert!(started.load(Ordering::Acquire));
        assert!(runtime.last_capabilities.is_none());
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        let status_started = Instant::now();
        let (response, outcome) = runtime.handle_ipc_request_with_runner(
            ipc::Request::Status,
            &RecordingRunner::new(),
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
        );
        assert!(matches!(response.response, ipc::Response::Status { .. }));
        assert!(!outcome.tick_attempted);
        assert!(
            status_started.elapsed() < Duration::from_millis(50),
            "IPC status handling must not wait for observation"
        );

        std::thread::sleep(Duration::from_millis(600));
        runtime.publish_completed_capability_snapshot();
        assert!(runtime.last_capabilities.is_some());
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn panicking_observation_clears_active_state_and_allows_retry() {
        let temp = TempDir::new();
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            temp.path().join("state/runtime-state.json"),
            temp.path().join("run/control.sock"),
        )
        .expect("runtime should bootstrap");

        runtime.begin_capability_refresh_with_runner(PanicObservationRunner);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(runtime.publish_completed_capability_snapshot(), Some(false));

        let runner = SlowObservationRunner {
            delay: Duration::from_millis(1),
            started: Arc::new(AtomicBool::new(false)),
            calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        let started = Arc::clone(&runner.started);
        runtime.begin_capability_refresh_with_runner(runner);
        let deadline = Instant::now() + Duration::from_millis(100);
        while !started.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(started.load(Ordering::Acquire));
    }

    #[test]
    fn bootstrap_rejects_future_state_without_replacing_it() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let original = br#"{"schema_version":2,"future":{"new":true},"monitors":{}}
"#;
        fs::create_dir_all(state_path.parent().expect("state dir"))
            .expect("state dir should exist");
        fs::write(&state_path, original).expect("future state should write");

        let error = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect_err("future state must prevent startup");

        assert!(matches!(
            error,
            RuntimeError::State(StateError::UnsupportedSchemaVersion {
                found: 2,
                supported: 1,
                ..
            })
        ));
        assert_eq!(
            fs::read(&state_path).expect("state should remain"),
            original
        );
        assert_eq!(
            fs::read_dir(temp.path().join("state"))
                .expect("state dir should be readable")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
                .count(),
            0
        );
    }

    #[test]
    fn bootstrap_prunes_stale_runtime_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut stale_state = RuntimeState::default();
        stale_state.record_apply_success("stale", 70, 1_800_000_000);
        stale_state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([(String::from("stale"), 55)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        stale_state
            .save_to_path(&state_path)
            .expect("stale state should save");

        let runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");

        assert!(runtime.state.monitor("stale").is_none());
        assert_eq!(runtime.state.manual_override, None);
        let persisted = RuntimeState::load_from_path(&state_path).expect("state file should load");
        assert!(persisted.monitor("stale").is_none());
        assert_eq!(persisted.manual_override, None);
    }

    #[test]
    fn no_op_tick_does_not_persist_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_dry_run_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &FakeRunner::new(),
                false,
            )
            .expect("tick should succeed");

        assert_eq!(report.apply_summary.attempted, 0);
        assert_eq!(report.apply_summary.skipped, 1);
        assert!(!state_path.exists());
        assert!(!runtime.persist_state_if_changed().expect("persist check"));
    }

    #[test]
    fn suspend_mutation_persists_state_once() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");

        runtime.state.suspend_until_epoch_s = Some(1_800_000_900);

        assert!(runtime
            .persist_state_if_changed()
            .expect("persist should work"));
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state should load")
                .suspend_until_epoch_s,
            Some(1_800_000_900)
        );
        assert!(!runtime.persist_state_if_changed().expect("repeat check"));
    }

    #[test]
    fn override_mutation_persists_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(40),
            global_expires_at_epoch_s: Some(1_800_000_600),
            targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
            expires_at_epoch_s: Some(1_800_000_600),
        });

        assert!(runtime
            .persist_state_if_changed()
            .expect("persist should work"));
        let persisted = RuntimeState::load_from_path(&state_path).expect("state should load");
        let manual_override = persisted
            .manual_override
            .expect("manual override should persist");
        assert_eq!(manual_override.global_percent(1_800_000_000), Some(40));
        assert_eq!(
            manual_override.target_percent("internal", 1_800_000_000),
            Some(27)
        );
    }

    #[test]
    fn weather_mutation_persists_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_weather_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.weather = Some(WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            fetched_at_epoch_s: 1_800_000_000,
            valid_at_epoch_s: 1_700_000_000,
            source_kind: crate::weather::WeatherSourceKind::Forecast,
            cloud_cover_percent: Some(81),
            smoothed_cloud_cover_percent: Some(77),
            temperature: Some(0.0),
            condition: crate::weather::WeatherCondition::Cloudy,
            condition_description: None,
            day_phase: None,
            forecast: vec![],
            ..Default::default()
        });

        assert!(runtime
            .persist_state_if_changed()
            .expect("persist should work"));
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state should load")
                .weather,
            runtime.state.weather
        );
    }

    #[test]
    fn persisted_state_writes_leave_no_temporary_files_behind() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.suspend_until_epoch_s = Some(1_800_000_900);

        assert!(runtime
            .persist_state_if_changed()
            .expect("persist should work"));
        assert_eq!(count_temp_state_files(&state_path), 0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn run_once_applies_manual_override_and_persists_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([(String::from("internal"), 33)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        runtime.last_capabilities = Some(test_capability_snapshot());
        let runner = FakeRunner::new().with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "33%",
            ],
            "",
        );

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &runner,
                false,
            )
            .expect("tick should succeed");

        assert!(report.manual_override_active);
        assert_eq!(report.apply_summary.succeeded, 1);
        assert_eq!(
            runtime
                .state
                .monitor("internal")
                .and_then(|monitor| monitor.last_applied_percent),
            Some(33)
        );

        let persisted = RuntimeState::load_from_path(&state_path).expect("state should persist");
        assert_eq!(
            persisted
                .monitor("internal")
                .and_then(|monitor| monitor.last_applied_percent),
            Some(33)
        );
    }

    #[test]
    fn run_once_without_capability_snapshot_fails_closed() {
        let temp = TempDir::new();
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            temp.path().join("state/runtime-state.json"),
            temp.path().join("run/control.sock"),
        )
        .expect("runtime should bootstrap");
        runtime.last_capabilities = None;
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(33),
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::new(),
            expires_at_epoch_s: None,
        });
        let runner = RecordingRunner::new();

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &runner,
                true,
            )
            .expect("tick should complete with a skipped apply");

        assert!(runtime.last_capabilities.is_none());
        assert_eq!(report.apply_summary.attempted, 0);
        assert_eq!(report.apply_summary.skipped, 1);
        assert!(runner.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn run_once_skips_apply_while_suspended_even_with_manual_override() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime =
            DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
                .expect("runtime should bootstrap");
        runtime.state.suspend_until_epoch_s = Some(1_800_000_100);
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(55),
            global_expires_at_epoch_s: Some(1_800_000_100),
            targets: std::collections::BTreeMap::from([(String::from("internal"), 33)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        let runner = FakeRunner::new();

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &runner,
                false,
            )
            .expect("tick should succeed");

        assert!(report.suspended);
        assert!(report.manual_override_active);
        assert_eq!(report.apply_summary.attempted, 0);
        assert_eq!(report.apply_summary.skipped, 1);
        assert!(runner.calls().is_empty());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn run_once_applies_global_override() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime =
            DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
                .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(44),
            global_expires_at_epoch_s: Some(1_800_000_100),
            targets: std::collections::BTreeMap::new(),
            expires_at_epoch_s: None,
        });
        runtime.last_capabilities = Some(test_capability_snapshot());
        let runner = FakeRunner::new().with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "44%",
            ],
            "",
        );

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &runner,
                false,
            )
            .expect("tick should succeed");

        assert!(report.manual_override_active);
        assert_eq!(report.apply_summary.succeeded, 1);
        assert_eq!(
            runtime
                .state
                .monitor("internal")
                .and_then(|monitor| monitor.last_applied_percent),
            Some(44)
        );
    }

    #[test]
    fn run_once_uses_persisted_weather_metadata_as_bounded_modifier() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_weather_config_report(),
            state_path,
            socket_path,
        )
        .expect("runtime should bootstrap");
        let weather_snapshot = crate::state::WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            fetched_at_epoch_s: 1_800_000_000,
            valid_at_epoch_s: 1_700_000_000,
            source_kind: crate::weather::WeatherSourceKind::Forecast,
            cloud_cover_percent: Some(100),
            smoothed_cloud_cover_percent: Some(100),
            temperature: Some(0.0),
            condition: crate::weather::WeatherCondition::Cloudy,
            condition_description: None,
            day_phase: None,
            forecast: vec![],
            ..Default::default()
        };
        runtime.state.weather = Some(weather_snapshot);
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([(String::from("internal"), 25)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        let runner = FakeRunner::new().with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "25%",
            ],
            "",
        );

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_030, 0)
                    .single()
                    .expect("valid time"),
                &runner,
                false,
            )
            .expect("tick should succeed");

        assert!(report.weather_modifier_applied);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn apply_failure_backoff_persists_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([(String::from("internal"), 33)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        let runner = FakeRunner::new().with_output(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "33%",
            ],
            Some(1),
            "",
            "temporarily unavailable",
        );

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &runner,
                false,
            )
            .expect("tick should succeed");

        assert_eq!(report.apply_summary.failed, 1);
        let persisted = RuntimeState::load_from_path(&state_path).expect("state should persist");
        let backoff = persisted
            .monitor("internal")
            .and_then(|monitor| monitor.backoff.as_ref())
            .expect("backoff state should persist");
        assert_eq!(backoff.backend, BackendKind::Backlight);
        assert_eq!(backoff.failure_kind, FailureKind::Transient);
    }

    #[test]
    fn ipc_status_reports_config_and_last_applied_values() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_weather_config_report(),
            state_path,
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.suspend_until_epoch_s = Some(1_800_000_100);
        runtime
            .state
            .record_apply_success("internal", 41, 1_800_000_000);
        runtime.state.weather = Some(crate::state::WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            fetched_at_epoch_s: 1_800_000_000,
            valid_at_epoch_s: 1_700_000_000,
            source_kind: crate::weather::WeatherSourceKind::Forecast,
            cloud_cover_percent: Some(50),
            smoothed_cloud_cover_percent: Some(50),
            temperature: Some(0.0),
            condition: crate::weather::WeatherCondition::PartlyCloudy,
            condition_description: Some(String::from("broken clouds")),
            day_phase: Some(crate::weather::WeatherDayPhase::Day),
            forecast: vec![],
            ..Default::default()
        });
        runtime.weather_refresh.last_attempted_at_epoch_s = Some(1_800_000_020);
        runtime.weather_refresh.next_refresh_at_epoch_s = Some(1_800_001_800);
        runtime.weather_refresh.consecutive_failures = 2;
        runtime.weather_refresh.last_error =
            Some(String::from("openweather request failed: network timeout"));

        let (response, outcome) = runtime.handle_ipc_request_with_runner(
            Request::Status,
            &FakeRunner::new(),
            Utc.timestamp_opt(1_800_000_030, 0)
                .single()
                .expect("valid time"),
        );

        assert!(!outcome.tick_attempted);

        let Response::Status { status } = response.response else {
            panic!("status request should return a status response");
        };

        assert!(status.daemon_alive);
        assert_eq!(status.tick_seconds, 60);
        assert_eq!(status.configured_monitors, 1);
        assert_eq!(status.stateful_monitors, 1);
        assert!(status.suspended);
        assert!(!status.manual_override_active);
        assert_eq!(status.monitors[0].last_applied_percent, Some(41));
        let weather = status.weather.expect("weather status should exist");
        assert!(weather.active);
        assert!(!weather.stale);
        assert_eq!(weather.last_refresh_attempt_epoch_s, Some(1_800_000_020));
        assert_eq!(weather.next_refresh_at_epoch_s, Some(1_800_001_800));
        assert_eq!(weather.consecutive_failures, 2);
        assert_eq!(
            weather.condition,
            crate::weather::WeatherCondition::PartlyCloudy
        );
        assert_eq!(
            weather.condition_description.as_deref(),
            Some("broken clouds")
        );
        assert_eq!(
            weather.day_phase,
            Some(crate::weather::WeatherDayPhase::Day)
        );
        assert_eq!(
            weather.last_error.as_deref(),
            Some("openweather request failed: network timeout")
        );
    }

    #[test]
    fn ipc_status_reports_topology_unavailability_and_ambiguity_in_configured_order() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut config = test_config_report();
        let mut external = config.config.monitors[0].clone();
        external.logical_id = String::from("external");
        external.backend = BackendKind::Ddc;
        external.selector = MonitorSelector {
            connector: None,
            serial: Some(String::from("external-serial")),
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: None,
            ddc_address: None,
        };
        config.config.monitors.push(external);

        let mut runtime = DaemonRuntime::bootstrap_with_paths(config, state_path, socket_path)
            .expect("runtime should bootstrap");
        runtime.last_capabilities = Some(CapabilitySnapshot {
            ddc_present: vec![
                crate::discovery::DdcMonitorDiscovery {
                    occurrence_id: String::from("first"),
                    stable_id: String::from("ddc:first"),
                    identity_status: crate::discovery::PhysicalIdentityStatus::Unique,
                    manufacturer: None,
                    model: None,
                    serial: Some(String::from("external-serial")),
                    display_number: 1,
                    bus_number: Some(7),
                    connector: None,
                    brightness_vcp_supported: Some(true),
                    backend_viable: true,
                    note: None,
                },
                crate::discovery::DdcMonitorDiscovery {
                    occurrence_id: String::from("second"),
                    stable_id: String::from("ddc:second"),
                    identity_status: crate::discovery::PhysicalIdentityStatus::Unique,
                    manufacturer: None,
                    model: None,
                    serial: Some(String::from("external-serial")),
                    display_number: 2,
                    bus_number: Some(8),
                    connector: None,
                    brightness_vcp_supported: Some(true),
                    backend_viable: true,
                    note: None,
                },
            ],
            backlights_present: Vec::new(),
        });

        let (response, outcome) = runtime.handle_ipc_request_with_runner(
            Request::Status,
            &FakeRunner::new(),
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
        );

        assert!(!outcome.tick_attempted);
        let Response::Status { status } = response.response else {
            panic!("status request should return a status response");
        };
        assert_eq!(status.monitors.len(), 2);
        assert_eq!(
            status.monitors[0].topology.as_deref(),
            Some("temporarily_unavailable")
        );
        assert_eq!(
            status.monitors[1].topology.as_deref(),
            Some("ambiguous_or_unsafe")
        );
    }

    #[test]
    fn ipc_status_hides_expired_manual_overrides() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime =
            DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
                .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(33),
            global_expires_at_epoch_s: Some(1_800_000_000),
            targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
            expires_at_epoch_s: Some(1_800_000_000),
        });

        let (response, _) = runtime.handle_ipc_request_with_runner(
            Request::Status,
            &FakeRunner::new(),
            Utc.timestamp_opt(1_800_000_001, 0)
                .single()
                .expect("valid time"),
        );

        let Response::Status { status } = response.response else {
            panic!("status request should return a status response");
        };

        assert!(!status.manual_override_active);
        assert_eq!(status.global_override_percent, None);
        assert_eq!(status.global_override_until_epoch_s, None);
        assert_eq!(status.per_monitor_override_until_epoch_s, None);
        assert_eq!(status.monitors[0].override_percent, None);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn ipc_suspend_and_resume_persist_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        let now = Utc
            .timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(40),
            global_expires_at_epoch_s: Some(1_800_000_100),
            targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });

        let (suspend_response, suspend_outcome) = runtime.handle_ipc_request_with_runner(
            Request::Suspend { minutes: Some(15) },
            &FakeRunner::new(),
            now,
        );
        assert!(!suspend_outcome.tick_attempted);
        assert!(matches!(suspend_response.response, Response::Ack { .. }));
        assert_eq!(runtime.state.suspend_until_epoch_s, Some(1_800_000_900));
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .suspend_until_epoch_s,
            Some(1_800_000_900)
        );

        let runner = RecordingRunner::new();
        let (resume_response, resume_outcome) =
            runtime.handle_ipc_request_with_runner(Request::Resume, &runner, now);
        assert!(resume_outcome.tick_attempted);
        assert!(matches!(resume_response.response, Response::Ack { .. }));
        assert_eq!(runtime.state.suspend_until_epoch_s, None);
        assert_eq!(runtime.state.manual_override, None);
        assert_eq!(runner.calls().len(), 1);
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .suspend_until_epoch_s,
            None
        );
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .manual_override,
            None
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn ipc_indefinite_suspend_persists_state_until_resume() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        let now = Utc
            .timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time");

        let (suspend_response, suspend_outcome) = runtime.handle_ipc_request_with_runner(
            Request::Suspend { minutes: None },
            &FakeRunner::new(),
            now,
        );
        assert!(!suspend_outcome.tick_attempted);
        assert!(matches!(suspend_response.response, Response::Ack { .. }));
        assert!(runtime.state.suspend_indefinite);
        assert_eq!(runtime.state.suspend_until_epoch_s, None);
        assert!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .suspend_indefinite
        );

        let runner = RecordingRunner::new();
        let (resume_response, resume_outcome) =
            runtime.handle_ipc_request_with_runner(Request::Resume, &runner, now);
        assert!(resume_outcome.tick_attempted);
        assert!(matches!(resume_response.response, Response::Ack { .. }));
        assert!(!runtime.state.suspend_indefinite);
        assert_eq!(runtime.state.suspend_until_epoch_s, None);
        assert_eq!(runner.calls().len(), 1);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn ipc_resume_forces_reapply_and_clears_monitor_backoff() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        let now = Utc
            .timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time");
        runtime.state.suspend_until_epoch_s = Some(1_800_000_900);
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: Some(40),
            global_expires_at_epoch_s: Some(1_800_000_100),
            targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        runtime.state.monitor_mut("internal").last_applied_percent = Some(90);
        runtime
            .state
            .monitor_mut("internal")
            .last_applied_at_epoch_s = Some(1_799_999_940);
        runtime.state.monitor_mut("internal").backoff = Some(FailureBackoffState {
            backend: BackendKind::Backlight,
            failure_kind: FailureKind::Transient,
            consecutive_failures: 4,
            suppress_until_epoch_s: Some(1_800_000_500),
        });

        let runner = RecordingRunner::new();
        let (response, outcome) =
            runtime.handle_ipc_request_with_runner(Request::Resume, &runner, now);

        assert!(outcome.tick_attempted);
        assert!(matches!(response.response, Response::Ack { .. }));
        assert_eq!(runner.calls().len(), 1);
        assert!(runner.calls()[0]
            .starts_with("brightnessctl|--quiet|--class|backlight|--device|intel_backlight|set|"));
        assert_eq!(runtime.state.suspend_until_epoch_s, None);
        assert_eq!(runtime.state.manual_override, None);
        assert_eq!(
            runtime
                .state
                .monitor("internal")
                .and_then(|monitor| monitor.backoff.as_ref()),
            None
        );
        assert_eq!(
            runtime
                .state
                .monitor("internal")
                .and_then(|monitor| monitor.last_applied_at_epoch_s),
            Some(1_800_000_000)
        );
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .monitor("internal")
                .and_then(|monitor| monitor.backoff.as_ref()),
            None
        );
    }

    #[test]
    fn ipc_set_and_clear_override_persist_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_dry_run_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        let now = Utc
            .timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time");

        let (set_response, set_outcome) = runtime.handle_ipc_request_with_runner(
            Request::SetOverride {
                monitor_id: Some(String::from("internal")),
                percent: 36,
                minutes: Some(10),
            },
            &FakeRunner::new(),
            now,
        );
        assert!(set_outcome.tick_attempted);
        assert!(matches!(set_response.response, Response::Ack { .. }));
        assert_eq!(
            runtime.state.manual_override.as_ref().and_then(
                |manual_override| manual_override.target_percent("internal", 1_800_000_000)
            ),
            Some(36)
        );
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .manual_override
                .as_ref()
                .and_then(
                    |manual_override| manual_override.target_percent("internal", 1_800_000_000)
                ),
            Some(36)
        );

        let (clear_response, clear_outcome) = runtime.handle_ipc_request_with_runner(
            Request::ClearOverride {
                monitor_id: Some(String::from("internal")),
                global: false,
            },
            &FakeRunner::new(),
            now,
        );
        assert!(clear_outcome.tick_attempted);
        assert!(matches!(clear_response.response, Response::Ack { .. }));
        assert_eq!(runtime.state.manual_override, None);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn ipc_set_override_reports_immediate_apply_failures_without_losing_state() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        let runner = FakeRunner::new().with_output(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "36%",
            ],
            Some(1),
            "",
            "permission denied",
        );

        let (response, outcome) = runtime.handle_ipc_request_with_runner(
            Request::SetOverride {
                monitor_id: Some(String::from("internal")),
                percent: 36,
                minutes: Some(10),
            },
            &runner,
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
        );

        assert!(outcome.tick_attempted);

        let Response::Error { message, .. } = response.response else {
            panic!("set_override should surface immediate apply failures");
        };
        assert!(message.contains("set manual override for internal to 36%"));
        assert!(message.contains("immediate apply failed"));
        assert_eq!(
            runtime.state.manual_override.as_ref().and_then(
                |manual_override| manual_override.target_percent("internal", 1_800_000_000)
            ),
            Some(36)
        );
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .manual_override
                .as_ref()
                .and_then(
                    |manual_override| manual_override.target_percent("internal", 1_800_000_000)
                ),
            Some(36)
        );
    }

    #[test]
    fn run_once_expires_manual_override_before_apply() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_dry_run_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([(String::from("internal"), 29)]),
            expires_at_epoch_s: Some(1_799_999_999),
        });

        let report = runtime
            .run_once_at_with_runner(
                Utc.timestamp_opt(1_800_000_000, 0)
                    .single()
                    .expect("valid time"),
                &FakeRunner::new(),
                false,
            )
            .expect("tick should succeed");

        assert!(!report.manual_override_active);
        assert_eq!(runtime.state.manual_override, None);
        assert_eq!(
            RuntimeState::load_from_path(&state_path)
                .expect("state file should load")
                .manual_override,
            None
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn ipc_run_once_executes_tick_and_returns_summary() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime =
            DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
                .expect("runtime should bootstrap");
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        runtime.state.monitor_mut("internal").backoff = Some(FailureBackoffState {
            backend: BackendKind::Backlight,
            failure_kind: FailureKind::Transient,
            consecutive_failures: 3,
            suppress_until_epoch_s: Some(1_800_000_500),
        });
        let runner = FakeRunner::new().with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "27%",
            ],
            "",
        );

        let (response, outcome) = runtime.handle_ipc_request_with_runner(
            Request::RunOnce { force: true },
            &runner,
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
        );

        assert!(outcome.tick_attempted);

        let Response::RunOnce { run_once, .. } = response.response else {
            panic!("run_once request should return a run_once response");
        };

        assert_eq!(run_once.monitors_evaluated, 1);
        assert_eq!(run_once.writes_attempted, 1);
        assert_eq!(run_once.writes_skipped, 0);
        assert_eq!(run_once.writes_succeeded, 1);
    }

    #[test]
    fn reload_config_failure_keeps_previous_config() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime =
            DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
                .expect("runtime should bootstrap");
        let previous_tick_seconds = runtime.config.daemon.tick_seconds;

        let error = runtime
            .reload_config_with(|| {
                Err(ConfigError::AlreadyExists(PathBuf::from(
                    "/tmp/invalid.toml",
                )))
            })
            .expect_err("reload should fail");

        assert_eq!(runtime.config.daemon.tick_seconds, previous_tick_seconds);
        assert!(error.to_string().contains("config file already exists"));
    }

    #[test]
    fn reload_config_prunes_stale_state_without_crashing() {
        let temp = TempDir::new();
        let state_path = temp.path().join("state/runtime-state.json");
        let socket_path = temp.path().join("run/control.sock");
        let mut runtime = DaemonRuntime::bootstrap_with_paths(
            test_config_report(),
            state_path.clone(),
            socket_path,
        )
        .expect("runtime should bootstrap");
        runtime
            .state
            .record_apply_success("stale", 70, 1_800_000_000);
        runtime.state.manual_override = Some(ManualOverrideState {
            global_percent: None,
            global_expires_at_epoch_s: None,
            targets: std::collections::BTreeMap::from([
                (String::from("internal"), 25),
                (String::from("stale"), 70),
            ]),
            expires_at_epoch_s: Some(1_800_000_100),
        });
        runtime.weather_refresh.next_refresh_at_epoch_s = Some(1_800_000_500);
        runtime.weather_refresh.last_attempted_at_epoch_s = Some(1_800_000_300);
        runtime.weather_refresh.consecutive_failures = 3;
        runtime.weather_refresh.last_error = Some(String::from("temporary outage"));

        runtime
            .reload_config_with(|| Ok(test_config_report()))
            .expect("reload should succeed");

        assert!(runtime.state.monitor("stale").is_none());
        assert_eq!(runtime.weather_refresh.next_refresh_at_epoch_s, None);
        assert_eq!(runtime.weather_refresh.last_attempted_at_epoch_s, None);
        assert_eq!(runtime.weather_refresh.consecutive_failures, 0);
        assert_eq!(runtime.weather_refresh.last_error, None);
        assert_eq!(
            runtime.state.manual_override.as_ref().and_then(
                |manual_override| manual_override.target_percent("internal", 1_800_000_000)
            ),
            Some(25)
        );
        assert_eq!(
            runtime
                .state
                .manual_override
                .as_ref()
                .and_then(|manual_override| manual_override.target_percent("stale", 1_800_000_000)),
            None
        );
        let persisted = RuntimeState::load_from_path(&state_path).expect("state should persist");
        assert!(persisted.monitor("stale").is_none());
    }

    fn test_config_report() -> ConfigReport {
        ConfigReport {
            path: PathBuf::from("/tmp/sunreactor-config.toml"),
            source: ConfigSource::Defaults,
            config: Config {
                daemon: DaemonConfig {
                    tick_seconds: 60,
                    dry_run: false,
                    desktop_idle_sync: false,
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
                    max_step_pct_per_tick: 6,
                    min_write_delta_pct: 2,
                },
                monitors: vec![MonitorConfig {
                    logical_id: String::from("internal"),
                    backend: crate::backends::BackendKind::Backlight,
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

    fn test_weather_config_report() -> ConfigReport {
        let mut report = test_config_report();
        report.config.weather.enabled = true;
        report.config.weather.refresh_minutes = 30;
        report.config.weather.min_multiplier = 0.75;
        report
    }

    fn test_capability_snapshot() -> CapabilitySnapshot {
        CapabilitySnapshot {
            ddc_present: Vec::new(),
            backlights_present: vec![crate::discovery::BacklightDeviceDiscovery {
                stable_id: String::from("backlight:intel_backlight"),
                device_name: String::from("intel_backlight"),
                class: String::from("backlight"),
                max_brightness: Some(100),
                backlight_type: Some(String::from("raw")),
                probe_source: String::from("test"),
                sysfs_path: String::from("/sys/class/backlight/intel_backlight"),
                ddcci_connector: None,
                backend_viable: true,
                note: None,
            }],
        }
    }

    fn test_dry_run_config_report() -> ConfigReport {
        let mut report = test_config_report();
        report.config.daemon.dry_run = true;
        report
    }

    fn count_temp_state_files(state_path: &Path) -> usize {
        let Some(state_dir) = state_path.parent() else {
            return 0;
        };
        let prefix = format!(
            "{}.tmp-",
            state_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("runtime-state.json")
        );
        fs::read_dir(state_dir)
            .expect("state dir should be readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .count()
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should work")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("sunreactor-runtime-test-{unique}"));
            fs::create_dir_all(&path).expect("temp dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).ok();
        }
    }
}
