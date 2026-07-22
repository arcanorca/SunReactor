use super::actions::{
    adjust_selected_monitor_milestone, reset_selected_monitor_milestone,
    select_next_monitor_milestone, select_previous_monitor_milestone, toggle_monitor_advanced,
};
use super::worker::IpcEvent;
use super::{ActiveInputKind, DaemonConnection, InputMode, Model, Tab};
use crossterm::event::{KeyCode, KeyEvent};

pub(crate) enum Message {
    Key(KeyEvent),
    Ipc(IpcEvent),
    Tick,
}

pub(crate) fn update(model: &mut Model, msg: Message) {
    match msg {
        Message::Key(key) => handle_key(key, model),
        Message::Ipc(event) => handle_ipc(event, model),
        Message::Tick => {
            model.check_debounced_save();
            model.refresh_monitor_milestones_if_needed();
        }
    }
}

fn handle_ipc(event: IpcEvent, model: &mut Model) {
    match event {
        IpcEvent::Status(status) => {
            let old_multiplier = model
                .status
                .as_ref()
                .and_then(|s| s.weather.as_ref())
                .and_then(|w| w.multiplier);
            let new_multiplier = status.weather.as_ref().and_then(|w| w.multiplier);

            model.status = Some(*status);

            if old_multiplier != new_multiplier {
                model.refresh_monitor_milestones();
            }
            model.daemon_connection = DaemonConnection::Connected;
        }
        IpcEvent::Connected => {
            model.daemon_connection = DaemonConnection::Connected;
        }
        IpcEvent::Disconnected => {
            model.daemon_connection = DaemonConnection::Disconnected;
            model.status = None;
        }
    }
}

fn handle_key(key: KeyEvent, app: &mut Model) {
    if !matches!(app.active_modal, super::model::ActiveModal::None) {
        match key.code {
            KeyCode::Up => app.theme_modal_up(),
            KeyCode::Down => app.theme_modal_down(),
            KeyCode::Enter => app.theme_modal_confirm(),
            KeyCode::Esc => app.theme_modal_cancel(),
            _ => {}
        }
        return;
    }

    if matches!(app.active_tab, Tab::Monitors)
        && !app.monitor_advanced_open
        && app.input_mode == InputMode::Normal
    {
        // 0-9 quick overrides were removed.
    }

    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => {
                if app.config_dirty {
                    app.save_config();
                }
                app.should_quit = true;
            }
            KeyCode::Char('?') => app.show_help = !app.show_help,
            KeyCode::Tab => {
                if app.monitor_advanced_open {
                    app.monitor_advanced_open = false;
                }
                if app.config_dirty {
                    app.save_config();
                }
                app.next_tab();
            }
            KeyCode::Right => {
                if matches!(app.active_tab, Tab::Monitors) && app.monitor_advanced_open {
                    adjust_selected_monitor_milestone(app, 1);
                    app.config_dirty = true;
                    app.last_config_mutation = Some(std::time::Instant::now());
                } else {
                    app.next_tab();
                }
            }
            KeyCode::Left => {
                if matches!(app.active_tab, Tab::Monitors) && app.monitor_advanced_open {
                    adjust_selected_monitor_milestone(app, -1);
                    app.config_dirty = true;
                    app.last_config_mutation = Some(std::time::Instant::now());
                } else {
                    app.previous_tab();
                }
            }
            KeyCode::Char('+') => {
                if matches!(app.active_tab, Tab::Monitors) {
                    app.increase_monitor_gamma();
                }
            }
            KeyCode::Char('-') => {
                if matches!(app.active_tab, Tab::Monitors) {
                    app.decrease_monitor_gamma();
                }
            }
            KeyCode::Down => {
                if matches!(app.active_tab, Tab::Monitors) && app.monitor_advanced_open {
                    select_next_monitor_milestone(app);
                } else {
                    app.move_selection_down();
                }
            }
            KeyCode::Up => {
                if matches!(app.active_tab, Tab::Monitors) && app.monitor_advanced_open {
                    select_previous_monitor_milestone(app);
                } else {
                    app.move_selection_up();
                }
            }
            KeyCode::Enter => {
                if app.active_input_kind() == Some(ActiveInputKind::Toggle) {
                    app.toggle_active_setting();
                    if matches!(app.active_modal, super::model::ActiveModal::None) {
                        app.save_config();
                    }
                } else {
                    app.start_editing();
                }
            }

            KeyCode::Esc => {
                if app.monitor_advanced_open {
                    app.monitor_advanced_open = false;
                }
                if app.config_dirty {
                    app.save_config();
                }
            }
            KeyCode::Char('a') => {
                if matches!(app.active_tab, Tab::Monitors) {
                    toggle_monitor_advanced(app);
                    if !app.monitor_advanced_open && app.config_dirty {
                        app.save_config();
                    }
                }
            }
            KeyCode::Char('s') => {
                if matches!(app.active_tab, Tab::Monitors | Tab::Settings) {
                    app.suspend_writes();
                }
            }
            KeyCode::Char('r') => {
                if matches!(app.active_tab, Tab::Monitors) && app.monitor_advanced_open {
                    reset_selected_monitor_milestone(app);
                    app.config_dirty = true;
                    app.last_config_mutation = Some(std::time::Instant::now());
                } else if matches!(app.active_tab, Tab::Monitors | Tab::Settings) {
                    app.resume_writes();
                }
            }
            _ => {}
        },
        InputMode::Editing => match key.code {
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
            KeyCode::Enter => {
                app.stop_editing();
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

                        app.save_config();
                    } else if app.form.city_search_input.value().trim().is_empty() {
                        app.form.lat_input =
                            tui_input::Input::default().with_value("0".to_string());
                        app.form.lon_input =
                            tui_input::Input::default().with_value("0".to_string());
                        app.form.timezone_input =
                            tui_input::Input::default().with_value("UTC".to_string());
                        app.form.city_search_results.clear();
                        app.form.city_search_selected_index = 0;
                        app.save_config();
                    } else {
                        app.form.refresh_from_config(&app.config);
                    }
                } else if matches!(app.active_tab, Tab::Settings) && app.active_setting == 4 {
                    app.suspend_writes();
                } else if matches!(app.active_tab, Tab::Monitors) {
                    // Monitor brightness input editing logic was removed.
                } else {
                    app.save_config();
                }
            }
            KeyCode::Esc => {
                app.stop_editing();
                if matches!(app.active_tab, Tab::Monitors) {
                    // Quick override input was removed
                } else {
                    app.form.refresh_from_config(&app.config);
                }
            }
            KeyCode::Char(c) if accepts_char(app.active_input_kind(), c) => {
                app.append_char_to_active_input(c);
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
            _ => {}
        },
    }
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
