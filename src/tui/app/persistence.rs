use std::time::{Duration, Instant};

use crate::config::{self as app_config, Config};
use crate::ipc::Request;
use crate::tui::app::environment::load_timezone;
use crate::tui::globe::{rotation_duration, GlobeCenter};
use crate::tui::model::{ActionKind, ActionState, ErrorCategory, Model};
use crate::tui::worker::IpcCommand;

impl Model {
    pub fn send_command(&mut self, action: ActionKind, request: Request) -> u64 {
        self.next_command_id = self.next_command_id.wrapping_add(1);
        let command_id = self.next_command_id;
        let _ = self.ipc_tx.try_send(IpcCommand::Send {
            command_id,
            action,
            request,
        });
        command_id
    }

    pub fn check_debounced_save(&mut self) {
        if self.config_dirty {
            if let Some(mutation_time) = self.last_config_mutation {
                if mutation_time.elapsed() >= Duration::from_millis(400) {
                    self.save_config();
                }
            }
        }
    }

    pub fn check_action_state_expiration(&mut self) {
        let now = Instant::now();
        match &self.action_state {
            ActionState::Success { completed_at, .. } => {
                if now.duration_since(*completed_at) >= Duration::from_secs(3) {
                    self.action_state = ActionState::Idle;
                }
            }
            ActionState::Warning { completed_at, .. } => {
                if now.duration_since(*completed_at) >= Duration::from_secs(6) {
                    self.action_state = ActionState::Idle;
                }
            }
            ActionState::Error { completed_at, .. } => {
                if now.duration_since(*completed_at) >= Duration::from_secs(6) {
                    self.action_state = ActionState::Idle;
                }
            }
            ActionState::Pending {
                command_id,
                started_at,
                action,
                ..
            } => {
                if now.duration_since(*started_at) >= Duration::from_secs(8) {
                    if *action == ActionKind::SaveConfig {
                        tracing::warn!(
                            command_id = *command_id,
                            category = ?ErrorCategory::Timeout,
                            "daemon_reload_failed"
                        );
                        tracing::warn!(
                            command_id = *command_id,
                            category = ?ErrorCategory::Timeout,
                            "config_saved_daemon_unsynced"
                        );
                        self.action_state = ActionState::Warning {
                            action: *action,
                            category: ErrorCategory::Timeout,
                            message: String::from("Saved — daemon did not respond"),
                            completed_at: now,
                        };
                    } else {
                        self.action_state = ActionState::Error {
                            action: *action,
                            category: ErrorCategory::Timeout,
                            message: format!(
                                "{}: daemon did not respond (timeout)",
                                action.label()
                            ),
                            completed_at: now,
                        };
                    }
                }
            }
            ActionState::Idle => {}
        }
    }

    pub fn save_config(&mut self) -> bool {
        if let Err(error) = self.form.validate_values(&self.config) {
            tracing::warn!(error = %error, "validation_failed");
            self.config_error = Some(error.clone());
            self.config_dirty = true;
            self.last_config_mutation = None;
            self.action_state = ActionState::Error {
                action: ActionKind::SaveConfig,
                category: ErrorCategory::Validation,
                message: format!("Invalid value: {error}"),
                completed_at: Instant::now(),
            };
            return false;
        }

        let mut candidate = self.config.clone();
        self.form.apply_to_config(&mut candidate);

        self.commit_config(candidate, String::from("Saving configuration…"))
    }

