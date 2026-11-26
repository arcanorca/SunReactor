use chrono::{DateTime, Utc};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(not(target_os = "linux"))]
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::backends::{ProcessRunner, RealProcessRunner};
use crate::ipc::BoundControlSocket;
#[cfg(all(test, target_os = "linux"))]
use crate::ipc::{self, ControlSocket};
use crate::platform::DisplayEventSources;

use super::idle::DesktopIdleSync;
use super::orchestrator::{
    classify_ipc_accept_error, log_accept_error_rate_limited, log_tick, log_tick_error,
    AcceptErrorAction, DaemonRuntime, RuntimeError, ACCEPT_BACKOFF,
};
use super::wake::WakeReason;

const IPC_DRAIN_MAX_REQUESTS: usize = 16;
const IPC_DRAIN_MAX_DURATION: Duration = Duration::from_millis(8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IpcDrainOutcome {
    processed: usize,
}

const DEFERRED_KICK_MIN_GAP: Duration = Duration::from_secs(5);
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
    // SAFETY: `pollfd` is a valid, exclusively borrowed pollfd and nfds is 1.
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
    handle: F,
) -> Result<IpcDrainOutcome, RuntimeError>
where
    F: FnMut(super::ipc::NativeIpcStream),
{
    drain_ready_connections_within(
        listener,
        IPC_DRAIN_MAX_REQUESTS,
        IPC_DRAIN_MAX_DURATION,
        handle,
    )
}

