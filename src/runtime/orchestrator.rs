use crate::platform::{DisplayEventSources, DisplayRecoveryReason};
use crate::runtime::fade::{self, FadeEngine};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::fs;
use std::io;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tracing::{error, info};

use crate::apply::{self, ApplyRecord, ApplyStatus, ApplySummary};
use crate::backends::{FailureKind, ProcessRunner, RealProcessRunner};
use crate::config::{self, ConfigError, ConfigReport, ConfigSource};
use crate::ipc::{self, ControlSocket};
use crate::paths::{self, PathError};
use crate::policy::{PolicyError, PolicyOutput};
use crate::runtime::capabilities::{CapabilityRefreshPublication, CapabilityRefreshState};
use crate::runtime::runtime_config::RuntimeConfig;
use crate::runtime::topology::CapabilitySnapshot;
use crate::runtime::wake::{ProbeOutcome, ProbeSchedule, WakeReason};
use crate::runtime::weather_refresh::WeatherRefreshState;
use crate::solar::{SolarError, SolarSample};
use crate::state::{self, RuntimeState, StateError};
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
    /// Ephemeral ownership holds for monitors changed by another controller.
    /// These intentionally disappear when the daemon restarts.
    pub(super) external_holds: BTreeMap<String, apply::ExternalHold>,
    /// Shared recovery window opened by lifecycle/topology signals.
    pub(super) recovery_until_epoch_s: Option<u64>,
    pub weather_engine: weather::WeatherEngine,
    /// Ephemeral capability observation from the last reconciliation.
    /// Not persisted; kept separate from user config.
    pub last_capabilities: Option<CapabilitySnapshot>,
    pub(super) capability_refresh: CapabilityRefreshState,
    pub(super) probe_schedule: ProbeSchedule,
    pub(super) display_events: Option<DisplayEventSources>,
    /// Last `day_curve` answer. The curve only changes with the local date,
    /// the policy config and the weather multiplier, so repeated widget
    /// requests are served from memory instead of re-running the policy.
    pub(super) day_curve_cache: Option<(String, crate::ipc::DayCurveResponse)>,
    pending_apply: bool,
    pending_apply_force: bool,
    pending_apply_clear_backoff: bool,
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

impl DaemonRuntime {
    pub fn run_once(&mut self) -> Result<TickReport, RuntimeError> {
        self.run_once_at_with_runner(Utc::now(), &RealProcessRunner, false)
    }

    pub(super) fn mark_pending_apply(&mut self, force: bool, clear_backoff: bool) {
        self.pending_apply = true;
        self.pending_apply_force |= force;
        self.pending_apply_clear_backoff |= clear_backoff;
    }

    pub(super) fn take_pending_apply(&mut self) -> Option<(bool, bool)> {
        if !self.pending_apply {
            return None;
        }
        self.pending_apply = false;
        Some((
            std::mem::take(&mut self.pending_apply_force),
            std::mem::take(&mut self.pending_apply_clear_backoff),
        ))
    }

    pub(super) fn has_pending_apply(&self) -> bool {
        self.pending_apply
    }

    pub(super) fn open_recovery_window(&mut self, now_epoch_s: u64) {
        let until = now_epoch_s.saturating_add(apply::RECOVERY_WINDOW_SECONDS);
        self.recovery_until_epoch_s = Some(
            self.recovery_until_epoch_s
                .map_or(until, |current| current.max(until)),
        );
    }

    pub(super) fn clear_external_holds(&mut self, monitor_id: Option<&str>) {
        if let Some(monitor_id) = monitor_id {
            self.external_holds.remove(monitor_id);
            self.fade_engine.active_fades.remove(monitor_id);
        } else {
            self.external_holds.clear();
            self.fade_engine.cancel_all();
        }
    }

