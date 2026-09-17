use std::time::Instant;

use super::actions::{
    adjust_selected_monitor_milestone, reset_selected_monitor_milestone,
    select_next_monitor_milestone, select_previous_monitor_milestone,
};
use super::input::{map_normal_key, UiAction};
use super::model::{settings_index, ActionKind, ActionState, ErrorCategory, MonitorDiscoveryState};
use super::worker::IpcEvent;
use super::{ActiveInputKind, DaemonConnection, InputMode, Model, Tab};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) enum Message {
    Key(KeyEvent),
    Ipc(IpcEvent),
    Resize(u16, u16),
    Tick,
}

pub(crate) fn update(model: &mut Model, msg: Message) {
    match msg {
        Message::Key(key) => handle_key(key, model),
        Message::Ipc(event) => handle_ipc(event, model),
        Message::Resize(cols, rows) => model.handle_resize(cols, rows),
        Message::Tick => {
            model.check_action_state_expiration();
            model.check_debounced_save();
            model.poll_preview_refresh();
            model.refresh_monitor_milestones_if_needed();
            model.motion.tick(std::time::Instant::now());
        }
    }
}

#[allow(clippy::too_many_lines)]
fn handle_ipc(event: IpcEvent, model: &mut Model) {
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
                                super::motion::TransientKind::WeatherUpdate,
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

#[allow(clippy::too_many_lines)]
fn handle_key(key: KeyEvent, app: &mut Model) {
    if key.kind == crossterm::event::KeyEventKind::Release {
        return;
    }

    if key
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c' | 'C'))
    {
        if app.config_dirty && !app.save_config() {
            return;
        }
        app.should_quit = true;
        return;
    }

    if !matches!(app.active_modal, super::model::ActiveModal::None) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.theme_modal_up(),
            KeyCode::Down | KeyCode::Char('j') => app.theme_modal_down(),
            KeyCode::Enter => app.theme_modal_confirm(),
            KeyCode::Esc => app.theme_modal_cancel(),
            _ => {}
        }
        return;
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?' | 'q') => {
                app.show_help = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.scroll_help_up(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.scroll_help_down(1, 24);
            }
            KeyCode::PageUp => {
                app.scroll_help_up(6);
            }
            KeyCode::PageDown => {
                app.scroll_help_down(6, 24);
            }
            KeyCode::Home => {
                app.scroll_help_home();
            }
            KeyCode::End => {
                app.scroll_help_end(24);
            }
            _ => {}
        }
        return;
    }

    match app.input_mode {
        InputMode::Normal => {
            if let Some(action) = map_normal_key(key) {
                dispatch_normal_action(action, app);
            }
        }
        InputMode::Editing => {
            let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

            match key.code {
                KeyCode::Up => {
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_selected_index =
                            app.form.city_search_selected_index.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        let len = app.form.city_search_results.len();
                        if len > 0 {
                            app.form.city_search_selected_index =
                                (app.form.city_search_selected_index + 1).min(len - 1);
                        }
                    }
                }
                KeyCode::Left if has_ctrl => {
                    app.move_cursor_prev_word();
                }
                KeyCode::Right if has_ctrl => {
                    app.move_cursor_next_word();
                }
                KeyCode::Left => {
                    app.move_cursor_left();
                }
                KeyCode::Right => {
                    app.move_cursor_right();
                }
                KeyCode::Home => {
                    app.move_cursor_start();
                }
                KeyCode::End => {
                    app.move_cursor_end();
                }
                KeyCode::Char('a') if has_ctrl => {
                    app.move_cursor_start();
                }
                KeyCode::Char('e') if has_ctrl => {
                    app.move_cursor_end();
                }
                KeyCode::Char('w') if has_ctrl => {
                    app.delete_prev_word_active_input();
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_results =
                            crate::tui::cities::search_cities(app.form.city_search_input.value());
                        app.form.city_search_selected_index = 0;
                    }
                }
                KeyCode::Char('u') if has_ctrl => {
                    app.handle_input_request(tui_input::InputRequest::DeleteLine);
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_results =
                            crate::tui::cities::search_cities(app.form.city_search_input.value());
                        app.form.city_search_selected_index = 0;
                    }
                }
                KeyCode::Char('k') if has_ctrl => {
                    app.handle_input_request(tui_input::InputRequest::DeleteTillEnd);
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_results =
                            crate::tui::cities::search_cities(app.form.city_search_input.value());
                        app.form.city_search_selected_index = 0;
                    }
                }
                KeyCode::Backspace => {
                    app.backspace_active_input();
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_results =
                            crate::tui::cities::search_cities(app.form.city_search_input.value());
                        app.form.city_search_selected_index = 0;
                    }
                }
                KeyCode::Delete => {
                    app.delete_active_input();
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_results =
                            crate::tui::cities::search_cities(app.form.city_search_input.value());
                        app.form.city_search_selected_index = 0;
                    }
                }
                KeyCode::Char(c) if !has_ctrl && accepts_char(app.active_input_kind(), c) => {
                    app.insert_char_to_active_input(c);
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        app.form.city_search_results =
                            crate::tui::cities::search_cities(app.form.city_search_input.value());
                        app.form.city_search_selected_index = 0;
                    }
                }
                KeyCode::Enter => {
                    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
                        if let Some(&idx) = app
                            .form
                            .city_search_results
                            .get(app.form.city_search_selected_index)
                        {
                            let city = &crate::tui::cities::get_cities()[idx];
                            app.form.lat_input =
                                tui_input::Input::default().with_value(city.lat.to_string());
                            app.form.lon_input =
                                tui_input::Input::default().with_value(city.lon.to_string());
                            app.form.timezone_input =
                                tui_input::Input::default().with_value(city.timezone.clone());
                            app.form.city_search_input = tui_input::Input::default()
                                .with_value(format!("{}, {}", city.name, city.country));
                            app.form.city_search_results.clear();
                            app.form.city_search_selected_index = 0;
                            app.status = None;

                            if app.save_config() {
                                app.stop_editing();
                            }
                        } else {
                            // If empty or no match, restore committed value rather than wiping coordinates
                            app.cancel_editing();
                        }
                    } else if matches!(app.active_tab, Tab::Settings)
                        && app.active_setting == settings_index::SUSPEND_DURATION
                    {
                        app.suspend_writes();
                        app.stop_editing();
                    } else if matches!(app.active_tab, Tab::Monitors) {
                        let is_min = app.monitor_control_index == 0;
                        if app.save_config() {
                            app.motion.trigger(
                                crate::tui::motion::TransientKind::RangeCommit { min: is_min },
                                std::time::Instant::now(),
                                std::time::Duration::from_millis(350),
                            );
                            app.stop_editing();
                        }
                    } else if app.save_config() {
                        app.stop_editing();
                    }
                }
                KeyCode::Esc => {
                    app.cancel_editing();
                }
                _ => {}
            }
        }
    }
}