/// Drain with explicit limits; the production quantum is
/// [`drain_ready_connections`]. Tests pass a generous duration so only the
/// request count is under test, independent of CPU load.
fn drain_ready_connections_within<F>(
    listener: &BoundControlSocket,
    max_requests: usize,
    max_duration: Duration,
    mut handle: F,
) -> Result<IpcDrainOutcome, RuntimeError>
where
    F: FnMut(super::ipc::NativeIpcStream),
{
    let started = Instant::now();
    let mut outcome = IpcDrainOutcome::default();
    // Dropped (bad) connections do not count as processed requests, so bound
    // them separately; otherwise a flood of bad peers could spin the loop.
    let mut dropped = 0_usize;
    while outcome.processed < max_requests
        && dropped < max_requests
        && started.elapsed() < max_duration
    {
        match listener.accept() {
            Ok(Some(stream)) => {
                outcome.processed += 1;
                handle(stream);
            }
            Ok(None) => return Ok(outcome),
            Err(error) => match classify_ipc_accept_error(&error) {
                AcceptErrorAction::DropConnection => {
                    log_accept_error_rate_limited(&error, AcceptErrorAction::DropConnection);
                    dropped += 1;
                }
                AcceptErrorAction::BackOff => {
                    log_accept_error_rate_limited(&error, AcceptErrorAction::BackOff);
                    std::thread::sleep(ACCEPT_BACKOFF);
                    return Ok(outcome);
                }
                AcceptErrorAction::ListenerDead => return Err(RuntimeError::from(error)),
            },
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
            LoopAction::RunFadeStep => self.run_fade_step(now_utc, runner, listener),
            LoopAction::Sleep(duration) => wait_for_ipc_or_sleep(listener, duration),
        }
    }

    pub(super) fn run_fade_step<R: ProcessRunner + Sync>(
        &mut self,
        now_utc: DateTime<Utc>,
        runner: &R,
        listener: &BoundControlSocket,
    ) {
        if self.config.daemon.dry_run {
            self.fade_engine.cancel_all();
            return;
        }
        // Fade steps are hardware writes too.  Do not let the high-
        // cadence path bypass topology authorization while the first
        // complete capability observation is still pending.
        let Some(capabilities) = self.last_capabilities.as_ref() else {
            self.fade_engine.cancel_all();
            wait_for_ipc_or_sleep(listener, Duration::from_millis(16));
            return;
        };
        let reconcile_actions =
            crate::runtime::topology::reconcile(&self.config.monitors, capabilities);
        let step_start = Instant::now();
        let now_epoch_s = now_utc.timestamp().max(0) as u64;
        let steps = self.fade_engine.process_tick_steps(now_epoch_s);
        for step in steps {
            self.apply_fade_step(&step, now_utc, runner, &reconcile_actions);
        }
        if let Some(remaining) = Duration::from_millis(16).checked_sub(step_start.elapsed()) {
            wait_for_ipc_or_sleep(listener, remaining);
        }
    }

    fn handle_fade_admission_skip(
        &mut self,
        reason: crate::apply::decision::SkipReason,
        step: &crate::runtime::fade::FadeStep,
        action: crate::runtime::topology::ReconcileAction,
        last_applied_percent: Option<u8>,
    ) {
        use crate::apply::decision::SkipReason;
        let monitor_id = &step.monitor_id;
        let percent = step.percent;
        match reason {
            SkipReason::Topology => {
                tracing::debug!(
                    monitor = %monitor_id,
                    action = action.name(),
                    "fade_step_skipped_topology"
                );
                self.fade_engine.active_fades.remove(monitor_id);
            }
            SkipReason::Contested { until_epoch_s } => {
                tracing::info!(
                    logical_id = %monitor_id,
                    until_epoch_s,
                    "fade_step_stopped; monitor_contested"
                );
                self.fade_engine.active_fades.remove(monitor_id);
            }
            SkipReason::Disabled | SkipReason::Hold { .. } => {
                self.fade_engine.active_fades.remove(monitor_id);
            }
            SkipReason::Budget {
                resume_at_epoch_s: Some(resume_at),
                ..
            } => {
                let last_applied = last_applied_percent.unwrap_or(percent);
                let final_target = if step.is_final {
                    percent
                } else {
                    self.fade_engine
                        .active_fades
                        .get(monitor_id)
                        .map_or(percent, |fade| fade.target_percent)
                };
                tracing::info!(
                    logical_id = %monitor_id,
                    final_target,
                    resume_at_epoch_s = resume_at,
                    "fade_collapsed; write_budget"
                );
                self.fade_engine
                    .park_final(monitor_id, last_applied, final_target, resume_at);
            }
            SkipReason::AlreadyMatches
            | SkipReason::Budget { .. }
            | SkipReason::Backoff { .. }
            | SkipReason::MinInterval { .. }
            | SkipReason::CorrectionInterval
            | SkipReason::ManualCap { .. } => {}
        }
    }

    /// Applies one fade step to one monitor, dropping its fade when the
    /// monitor is held, contested, gone, topology-unsafe, or failing.
    pub(super) fn apply_fade_step<R: ProcessRunner + Sync>(
        &mut self,
        step: &crate::runtime::fade::FadeStep,
        now_utc: DateTime<Utc>,
        runner: &R,
        reconcile_actions: &[crate::runtime::topology::ReconcileAction],
    ) {
        use crate::apply::decision::{
            admit_write, record_attempted_hardware_write, AdmissionFacts, WritePath,
        };

        let monitor_id = step.monitor_id.clone();
        let percent = step.percent;
        let now_epoch_s = now_utc.timestamp().max(0) as u64;
        if let Some(hold) = self.external_holds.get(&monitor_id).copied() {
            if hold.until_epoch_s <= now_epoch_s {
                self.external_holds.remove(&monitor_id);
                self.fade_engine.active_fades.remove(&monitor_id);
                return;
            }
        }
        let Some(monitor_index) = self
            .config
            .monitors
            .iter()
            .position(|monitor| monitor.logical_id == monitor_id)
        else {
            self.fade_engine.active_fades.remove(&monitor_id);
            return;
        };
        let action = reconcile_actions
            .get(monitor_index)
            .copied()
            .unwrap_or(crate::runtime::topology::ReconcileAction::AmbiguousOrUnsafe);
        let Some(monitor) = self.config.monitors.get(monitor_index) else {
            return;
        };
        let monitor_state = self.state.monitor(&monitor_id);
        let last_applied_percent = monitor_state.and_then(|state| state.last_applied_percent);
        let facts = AdmissionFacts {
            logical_id: &monitor_id,
            backend: monitor.backend,
            enabled: monitor.enabled,
            topology_allowed: action.allowed_to_apply(),
            active_hold_until_epoch_s: self
                .external_holds
                .get(&monitor_id)
                .map(|hold| hold.until_epoch_s),
            last_applied_percent,
            last_applied_at_epoch_s: monitor_state.and_then(|state| state.last_applied_at_epoch_s),
            backoff: monitor_state.and_then(|state| state.backoff.as_ref()),
            min_apply_interval: Duration::ZERO,
            in_recovery: false,
            budget_exempt: step.budget_exempt,
            fade_target_percent: Some(percent),
            check_rate_limits: true,
        };
        if let Err(reason) = admit_write(WritePath::FadeStep, &facts, &self.state, now_epoch_s) {
            self.handle_fade_admission_skip(reason, step, action, last_applied_percent);
            return;
        }
        let target = crate::policy::PerMonitorTarget {
            logical_id: monitor_id,
            percent,
            solar_daylight_factor: 0.0,
            effective_daylight_factor: 0.0,
        };
        let settings = self.config.apply;
        let result =
            crate::apply::apply_monitor_target(runner, monitor, &target, percent, &settings);
        record_attempted_hardware_write(
            &mut self.state,
            &target.logical_id,
            now_epoch_s,
            &result,
            "hardware_write_fade_step",
        );
        match result {
            Ok(_) => {
                self.state
                    .record_apply_success(&target.logical_id, percent, now_epoch_s);
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
                    now_epoch_s,
                );
                // Stop fading this monitor if it's failing
                self.fade_engine.active_fades.remove(&target.logical_id);
            }
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

    fn execute_pending_apply(
        &mut self,
        now_utc: DateTime<Utc>,
        cadence: &mut LoopCadence,
        desktop_idle: &mut DesktopIdleSync,
    ) {
        let Some((force, clear_backoff)) = self.take_pending_apply() else {
            return;
        };
        cadence.note_tick_attempt();
        desktop_idle.note_tick_attempt(now_utc);
        let result = if clear_backoff {
            self.run_resync_at_with_runner(now_utc, &RealProcessRunner, true)
        } else {
            self.run_once_at_with_runner(now_utc, &RealProcessRunner, force)
        };
        match result {
            Ok(report) => log_tick(&report),
            Err(error) => log_tick_error(&error),
        }
    }
}

impl DaemonRuntime {
    fn initialize_loop(
        &mut self,
    ) -> Result<(BoundControlSocket, DesktopIdleSync, LoopCadence), RuntimeError> {
        #[cfg(target_os = "linux")]
        let listener = self
            .socket
            .bind_listener_with_lock(self.instance_lock.take())?;
        #[cfg(not(target_os = "linux"))]
        let listener = self.socket.bind_listener()?;
        let desktop_idle_sync = self.config.daemon.desktop_idle_sync;
        let desktop_idle = DesktopIdleSync::new(
            desktop_idle_sync,
            PathBuf::from(self.socket.display_target()),
            self.config.daemon.tick_seconds,
            self.config.daemon.desktop_idle_timeout_minutes,
        );
        let cadence = LoopCadence::new(self.config.daemon.tick_seconds);
        self.display_events = Some(DisplayEventSources::spawn());

        self.log_startup(desktop_idle_sync);

        Ok((listener, desktop_idle, cadence))
    }

    fn run_loop_iteration<F>(
        &mut self,
        listener: &BoundControlSocket,
        cadence: &mut LoopCadence,
        desktop_idle: &mut DesktopIdleSync,
        apply_after_drain: &mut bool,
        shutdown_requested: &mut F,
    ) -> Result<bool, RuntimeError>
    where
        F: FnMut() -> bool,
    {
        self.observe_clock();
        self.queue_deferred_manual_apply();
        let _ = self.drain_ipc_requests_with_quantum_impl(listener, cadence, desktop_idle)?;

        if shutdown_requested() {
            return Ok(false);
        }

        if *apply_after_drain {
            *apply_after_drain = false;
            self.execute_pending_apply(Utc::now(), cadence, desktop_idle);
        }
        if self.has_pending_apply() {
            // Give clients that arrive immediately after a mutation one more
            // bounded IPC drain before a slow hardware apply starts.
            *apply_after_drain = true;
            return Ok(true);
        }

        let now_utc = Utc::now();
        self.handle_pending_display_event();
        #[cfg(target_os = "linux")]
        desktop_idle.maintain_watcher(&self.socket.path);
        #[cfg(not(target_os = "linux"))]
        desktop_idle.maintain_watcher(Path::new(""));
        if desktop_idle.perform_due_action(self, now_utc, &RealProcessRunner, cadence) {
            return Ok(true);
        }

        if DesktopIdleSync::take_input_resume() {
            let _ = self.request_wake_reassert(WakeReason::WaylandIdleResume);
        }
        let probe_at = Instant::now();
        if self.probe_schedule.take_due_probe(probe_at) {
            let outcome = self.run_wake_probe(now_utc, &RealProcessRunner);
            self.probe_schedule.note_outcome(outcome, Instant::now());
            return Ok(true);
        }

        let next_deadline = self.next_wake_deadline(desktop_idle);
        let action = cadence.next_action(
            next_deadline,
            self.fade_engine
                .is_stepping(now_utc.timestamp().max(0) as u64),
        );
        if let Some(result) = self.publish_completed_capability_snapshot() {
            if self.capability_refresh.take_pending() {
                if result {
                    self.execute_scheduled_tick(now_utc, &RealProcessRunner, cadence, desktop_idle);
                }
                return Ok(true);
            }
        }
        if self.capability_refresh.is_pending() {
            wait_for_ipc_or_sleep(listener, Duration::from_millis(16));
            return Ok(true);
        }

        // `ddcutil detect` + capabilities is heavy bus traffic: observe only on
        // startup, on hotplug/resume/reload/explicit request, and as a slow
        // safety net, never on every tick.
        let refresh_now = Instant::now();
        if self
            .capability_refresh
            .due(refresh_now, self.last_capabilities.is_some())
        {
            self.capability_refresh.set_pending();
            self.capability_refresh.note_started(refresh_now);
            self.begin_capability_refresh_with_runner(RealProcessRunner);
            wait_for_ipc_or_sleep(listener, Duration::from_millis(16));
            return Ok(true);
        }

        self.perform_loop_action(
            action,
            now_utc,
            &RealProcessRunner,
            cadence,
            desktop_idle,
            listener,
        );
        Ok(true)
    }

    /// Starts the main daemon event loop.
    ///
    /// This function handles IPC requests, idle state transitions, and
    /// applies hardware brightness targets asynchronously until
    /// `shutdown_requested` returns true.
    pub fn run_loop<F>(&mut self, mut shutdown_requested: F) -> Result<(), RuntimeError>
    where
        F: FnMut() -> bool,
    {
        let (listener, mut desktop_idle, mut cadence) = self.initialize_loop()?;

        let mut apply_after_drain = false;
        let mut result = Ok(());
        while !shutdown_requested() {
            match self.run_loop_iteration(
                &listener,
                &mut cadence,
                &mut desktop_idle,
                &mut apply_after_drain,
                &mut shutdown_requested,
            ) {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    result = Err(error);
                    break;
                }
            }
        }

        // Persist on every exit path, including a fatal loop error.
        let _ = self.persist_state_if_changed();
        drop(listener);

        match &result {
            Ok(()) => tracing::info!(mode = "daemon", reason = "signal", "shutdown"),
            Err(error) => {
                tracing::error!(mode = "daemon", error = %error, "shutdown_after_fatal_error");
            }
        }
        result
    }

    fn log_startup(&self, desktop_idle_sync: bool) {
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
    }

    fn handle_pending_display_event(&mut self) {
        let pending_event = self
            .display_events
            .as_ref()
            .and_then(DisplayEventSources::drain);
        if let Some(request) = pending_event {
            tracing::info!(reason = ?request.reason, "display_recovery_requested");
            self.handle_display_event(request.reason);
        }
    }

    /// A user-forced write that hit the manual rate limit is retried once at
    /// the time the limiter allows: one coalesced forced apply of the latest
    /// value, never a stream of writes. Kicks are spaced by
    /// [`DEFERRED_KICK_MIN_GAP`] so a stale marker cannot spin the loop.
    fn queue_deferred_manual_apply(&mut self) {
        let now_epoch_s = crate::state::current_epoch_s();
        let Some(at) = crate::apply::next_deferred_manual_at(now_epoch_s) else {
            return;
        };
        if at > now_epoch_s || self.has_pending_apply() {
            return;
        }
        let now = Instant::now();
        if self
            .last_deferred_kick
            .is_some_and(|last| now.duration_since(last) < DEFERRED_KICK_MIN_GAP)
        {
            return;
        }
        self.last_deferred_kick = Some(now);
        self.mark_pending_apply(true, false);
    }

    fn deferred_manual_deadline() -> Option<Instant> {
        let now_epoch_s = crate::state::current_epoch_s();
        crate::apply::next_deferred_manual_at(now_epoch_s).map(|at| {
            Instant::now()
                + Duration::from_secs(at.saturating_sub(now_epoch_s)).max(DEFERRED_KICK_MIN_GAP)
        })
    }

    fn next_wake_deadline(&self, desktop_idle: &DesktopIdleSync) -> Option<Instant> {
        [
            desktop_idle.next_deadline(),
            self.probe_schedule.next_deadline(),
            Self::deferred_manual_deadline(),
        ]
        .into_iter()
        .flatten()
        .min()
    }
}