    pub(crate) fn commit_config(&mut self, candidate: Config, description: String) -> bool {
        let previous_center = GlobeCenter::new(
            self.config.location.longitude,
            self.config.location.latitude,
        );
        let next_center =
            GlobeCenter::new(candidate.location.longitude, candidate.location.latitude);
        let location_changed = (previous_center.lon_deg - next_center.lon_deg).abs() > 1e-9
            || (previous_center.lat_deg - next_center.lat_deg).abs() > 1e-9;
        tracing::info!("config_write_started");
        let saved = match &self.config_save_path {
            Some(path) => app_config::save_to_path(&candidate, path),
            None => app_config::save(&candidate),
        };
        match saved {
            Ok(_) => {
                tracing::info!("config_write_succeeded");
                self.config = candidate;
                if location_changed {
                    self.motion.trigger(
                        crate::tui::motion::TransientKind::GlobeRotation {
                            from_lon: previous_center.lon_deg,
                            from_lat: previous_center.lat_deg,
                            to_lon: next_center.lon_deg,
                            to_lat: next_center.lat_deg,
                        },
                        Instant::now(),
                        rotation_duration(previous_center, next_center),
                    );
                }
                self.timezone_cache = load_timezone(&self.config.location.timezone);
                self.config_error = None;
                self.config_dirty = false;
                self.last_config_mutation = None;

                self.form.refresh_from_config(&self.config);
                self.request_preview_refresh();
                self.clamp_monitor_selection();
                let command_id = self.send_command(ActionKind::SaveConfig, Request::ReloadConfig);
                tracing::info!(command_id, "daemon_reload_requested");
                self.action_state = ActionState::Pending {
                    command_id,
                    action: ActionKind::SaveConfig,
                    description,
                    started_at: Instant::now(),
                };
                true
            }
            Err(error) => {
                tracing::error!(error = %error, "config_write_failed");
                self.config_error = Some(error.to_string());
                self.last_config_mutation = None;
                self.config_dirty = true;
                self.action_state = ActionState::Error {
                    action: ActionKind::SaveConfig,
                    category: ErrorCategory::ConfigWrite,
                    message: format!("Save failed: {error}"),
                    completed_at: Instant::now(),
                };
                false
            }
        }
    }

    pub fn suspend_writes(&mut self) {
        if matches!(
            self.action_state,
            ActionState::Pending {
                action: ActionKind::Suspend,
                ..
            }
        ) {
            return;
        }

        match self.form.suspend_duration_minutes() {
            Ok(minutes) => {
                let desc = match minutes {
                    Some(m) => format!("Suspending writes for {m}m…"),
                    None => String::from("Suspending writes until resume…"),
                };
                let command_id =
                    self.send_command(ActionKind::Suspend, Request::Suspend { minutes });
                self.action_state = ActionState::Pending {
                    command_id,
                    action: ActionKind::Suspend,
                    description: desc,
                    started_at: Instant::now(),
                };
            }
            Err(error) => {
                self.action_state = ActionState::Error {
                    action: ActionKind::Suspend,
                    category: ErrorCategory::Validation,
                    message: format!("Invalid value: {error}"),
                    completed_at: Instant::now(),
                };
            }
        }
    }

    pub fn resume_writes(&mut self) {
        if matches!(
            self.action_state,
            ActionState::Pending {
                action: ActionKind::Resume,
                ..
            }
        ) {
            return;
        }

        let command_id = self.send_command(ActionKind::Resume, Request::Resume);
        self.action_state = ActionState::Pending {
            command_id,
            action: ActionKind::Resume,
            description: String::from("Resuming daemon writes…"),
            started_at: Instant::now(),
        };
    }