fn dispatch_normal_action(action: UiAction, app: &mut Model) {
    use super::model::AutomationRegionFocus;

    // On the tab bar the workspaces form a horizontal row: ←/→ move along it
    // and ↓, Enter, or Esc step down into the workspace.
    if app.tabs_focused {
        match action {
            UiAction::MoveLeft => return app.previous_tab(),
            UiAction::MoveRight => return app.next_tab(),
            UiAction::MoveDown | UiAction::Activate | UiAction::Back => {
                app.tabs_focused = false;
                // Stepping down from the tabs lands on the workspace's first
                // region, never on a control reached earlier from the side.
                if app.active_tab == Tab::Monitors {
                    app.monitor_pane_focus = super::model::MonitorPaneFocus::List;
                }
                return;
            }
            UiAction::MoveUp | UiAction::Toggle => return,
            _ => {}
        }
    }

    match action {
        UiAction::SelectTab(tab) => app.switch_to_tab(tab),
        UiAction::NextTab => app.next_tab(),
        UiAction::PreviousTab => app.previous_tab(),
        UiAction::Quit => {
            if !app.config_dirty || app.save_config() {
                app.should_quit = true;
            }
        }
        UiAction::ToggleHelp => {
            app.show_help = !app.show_help;
            app.help_scroll = 0;
        }
        UiAction::MoveLeft => move_horizontal(app, -1),
        UiAction::MoveRight => move_horizontal(app, 1),
        UiAction::MoveUp => move_vertical(app, -1),
        UiAction::MoveDown => move_vertical(app, 1),
        UiAction::Activate => activate_focused_control(app),
        UiAction::Back => back_from_focused_control(app),
        UiAction::Toggle => toggle_focused_control(app),
        UiAction::PageUp => app.page_up(),
        UiAction::PageDown => app.page_down(),
        UiAction::MoveFirst => app.move_to_first(),
        UiAction::MoveLast => app.move_to_last(),
        UiAction::NextMonitor => {
            if matches!(app.active_tab, Tab::Monitors | Tab::Limits) {
                app.select_next_monitor();
            }
        }
        UiAction::PreviousMonitor => {
            if matches!(app.active_tab, Tab::Monitors | Tab::Limits) {
                app.select_previous_monitor();
            }
        }
        UiAction::OpenAutomation => {
            if matches!(app.active_tab, Tab::Monitors) {
                app.switch_to_tab(Tab::Limits);
                app.tabs_focused = false;
                app.automation_focus = AutomationRegionFocus::Curve;
            }
        }
        UiAction::Suspend => {
            if matches!(app.active_tab, Tab::Monitors | Tab::Limits | Tab::Settings) {
                app.suspend_writes();
            }
        }
        UiAction::ContextualReset => match app.active_tab {
            Tab::Limits if matches!(app.automation_focus, AutomationRegionFocus::Milestones) => {
                reset_selected_monitor_milestone(app);
                mark_milestone_adjustment(app);
            }
            Tab::Monitors | Tab::Settings => app.resume_writes(),
            Tab::Weather => app.retry_weather(),
            Tab::Limits | Tab::Location => {}
        },
        UiAction::RetryWeather => {
            if matches!(app.active_tab, Tab::Weather) {
                app.retry_weather();
            }
        }
    }
}

