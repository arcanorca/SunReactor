use chrono::{DateTime, Utc};

use crate::apply::{ApplyStatus, ApplySummary};
use crate::backends::{ProcessRunner, RealProcessRunner};
use crate::config::{self, ConfigError, ConfigReport};
use crate::ipc;

use super::orchestrator::{log_tick, DaemonRuntime, IpcOutcome, RuntimeError, TickReport};
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