#[cfg(test)]
mod loop_timing_tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use std::os::unix::net::UnixStream;
    #[cfg(target_os = "linux")]
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Upper bound for test drains so only the request count is asserted.
    #[cfg(target_os = "linux")]
    const TEST_DRAIN_DEADLINE: Duration = Duration::from_secs(2);

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
        // `connect` returns only once the connection sits in the listener's
        // backlog, so everything is queued here; no timing assumption. The
        // generous duration keeps CPU load from cutting the drain short.
        let handled = std::sync::atomic::AtomicUsize::new(0);
        let first = drain_ready_connections_within(
            &listener,
            IPC_DRAIN_MAX_REQUESTS,
            TEST_DRAIN_DEADLINE,
            |stream| {
                handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                drop(stream);
            },
        )
        .expect("first drain should succeed");
        assert_eq!(first.processed, IPC_DRAIN_MAX_REQUESTS);

        let mut second_processed = 0;
        let deadline = Instant::now() + TEST_DRAIN_DEADLINE;
        while second_processed < 2 && Instant::now() < deadline {
            let second = drain_ready_connections_within(
                &listener,
                IPC_DRAIN_MAX_REQUESTS,
                TEST_DRAIN_DEADLINE,
                |stream| {
                    handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    drop(stream);
                },
            )
            .expect("second drain should succeed");
            second_processed += second.processed;
            if second.processed == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        assert!(second_processed >= 1);
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
        let handled = std::sync::atomic::AtomicUsize::new(0);
        let result = drain_ready_connections_within(
            &listener,
            IPC_DRAIN_MAX_REQUESTS,
            TEST_DRAIN_DEADLINE,
            |stream| {
                // Production counts the connection before parsing its frame.
                handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                drop(stream);
            },
        )
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