fn select_monitor_step(app: &mut Model, delta: i8) {
    if delta > 0 {
        app.select_next_monitor();
    } else {
        app.select_previous_monitor();
    }
}

/// ←/→ act on horizontal arrangements: the narrow monitor switcher, the
/// Automation monitor chips, value steppers, and time offsets.
fn move_horizontal(app: &mut Model, delta: i8) {
    use super::model::{AutomationRegionFocus, MonitorPaneFocus};

    match app.active_tab {
        Tab::Monitors => match app.monitor_pane_focus {
            MonitorPaneFocus::List if app.monitor_selector_horizontal => {
                select_monitor_step(app, delta);
            }
            // The vertical list sits left of the detail pane: → steps into it.
            MonitorPaneFocus::List if delta > 0 && app.monitors_count() > 0 => {
                app.monitor_pane_focus = MonitorPaneFocus::Detail;
                app.monitor_control_index = 0;
            }
            MonitorPaneFocus::List => {}
            MonitorPaneFocus::Detail => app.step_monitor_range(delta),
        },
        Tab::Limits => match app.automation_focus {
            AutomationRegionFocus::Selector => select_monitor_step(app, delta),
            AutomationRegionFocus::Curve => app.step_monitor_curve(f64::from(delta) * 0.05),
            AutomationRegionFocus::Milestones => {
                adjust_selected_monitor_milestone(app, i16::from(delta));
                mark_milestone_adjustment(app);
            }
        },
        Tab::Settings if app.active_input_kind() == Some(ActiveInputKind::Toggle) => {
            if delta < 0 {
                app.cycle_active_setting_backward();
            } else {
                app.toggle_active_setting();
            }
            save_toggled_settings(app);
        }
        Tab::Location | Tab::Weather | Tab::Settings => {}
    }
}

/// ↑/↓ act on vertical arrangements. ↑ from the first element of any
/// workspace reaches the tab bar.
fn move_vertical(app: &mut Model, delta: i8) {
    use super::model::{AutomationRegionFocus, MonitorPaneFocus};

    match app.active_tab {
        Tab::Monitors => match app.monitor_pane_focus {
            MonitorPaneFocus::List if app.monitor_selector_horizontal => {
                if delta > 0 && app.monitors_count() > 0 {
                    app.monitor_pane_focus = MonitorPaneFocus::Detail;
                    app.monitor_control_index = 0;
                } else if delta < 0 {
                    app.tabs_focused = true;
                }
            }
            MonitorPaneFocus::List if delta > 0 => app.move_selection_down(),
            MonitorPaneFocus::List if app.selected_monitor == 0 => app.tabs_focused = true,
            MonitorPaneFocus::List => app.move_selection_up(),
            MonitorPaneFocus::Detail if delta > 0 => {
                app.monitor_control_index = (app.monitor_control_index + 1).min(1);
            }
            MonitorPaneFocus::Detail if app.monitor_control_index == 0 => {
                if app.monitor_selector_horizontal {
                    app.monitor_pane_focus = MonitorPaneFocus::List;
                } else {
                    app.tabs_focused = true;
                }
            }
            MonitorPaneFocus::Detail => {
                app.monitor_control_index = app.monitor_control_index.saturating_sub(1);
            }
        },
        Tab::Limits => match (app.automation_focus, delta) {
            (AutomationRegionFocus::Selector, 1) => {
                app.automation_focus = AutomationRegionFocus::Curve;
            }
            (AutomationRegionFocus::Selector, _) => app.tabs_focused = true,
            (AutomationRegionFocus::Curve, 1) => {
                app.automation_focus = AutomationRegionFocus::Milestones;
                app.selected_monitor_milestone = 0;
            }
            (AutomationRegionFocus::Curve, _) if app.monitors_count() > 1 => {
                app.automation_focus = AutomationRegionFocus::Selector;
            }
            (AutomationRegionFocus::Curve, _) => app.tabs_focused = true,
            (AutomationRegionFocus::Milestones, 1) => select_next_monitor_milestone(app),
            (AutomationRegionFocus::Milestones, _) if app.selected_monitor_milestone == 0 => {
                app.automation_focus = AutomationRegionFocus::Curve;
            }
            (AutomationRegionFocus::Milestones, _) => select_previous_monitor_milestone(app),
        },
        _ if delta > 0 => app.move_selection_down(),
        _ if app.active_setting == 0 => app.tabs_focused = true,
        _ => app.move_selection_up(),
    }
}

