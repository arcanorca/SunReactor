use std::time::Instant;

use crate::tui::model::{ActionKind, ActionState, ErrorCategory, MonitorDiscoveryState};
use crate::tui::worker::IpcEvent;
use crate::tui::{DaemonConnection, Model};

#[allow(clippy::too_many_lines)]
pub(super) fn handle_ipc(event: IpcEvent, model: &mut Model) {
    match event {
        IpcEvent::Status(status) => {
            let now = std::time::Instant::now();
            model.motion.record_heartbeat(now);
            for monitor in &status.monitors {
                let previous = model
                    .status
                    .as_ref()
                    .and_then(|old| {
                        old.monitors
                            .iter()
                            .find(|old| old.logical_id == monitor.logical_id)
                    })
                    .and_then(|old| old.last_applied_percent);
                if let (Some(from), Some(to)) = (previous, monitor.last_applied_percent) {
                    if from != to {
                        model
                            .motion
                            .trigger_output_roll(&monitor.logical_id, from, to, now);
                    }
                }
            }
            let old_multiplier = model
                .status
                .as_ref()
                .and_then(|s| s.weather.as_ref())
                .and_then(|w| w.multiplier);
            let new_multiplier = status.weather.as_ref().and_then(|w| w.multiplier);

            if let Some(weather) = &status.weather {
                if let Some(fetched_at) = weather.fetched_at_epoch_s {
                    match model.last_weather_fetched_at {
                        None => {
                            // First observed snapshot establishes baseline only.
                            // Synchronization must NOT masquerade as a new fetch.
                            model.last_weather_fetched_at = Some(fetched_at);
                        }
                        Some(prev) if fetched_at > prev => {
                            // Strictly newer snapshot: trigger confirmation transient.
                            model.motion.trigger(
                                crate::tui::motion::TransientKind::WeatherUpdate,
                                now,
                                std::time::Duration::from_millis(650),
                            );
                            model.motion.stop_activity();
                            model.last_weather_fetched_at = Some(fetched_at);
                        }
                        Some(_) => {
                            // Same or older snapshot: no update transient.
                        }
                    }
                }
                if weather.last_error.is_some() {
                    model.motion.stop_activity();
                }
            }

            model.status = Some(*status);
            model.clamp_monitor_selection();

            if old_multiplier != new_multiplier {
                model.request_preview_refresh();
            }
            model.daemon_connection = DaemonConnection::Connected;
            model.request_monitor_discovery_if_needed();
        }
        IpcEvent::MonitorDiscovery(report) => {
            model.monitor_discovery = MonitorDiscoveryState::Complete(report);
        }
        IpcEvent::Connected => {
            model.daemon_connection = DaemonConnection::Connected;
        }
        IpcEvent::Disconnected => {
            model.daemon_connection = DaemonConnection::Disconnected;
            model.status = None;
            if let ActionState::Pending {
                command_id, action, ..
            } = model.action_state
            {
                if action == ActionKind::SaveConfig {
                    tracing::warn!(
                        command_id,
                        category = ?ErrorCategory::DaemonUnavailable,
                        "config_saved_daemon_unsynced"
                    );
                    model.action_state = ActionState::Warning {
                        action,
                        category: ErrorCategory::DaemonUnavailable,
                        message: String::from("Saved to disk — daemon offline"),
                        completed_at: Instant::now(),
                    };
                } else {
                    model.action_state = ActionState::Error {
                        action,
                        category: ErrorCategory::DaemonUnavailable,
                        message: format!("{}: daemon is offline", action.label()),
                        completed_at: Instant::now(),
                    };
                }
            }
        }
        IpcEvent::CommandSucceeded {
            command_id,
            action,
            message,
        } => {
            let is_matching = match model.action_state {
                ActionState::Pending {
                    command_id: pending_id,
                    ..
                } => pending_id == command_id,
                _ => false,
            };
            if !is_matching {
                tracing::info!(
                    command_id,
                    action = ?action,
                    "tui_stale_command_success_discarded"
                );
                return;
            }

            if action == ActionKind::SaveConfig || action == ActionKind::ReloadConfig {
                tracing::info!(command_id, "daemon_reload_succeeded");
            } else {
                tracing::info!(
                    command_id,
                    action = ?action,
                    response = %message,
                    "tui_command_succeeded"
                );
            }

            let display_message = match action {
                ActionKind::Suspend => String::from("Writes suspended"),
                ActionKind::Resume => String::from("Writes resumed"),
                ActionKind::RefreshWeather => String::from("Weather refresh requested"),
                ActionKind::SaveConfig | ActionKind::ReloadConfig => {
                    String::from("Settings saved and applied")
                }
            };
            model.action_state = ActionState::Success {
                action,
                message: display_message,
                completed_at: Instant::now(),
            };
        }
        IpcEvent::CommandFailed {
            command_id,
            action,
            error_category,
            message,
        } => {
            let is_matching = match model.action_state {
                ActionState::Pending {
                    command_id: pending_id,
                    ..
                } => pending_id == command_id,
                _ => false,
            };
            if !is_matching {
                tracing::info!(
                    command_id,
                    action = ?action,
                    "tui_stale_command_failure_discarded"
                );
                return;
            }

            if action == ActionKind::SaveConfig {
                tracing::warn!(
                    command_id,
                    category = ?error_category,
                    error = %message,
                    "daemon_reload_failed"
                );
                tracing::warn!(
                    command_id,
                    category = ?error_category,
                    "config_saved_daemon_unsynced"
                );

                let display_message = match error_category {
                    ErrorCategory::DaemonUnavailable => {
                        String::from("Saved to disk — daemon offline")
                    }
                    ErrorCategory::DaemonRejected => {
                        format!("Saved; daemon reload failed: {message}")
                    }
                    ErrorCategory::Timeout => String::from("Saved — daemon did not respond"),
                    _ => format!("Saved; reload error: {message}"),
                };

                model.action_state = ActionState::Warning {
                    action,
                    category: error_category,
                    message: display_message,
                    completed_at: Instant::now(),
                };
            } else {
                tracing::warn!(
                    command_id,
                    action = ?action,
                    category = ?error_category,
                    error = %message,
                    "tui_command_failed"
                );
                let display_message = match error_category {
                    ErrorCategory::DaemonUnavailable => {
                        format!("{}: daemon is offline", action.label())
                    }
                    ErrorCategory::DaemonRejected => format!("{}: {message}", action.label()),
                    ErrorCategory::Timeout => format!("{}: daemon timed out", action.label()),
                    ErrorCategory::Transport => {
                        format!("{}: transport error ({message})", action.label())
                    }
                    _ => message,
                };
                model.action_state = ActionState::Error {
                    action,
                    category: error_category,
                    message: display_message,
                    completed_at: Instant::now(),
                };
            }
        }
    }
}
