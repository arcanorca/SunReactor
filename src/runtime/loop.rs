use chrono::{DateTime, Utc};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(not(target_os = "linux"))]
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::backends::{ProcessRunner, RealProcessRunner};
use crate::ipc::BoundControlSocket;
#[cfg(test)]
use crate::ipc::{self, ControlSocket};
use crate::platform::DisplayEventSources;

use super::idle::DesktopIdleSync;
use super::orchestrator::{
    is_transient_ipc_accept_error, log_tick, log_tick_error, DaemonRuntime, RuntimeError,
};
use super::wake::WakeReason;

const IPC_DRAIN_MAX_REQUESTS: usize = 16;
const IPC_DRAIN_MAX_DURATION: Duration = Duration::from_millis(8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IpcDrainOutcome {
    processed: usize,
}

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
    F: FnMut(super::ipc::NativeIpcStream),
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