    pub fn retry_weather(&mut self) {
        if matches!(
            self.action_state,
            ActionState::Pending {
                action: ActionKind::RefreshWeather,
                ..
            }
        ) {
            return;
        }

        let command_id = self.send_command(ActionKind::RefreshWeather, Request::RefreshWeather);
        self.action_state = ActionState::Pending {
            command_id,
            action: ActionKind::RefreshWeather,
            description: String::from("Refreshing weather…"),
            started_at: Instant::now(),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::tui::model::{ActionKind, ActionState, ErrorCategory};
    use crate::tui::update::{self, Message};
    use crate::tui::worker::IpcEvent;
    use crate::tui::Model;

    #[test]
    fn test_validation_error_does_not_invoke_ipc() {
        let mut model = Model::new();

        // Type invalid zero duration into suspend input
        model.form.suspend_minutes_input = tui_input::Input::new(String::from("0"));

        model.suspend_writes();

        // Action state reflects validation error
        assert!(matches!(
            model.action_state,
            ActionState::Error {
                action: ActionKind::Suspend,
                category: ErrorCategory::Validation,
                ..
            }
        ));
    }

    #[test]
    fn test_duplicate_command_suppression() {
        let mut model = Model::new();

        // First suspend sets Pending
        model.suspend_writes();
        assert!(matches!(
            model.action_state,
            ActionState::Pending {
                action: ActionKind::Suspend,
                ..
            }
        ));

        let initial_pending_time = match &model.action_state {
            ActionState::Pending { started_at, .. } => *started_at,
            _ => unreachable!(),
        };

        // Second suspend is suppressed
        model.suspend_writes();
        let second_pending_time = match &model.action_state {
            ActionState::Pending { started_at, .. } => *started_at,
            _ => unreachable!(),
        };

        assert_eq!(initial_pending_time, second_pending_time);
    }

    #[test]
    fn test_action_state_expiration() {
        let mut model = Model::new();

        // Success expires after 3 seconds
        model.action_state = ActionState::Success {
            action: ActionKind::Suspend,
            message: String::from("Done"),
            completed_at: Instant::now().checked_sub(Duration::from_secs(4)).unwrap(),
        };
        model.check_action_state_expiration();
        assert_eq!(model.action_state, ActionState::Idle);

        // Warning expires after 6 seconds
        model.action_state = ActionState::Warning {
            action: ActionKind::SaveConfig,
            category: ErrorCategory::DaemonUnavailable,
            message: String::from("Saved to disk — daemon offline"),
            completed_at: Instant::now().checked_sub(Duration::from_secs(7)).unwrap(),
        };
        model.check_action_state_expiration();
        assert_eq!(model.action_state, ActionState::Idle);

        // Error expires after 6 seconds
        model.action_state = ActionState::Error {
            action: ActionKind::Suspend,
            category: ErrorCategory::Transport,
            message: String::from("failed"),
            completed_at: Instant::now().checked_sub(Duration::from_secs(7)).unwrap(),
        };
        model.check_action_state_expiration();
        assert_eq!(model.action_state, ActionState::Idle);

        // Pending times out after 8 seconds
        model.action_state = ActionState::Pending {
            command_id: 1,
            action: ActionKind::Suspend,
            description: String::from("Waiting"),
            started_at: Instant::now().checked_sub(Duration::from_secs(9)).unwrap(),
        };
        model.check_action_state_expiration();
        assert!(matches!(
            model.action_state,
            ActionState::Error {
                action: ActionKind::Suspend,
                category: ErrorCategory::Timeout,
                ..
            }
        ));
    }

    #[test]
    fn test_save_config_validation_failure_surfaces_error_without_claiming_success() {
        let mut model = Model::new();
        model.config_dirty = true;

        // Invalid FPS (out of range 1..=120)
        model.form.fps_input = tui_input::Input::new(String::from("9999"));
        let saved = model.save_config();
        assert!(!saved);
        assert!(matches!(
            model.action_state,
            ActionState::Error {
                action: ActionKind::SaveConfig,
                category: ErrorCategory::Validation,
                ..
            }
        ));

        // Must not claim success or clear dirty flag
        assert!(model.config_dirty);
    }

    #[test]
    fn test_save_config_full_success_flow() {
        let mut model = Model::new();
        model.config_dirty = true;

        // Trigger save
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

        // Daemon acknowledges reload
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::CommandSucceeded {
                command_id,
                action: ActionKind::SaveConfig,
                message: String::from("reloaded config"),
            }),
        );

        match &model.action_state {
            ActionState::Success { message, .. } => {
                assert_eq!(message, "Settings saved and applied");
            }
            _ => panic!("expected Success action_state"),
        }
    }

    #[test]
    fn test_save_config_persisted_but_timeout_yields_warning() {
        let mut model = Model::new();
        let command_id =
            model.send_command(ActionKind::SaveConfig, crate::ipc::Request::ReloadConfig);

        model.action_state = ActionState::Pending {
            command_id,
            action: ActionKind::SaveConfig,
            description: String::from("Saving configuration…"),
            started_at: Instant::now().checked_sub(Duration::from_secs(9)).unwrap(),
        };

        model.check_action_state_expiration();

        // Must be Warning because config is already saved on disk
        match &model.action_state {
            ActionState::Warning {
                action,
                category,
                message,
                ..
            } => {
                assert_eq!(*action, ActionKind::SaveConfig);
                assert_eq!(*category, ErrorCategory::Timeout);
                assert_eq!(message, "Saved — daemon did not respond");
            }
            _ => panic!("expected Warning state for save_config timeout"),
        }
    }
}
