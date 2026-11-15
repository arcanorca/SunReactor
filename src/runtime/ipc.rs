use chrono::{DateTime, Utc};

use crate::apply;
use crate::backends::{ProcessRunner, RealProcessRunner};
use crate::config::{self, ConfigError, ConfigReport};
use crate::ipc;

use super::orchestrator::{DaemonRuntime, IpcOutcome, RuntimeError};
use super::runtime_config::RuntimeConfig;
use super::wake::WakeReason;

#[cfg(target_os = "linux")]
pub(crate) type NativeIpcStream = std::os::unix::net::UnixStream;

#[cfg(target_os = "windows")]
pub(crate) type NativeIpcStream = crate::platform::windows::pipe::NamedPipeStream;

impl DaemonRuntime {
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_ipc_request_with_runner<R: ProcessRunner + Sync>(
        &mut self,
        request: ipc::Request,
        runner: &R,
        now_utc: DateTime<Utc>,
    ) -> (ipc::ResponseEnvelope, IpcOutcome) {
        let _ = runner;
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
                    Ok(message) => {
                        self.mark_pending_apply(false, false);
                        (ipc::ResponseEnvelope::ack(message), IpcOutcome::default())
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
            ipc::Request::Resume => match self.resume() {
                Ok(()) => {
                    self.mark_pending_apply(true, true);
                    (
                        ipc::ResponseEnvelope::ack(
                            "resumed automatic apply and cleared manual overrides; apply queued",
                        ),
                        IpcOutcome {
                            tick_attempted: true,
                            config_reloaded: false,
                        },
                    )
                }
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
                    Ok(message) => {
                        self.mark_pending_apply(true, false);
                        (
                            ipc::ResponseEnvelope::ack(format!("{message}; apply queued")),
                            outcome,
                        )
                    }
                    Err(error) => (ipc_error_response(&error), outcome),
                }
            }
            ipc::Request::SetMonitorLimits {
                monitor_id,
                min_pct,
                max_pct,
            } => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                match self.set_monitor_limits(&monitor_id, min_pct, max_pct) {
                    Ok(message) => {
                        self.mark_pending_apply(true, false);
                        (
                            ipc::ResponseEnvelope::ack(format!("{message}; apply queued")),
                            outcome,
                        )
                    }
                    Err(error) => (ipc_error_response(&error), outcome),
                }
            }
            ipc::Request::SetCloudDimming { percent } => {
                if percent > 50 {
                    return (
                        ipc::ResponseEnvelope::error(
                            ipc::ErrorCode::InvalidRequest,
                            "cloud dimming percent must be within 0..=50",
                        ),
                        IpcOutcome::default(),
                    );
                }
                match self.set_cloud_dimming(percent) {
                    Ok(message) => {
                        // The next daemon tick observes the new multiplier. Keep the
                        // request path free of a synchronous hardware apply.
                        self.mark_pending_apply(true, false);
                        (
                            ipc::ResponseEnvelope::ack(format!("{message}; apply queued")),
                            IpcOutcome::default(),
                        )
                    }
                    Err(error) => (ipc_error_response(&error), IpcOutcome::default()),
                }
            }
            ipc::Request::ClearOverride { monitor_id, global } => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                match self.clear_override(monitor_id.as_deref(), global) {
                    Ok(message) => {
                        self.mark_pending_apply(true, false);
                        (
                            ipc::ResponseEnvelope::ack(format!("{message}; apply queued")),
                            outcome,
                        )
                    }
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
                        self.mark_pending_apply(true, false);
                        (
                            ipc::ResponseEnvelope::ack(format!("{message}; apply queued")),
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

                self.clear_external_holds(None);
                self.mark_pending_apply(force, false);
                (
                    ipc::ResponseEnvelope::run_once(
                        queued_run_once_response(),
                        "daemon tick queued",
                    ),
                    outcome,
                )
            }
            ipc::Request::IdleDim => {
                let outcome = IpcOutcome {
                    tick_attempted: true,
                    config_reloaded: false,
                };
                self.state.desktop_idle_dimmed = true;
                self.mark_pending_apply(true, false);
                (
                    ipc::ResponseEnvelope::ack("desktop idle dimming active; apply queued"),
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
                self.mark_pending_apply(true, false);
                (
                    ipc::ResponseEnvelope::ack("desktop idle wake queued for re-observation"),
                    outcome,
                )
            }
            ipc::Request::ListThemes => (Self::theme_list_response(), IpcOutcome::default()),
            ipc::Request::GetTheme { id } => (Self::theme_response(&id), IpcOutcome::default()),
            ipc::Request::DayCurve { monitor_id } => (
                self.day_curve_response(monitor_id.as_deref(), now_utc),
                IpcOutcome::default(),
            ),
            ipc::Request::ExternalBrightnessChange => {
                let response = self
                    .collect_tick_inputs(now_utc, false)
                    .and_then(|inputs| self.compute_tick_policy(inputs))
                    .map_or_else(
                        |error| {
                            ipc::ResponseEnvelope::error(
                                ipc::ErrorCode::InternalError,
                                error.to_string(),
                            )
                        },
                        |computed| {
                            if computed.suspended {
                                return ipc::ResponseEnvelope::ack(
                                    "external brightness change observed while suspended",
                                );
                            }
                            let settings = self.force_apply_settings();
                            let summary = apply::probe_and_correct_with_external_state(
                                &self.config.monitors,
                                &computed.policy,
                                &mut self.state,
                                now_epoch_s,
                                settings.dry_run,
                                settings.external_change_hold,
                                |monitor| {
                                    crate::apply::dispatch::probe_monitor_percent(
                                        runner, monitor, &settings,
                                    )
                                },
                                |monitor, target, percent| {
                                    crate::apply::dispatch::apply_monitor_target(
                                        runner, monitor, target, percent, &settings,
                                    )
                                },
                                Some(&mut self.external_holds),
                                self.recovery_until_epoch_s,
                                Some(&mut self.fade_engine),
                            );
                            if summary.corrected > 0 {
                                let _ = self.persist_state_if_changed();
                            }
                            ipc::ResponseEnvelope::ack(format!(
                                "external brightness change observed; {} monitor(s) held",
                                self.external_holds.len()
                            ))
                        },
                    );
                (response, IpcOutcome::default())
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
        let previous_monitors = self.state.monitors.clone();
        let previous_holds = self.external_holds.clone();
        let previous_recovery_until = self.recovery_until_epoch_s;

        self.state.suspend_until_epoch_s = None;
        self.state.suspend_indefinite = false;
        self.clear_external_holds(None);
        self.open_recovery_window(crate::state::current_epoch_s());
        for monitor in self.state.monitors.values_mut() {
            monitor.backoff = None;
        }

        if let Err(error) = self.persist_state_if_changed() {
            self.state.suspend_until_epoch_s = previous_suspend_until;
            self.state.suspend_indefinite = previous_suspend_indefinite;
            self.state.manual_override = previous_override;
            self.state.monitors = previous_monitors;
            self.external_holds = previous_holds;
            self.recovery_until_epoch_s = previous_recovery_until;
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

        self.clear_external_holds(monitor_id);

        Ok(message)
    }

    fn set_monitor_limits(
        &mut self,
        monitor_id: &str,
        min_pct: u8,
        max_pct: u8,
    ) -> Result<String, RuntimeError> {
        if min_pct > 100 || max_pct > 100 {
            return Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                message: String::from("limits must be in the range 0..=100"),
            }));
        }
        if min_pct > max_pct {
            return Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                message: format!("min_pct ({min_pct}) cannot exceed max_pct ({max_pct})"),
            }));
        }
        self.ensure_configured_monitor(monitor_id)?;

        let message = self.persist_config_edit(|config| {
            let Some(monitor) = config
                .monitors
                .iter_mut()
                .find(|monitor| monitor.logical_id == monitor_id)
            else {
                return Err(RuntimeError::Ipc(ipc::IpcError::Protocol {
                    message: format!("unknown monitor logical_id `{monitor_id}` in config file"),
                }));
            };
            monitor.min_pct = min_pct;
            monitor.max_pct = max_pct;
            Ok(format!(
                "updated limits for {monitor_id} to min={min_pct}%, max={max_pct}%"
            ))
        })?;
        // Apply exactly the persisted edit to the live config; unrelated
        // on-disk changes wait for an explicit reload.
        if let Some(monitor) = self
            .config
            .monitors
            .iter_mut()
            .find(|monitor| monitor.logical_id == monitor_id)
        {
            monitor.min_pct = min_pct;
            monitor.max_pct = max_pct;
        }
        self.fade_engine.cancel_all();
        self.day_curve_cache = None;

        Ok(message)
    }

    fn set_cloud_dimming(&mut self, percent: u8) -> Result<String, RuntimeError> {
        let message = self.persist_config_edit(|config| {
            config
                .weather
                .set_cloud_dimming_percent(percent)
                .map_err(|message| RuntimeError::Ipc(ipc::IpcError::Protocol { message }))?;
            Ok(format!("updated maximum cloud dimming to {percent}%"))
        })?;

        self.config
            .weather
            .set_cloud_dimming_percent(percent)
            .map_err(|message| RuntimeError::Ipc(ipc::IpcError::Protocol { message }))?;
        self.weather_engine
            .sync_config(&self.config.weather, &self.config.location);
        self.day_curve_cache = None;
        Ok(message)
    }

    /// Read the on-disk config, apply one targeted edit, validate, and
    /// atomically persist it. Other keys keep their on-disk values. The
    /// caller mirrors the same edit onto the live config; it must not adopt
    /// the disk snapshot, which would bypass `apply_reloaded_runtime_config`.
    fn persist_config_edit<F>(&self, update: F) -> Result<String, RuntimeError>
    where
        F: FnOnce(&mut crate::config::Config) -> Result<String, RuntimeError>,
    {
        let mut report = config::load_from_path(&self.config.path)?;
        let message = update(&mut report.config)?;
        RuntimeConfig::from_report(report.clone())?;
        config::save_to_path(&report.config, &self.config.path)?;
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

        self.clear_external_holds(monitor_id);

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
        // Pick up added/edited user theme files before the config is
        // validated against the registry.
        crate::theme::reload_registry();
        self.reload_config_with(config::load)
    }

    /// Test hook: reload with an explicit theme registry instead of the
    /// process-wide one.
    #[cfg(test)]
    pub(super) fn reload_config_with_themes<F>(
        &mut self,
        loader: F,
        registry: &crate::theme::Registry,
    ) -> Result<(), RuntimeError>
    where
        F: FnOnce() -> Result<ConfigReport, ConfigError>,
    {
        let next = loader()?;
        self.apply_reloaded_runtime_config(RuntimeConfig::from_report_with_themes(next, registry)?)
    }

    fn apply_reloaded_config(&mut self, next: ConfigReport) -> Result<(), RuntimeError> {
        self.apply_reloaded_runtime_config(RuntimeConfig::from_report(next)?)
    }

    fn apply_reloaded_runtime_config(
        &mut self,
        new_config: RuntimeConfig,
    ) -> Result<(), RuntimeError> {
        self.fade_engine.cancel_all();
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

        self.external_holds.retain(|logical_id, _| {
            self.config
                .monitors
                .iter()
                .any(|monitor| monitor.logical_id == *logical_id)
        });
        if self.config.daemon.external_change_hold_minutes == 0 {
            self.clear_external_holds(None);
        }
        self.open_recovery_window(crate::state::current_epoch_s());

        Ok(())
    }
}

pub(super) fn ipc_error_response(error: &RuntimeError) -> ipc::ResponseEnvelope {
    let code = match error {
        RuntimeError::Ipc(ipc::IpcError::Protocol { .. }) => ipc::ErrorCode::InvalidRequest,
        _ => ipc::ErrorCode::InternalError,
    };
    ipc::ResponseEnvelope::error(code, error.to_string())
}

fn queued_run_once_response() -> ipc::RunOnceResponse {
    ipc::RunOnceResponse {
        tick_duration_ms: 0,
        monitors_evaluated: 0,
        writes_attempted: 0,
        writes_skipped: 0,
        writes_succeeded: 0,
        writes_failed: 0,
    }
}

impl DaemonRuntime {
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
        } else if !matches!(
            response_kind.as_str(),
            "status" | "pong" | "themes" | "theme" | "day_curve"
        ) {
            tracing::info!(
                request = %request_name,
                response = %response_kind,
                "ipc_request"
            );
        }

        outcome
    }
}
