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

#[cfg(test)]
mod tests {
    use crate::tui::model::{ActionKind, ActionState, ErrorCategory};
    use crate::tui::test_support::dummy_status;
    use crate::tui::update::{self, Message};
    use crate::tui::worker::IpcEvent;
    use crate::tui::Model;

    #[test]
    fn test_action_state_suspend_flow_success() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));

        // User triggers suspend
        model.suspend_writes();
        let ActionState::Pending {
            command_id,
            action: ActionKind::Suspend,
            ..
        } = model.action_state
        else {
            panic!("expected pending suspend");
        };

        // Daemon acknowledges
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandSucceeded {
                command_id,
                action: ActionKind::Suspend,
                message: String::from("suspended writes for 10 minutes"),
            }),
        );

        assert!(matches!(
            model.action_state,
            ActionState::Success {
                action: ActionKind::Suspend,
                ..
            }
        ));

        // Authoritative status arrives from daemon
        let mut updated_status = dummy_status(2);
        updated_status.suspended = true;
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(updated_status))),
        );

        assert!(model.status.as_ref().unwrap().suspended);
    }

    #[test]
    fn test_action_state_resume_flow_success() {
        let mut model = Model::new();
        let mut status = dummy_status(2);
        status.suspended = true;
        model.status = Some(status);

        // User triggers resume
        model.resume_writes();
        let ActionState::Pending {
            command_id,
            action: ActionKind::Resume,
            ..
        } = model.action_state
        else {
            panic!("expected pending resume");
        };

        // Daemon acknowledges
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandSucceeded {
                command_id,
                action: ActionKind::Resume,
                message: String::from("resumed automatic apply"),
            }),
        );

        assert!(matches!(
            model.action_state,
            ActionState::Success {
                action: ActionKind::Resume,
                ..
            }
        ));

        // Authoritative status arrives from daemon
        let mut updated_status = dummy_status(2);
        updated_status.suspended = false;
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(updated_status))),
        );

        assert!(!model.status.as_ref().unwrap().suspended);
    }

    #[test]
    fn test_action_state_daemon_rejection_leaves_authoritative_state_intact() {
        let mut model = Model::new();
        let initial_status = dummy_status(2);
        model.status = Some(initial_status.clone());

        model.suspend_writes();
        let ActionState::Pending {
            command_id,
            action: ActionKind::Suspend,
            ..
        } = model.action_state
        else {
            panic!("expected pending suspend");
        };

        // Daemon rejects operation
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandFailed {
                command_id,
                action: ActionKind::Suspend,
                error_category: ErrorCategory::DaemonRejected,
                message: String::from("permission denied"),
            }),
        );

        assert!(matches!(
            model.action_state,
            ActionState::Error {
                action: ActionKind::Suspend,
                category: ErrorCategory::DaemonRejected,
                ..
            }
        ));

        // Authoritative state was NOT falsely modified to suspended!
        assert!(!model.status.as_ref().unwrap().suspended);
        assert_eq!(model.status, Some(initial_status));
    }

    #[test]
    fn test_action_state_daemon_unavailable() {
        let mut model = Model::new();

        model.resume_writes();
        let ActionState::Pending {
            command_id,
            action: ActionKind::Resume,
            ..
        } = model.action_state
        else {
            panic!("expected pending resume");
        };

        // Daemon is offline / socket unavailable
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandFailed {
                command_id,
                action: ActionKind::Resume,
                error_category: ErrorCategory::DaemonUnavailable,
                message: String::from("daemon socket not found"),
            }),
        );

        assert!(matches!(
            model.action_state,
            ActionState::Error {
                action: ActionKind::Resume,
                category: ErrorCategory::DaemonUnavailable,
                ..
            }
        ));
    }

    #[test]
    fn test_save_config_persisted_but_daemon_unavailable_yields_warning() {
        let mut model = Model::new();
        model.config_dirty = true;

        let saved = model.save_config();
        assert!(saved);
        assert!(!model.config_dirty);

        let ActionState::Pending {
            command_id,
            action: ActionKind::SaveConfig,
            ..
        } = model.action_state
        else {
            panic!("expected pending save_config");
        };

        // Daemon is offline / unavailable
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandFailed {
                command_id,
                action: ActionKind::SaveConfig,
                error_category: ErrorCategory::DaemonUnavailable,
                message: String::from("daemon socket not found"),
            }),
        );

        // Must be Warning (partial success), NOT Error!
        match &model.action_state {
            ActionState::Warning {
                action,
                category,
                message,
                ..
            } => {
                assert_eq!(*action, ActionKind::SaveConfig);
                assert_eq!(*category, ErrorCategory::DaemonUnavailable);
                assert_eq!(message, "Saved to disk — daemon offline");
            }
            _ => panic!("expected Warning state for persisted config with offline daemon"),
        }
    }

    #[test]
    fn test_save_config_persisted_but_daemon_rejected_yields_warning() {
        let mut model = Model::new();
        model.config_dirty = true;

        let saved = model.save_config();
        assert!(saved);

        let ActionState::Pending {
            command_id,
            action: ActionKind::SaveConfig,
            ..
        } = model.action_state
        else {
            panic!("expected pending save_config");
        };

        // Daemon rejects reload
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandFailed {
                command_id,
                action: ActionKind::SaveConfig,
                error_category: ErrorCategory::DaemonRejected,
                message: String::from("failed to apply monitors"),
            }),
        );

        match &model.action_state {
            ActionState::Warning {
                action,
                category,
                message,
                ..
            } => {
                assert_eq!(*action, ActionKind::SaveConfig);
                assert_eq!(*category, ErrorCategory::DaemonRejected);
                assert!(message.contains("Saved; daemon reload failed"));
            }
            _ => panic!("expected Warning state for persisted config with rejected reload"),
        }
    }

    #[test]
    fn test_command_id_correlation_discards_stale_and_superseded_completions() {
        let mut model = Model::new();

        // Command 1: Suspend
        model.suspend_writes();
        let ActionState::Pending {
            command_id: cmd1_id,
            ..
        } = model.action_state
        else {
            panic!("expected pending");
        };

        // Command 2: Resume (supersedes Command 1)
        model.resume_writes();
        let ActionState::Pending {
            command_id: cmd2_id,
            ..
        } = model.action_state
        else {
            panic!("expected pending");
        };

        assert_ne!(cmd1_id, cmd2_id);

        // Late completion for Command 1 arrives
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandSucceeded {
                command_id: cmd1_id,
                action: ActionKind::Suspend,
                message: String::from("suspended"),
            }),
        );

        // Command 2 must STILL be pending!
        match model.action_state {
            ActionState::Pending {
                command_id,
                action: ActionKind::Resume,
                ..
            } => {
                assert_eq!(command_id, cmd2_id);
            }
            _ => panic!("Command 2 pending state was overwritten by late Command 1 completion!"),
        }

        // Now Command 2 completion arrives
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandSucceeded {
                command_id: cmd2_id,
                action: ActionKind::Resume,
                message: String::from("resumed"),
            }),
        );

        // Now state transitions to Success for Command 2
        match model.action_state {
            ActionState::Success {
                action: ActionKind::Resume,
                ..
            } => {}
            _ => panic!("expected Success for Command 2"),
        }
    }
}