    /// Perform one complete capability observation before a one-shot apply.
    /// A one-shot invocation has no daemon loop available to perform the
    /// normal asynchronous observation/reconciliation handshake.
    pub fn refresh_capabilities(&mut self) {
        let report = crate::discovery::discover_with_runner(
            &RealProcessRunner,
            std::path::Path::new("/sys/class/backlight"),
        );
        self.fade_engine.cancel_all();
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

    pub(super) fn publish_completed_capability_snapshot(&mut self) -> Option<bool> {
        match self.capability_refresh.publish_completed() {
            Some(CapabilityRefreshPublication::Published(snapshot)) => {
                let topology_changed = self
                    .last_capabilities
                    .as_ref()
                    .is_some_and(|previous| previous != &snapshot);
                self.fade_engine.cancel_all();
                self.last_capabilities = Some(snapshot);
                if topology_changed {
                    self.open_recovery_window(state::current_epoch_s());
                }
                Some(true)
            }
            Some(
                CapabilityRefreshPublication::Discarded | CapabilityRefreshPublication::Failed,
            ) => Some(false),
            None => None,
        }
    }

    /// Speeds probes up after a wake hint; probes then fix any brightness a
    /// waking monitor or the desktop restored on its own. Hints are optional:
    /// the schedule finds the same wake by itself, just a little later.
    pub(super) fn request_wake_reassert(&mut self, reason: WakeReason) -> bool {
        if matches!(
            reason,
            WakeReason::WaylandIdleResume
                | WakeReason::SystemResume
                | WakeReason::DrmHotplug
                | WakeReason::DrmConnectorChange
        ) {
            self.open_recovery_window(state::current_epoch_s());
        }
        let opened = self.probe_schedule.signal(reason, Instant::now());
        if opened {
            tracing::info!(reason = ?reason, "probe_fast_window_opened");
        } else {
            tracing::debug!(reason = ?reason, "probe_fast_window_extended");
        }
        opened
    }

    pub(super) fn handle_display_event(&mut self, reason: DisplayRecoveryReason) {
        self.fade_engine.cancel_all();
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

    /// One probe: read every monitor that can be read cheaply and correct any
    /// whose brightness differs from its target. The outcome drives the next
    /// probe time, so a sleeping monitor is polled until it answers.
    pub(super) fn run_wake_probe<R: ProcessRunner>(
        &mut self,
        now_utc: DateTime<Utc>,
        runner: &R,
    ) -> ProbeOutcome {
        if self.fade_engine.is_fading() {
            return ProbeOutcome::default();
        }
        let computed = match self
            .collect_tick_inputs(now_utc, false)
            .and_then(|inputs| self.compute_tick_policy(inputs))
        {
            Ok(computed) => computed,
            Err(error) => {
                tracing::debug!(error = %error, "wake_probe_policy_unavailable");
                return ProbeOutcome::default();
            }
        };
        if computed.suspended {
            return ProbeOutcome::default();
        }
        let settings = self.force_apply_settings();
        let summary = apply::probe_and_correct_with_external_state(
            &self.config.monitors,
            &computed.policy,
            &mut self.state,
            computed.inputs.now_epoch_s,
            settings.dry_run,
            settings.external_change_hold,
            |monitor| crate::apply::dispatch::probe_monitor_percent(runner, monitor, &settings),
            |monitor, target, percent| {
                crate::apply::dispatch::apply_monitor_target(
                    runner, monitor, target, percent, &settings,
                )
            },
            Some(&mut self.external_holds),
            self.recovery_until_epoch_s,
            Some(&mut self.fade_engine),
        );
        tracing::debug!(
            probe = self.probe_schedule.probes(),
            answered = summary.answered,
            unreachable = summary.unreachable,
            corrected = summary.corrected,
            failed = summary.failed,
            "probe_completed"
        );
        if summary.corrected > 0 {
            let _ = self.persist_state_if_changed();
        }
        ProbeOutcome {
            answered: summary.answered,
            unreachable: summary.unreachable,
            corrected: summary.corrected,
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

// --- From helpers.rs ---
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

    /// Bootstraps one-shot mode without resolving or creating the IPC socket
    /// directory. Hardware apply remains available; only the persistent state
    /// directory is initialized.
    pub fn bootstrap_once() -> Result<Self, RuntimeError> {
        let config = config::load()?;
        let state_path = paths::state_file()?;
        let socket_path = state_path.with_file_name("once-control.sock");
        Self::bootstrap_with_paths_mode(config, state_path, socket_path, false)
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
        Self::bootstrap_with_paths_mode(config, state_path, socket_path, true)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn bootstrap_with_paths_mode(
        config: ConfigReport,
        state_path: PathBuf,
        socket_path: PathBuf,
        create_socket_dir: bool,
    ) -> Result<Self, RuntimeError> {
        for warning in &config.warnings {
            tracing::info!(warning = %warning, "config_compat_warning");
        }

        if let Some(state_dir) = state_path.parent() {
            create_runtime_dir(state_dir)?;
        }
        #[cfg(target_os = "linux")]
        if create_socket_dir {
            if let Some(socket_dir) = socket_path.parent() {
                create_runtime_dir(socket_dir)?;
            }
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
        let probe_schedule = ProbeSchedule::new(
            Duration::from_secs(config.daemon.probe_seconds),
            Instant::now(),
        );
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
            external_holds: BTreeMap::new(),
            recovery_until_epoch_s: Some(
                state::current_epoch_s().saturating_add(apply::RECOVERY_WINDOW_SECONDS),
            ),
            weather_engine,
            last_capabilities: None,
            capability_refresh: CapabilityRefreshState::default(),
            probe_schedule,
            display_events: None,
            day_curve_cache: None,
            pending_apply: false,
            pending_apply_force: false,
            pending_apply_clear_backoff: false,
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

#[cfg(test)]
mod tests;