fn activate_focused_control(app: &mut Model) {
    use super::model::{AutomationRegionFocus, MonitorPaneFocus};

    match app.active_tab {
        Tab::Monitors if matches!(app.monitor_pane_focus, MonitorPaneFocus::List) => {
            if app.monitors_count() == 0 {
                app.import_discovered_monitors();
            } else {
                app.monitor_pane_focus = MonitorPaneFocus::Detail;
                app.monitor_control_index = 0;
            }
        }
        Tab::Limits if app.monitors_count() == 0 => app.import_discovered_monitors(),
        Tab::Limits if matches!(app.automation_focus, AutomationRegionFocus::Selector) => {
            app.automation_focus = AutomationRegionFocus::Curve;
        }
        Tab::Limits if matches!(app.automation_focus, AutomationRegionFocus::Curve) => {
            app.start_editing();
        }
        Tab::Limits => {}
        Tab::Weather => app.retry_weather(),
        _ if app.active_input_kind() == Some(ActiveInputKind::Toggle) => {
            app.toggle_active_setting();
            save_toggled_settings(app);
        }
        _ => app.start_editing(),
    }
}

/// Esc steps out one level: detail → list, schedule → curve → monitor chips,
/// and from the top of any workspace to the tab bar.
fn back_from_focused_control(app: &mut Model) {
    use super::model::{AutomationRegionFocus, MonitorPaneFocus};

    match app.active_tab {
        Tab::Monitors if matches!(app.monitor_pane_focus, MonitorPaneFocus::Detail) => {
            app.monitor_pane_focus = MonitorPaneFocus::List;
        }
        Tab::Limits if matches!(app.automation_focus, AutomationRegionFocus::Milestones) => {
            app.automation_focus = AutomationRegionFocus::Curve;
        }
        Tab::Limits
            if matches!(app.automation_focus, AutomationRegionFocus::Curve)
                && app.monitors_count() > 1 =>
        {
            app.automation_focus = AutomationRegionFocus::Selector;
        }
        _ => app.tabs_focused = true,
    }
}

fn toggle_focused_control(app: &mut Model) {
    if matches!(app.active_tab, Tab::Limits) {
        if app.status.as_ref().is_some_and(|status| status.suspended) {
            app.resume_writes();
        } else {
            app.suspend_writes();
        }
    } else if app.active_input_kind() == Some(ActiveInputKind::Toggle) {
        app.toggle_active_setting();
        save_toggled_settings(app);
    } else if matches!(app.active_tab, Tab::Weather) {
        app.retry_weather();
    }
}

fn save_toggled_settings(app: &mut Model) {
    if matches!(app.active_modal, super::model::ActiveModal::None) {
        let _ = app.save_config();
    }
}

fn mark_milestone_adjustment(app: &mut Model) {
    app.config_dirty = true;
    app.last_config_mutation = Some(std::time::Instant::now());
    app.motion.trigger(
        crate::tui::motion::TransientKind::MilestoneAdjust {
            index: app.selected_monitor_milestone,
        },
        std::time::Instant::now(),
        std::time::Duration::from_millis(250),
    );
}

fn accepts_char(kind: Option<ActiveInputKind>, c: char) -> bool {
    match kind {
        Some(ActiveInputKind::Decimal) => c.is_ascii_digit() || c == '.' || c == '-',
        Some(ActiveInputKind::Integer) => c.is_ascii_digit(),
        Some(ActiveInputKind::Time) => c.is_ascii_digit() || c == ':',
        Some(ActiveInputKind::Text | ActiveInputKind::Secret) => !c.is_control(),
        Some(ActiveInputKind::Toggle) => false,
        None => false,
    }
}
