use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::model::settings_index;
use crate::tui::{ActiveInputKind, Model, Tab};

#[allow(clippy::too_many_lines)]
pub(super) fn handle_editing_key(key: KeyEvent, app: &mut Model) {
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
            update_city_search_if_location(app);
        }
        KeyCode::Char('u') if has_ctrl => {
            app.handle_input_request(tui_input::InputRequest::DeleteLine);
            update_city_search_if_location(app);
        }
        KeyCode::Char('k') if has_ctrl => {
            app.handle_input_request(tui_input::InputRequest::DeleteTillEnd);
            update_city_search_if_location(app);
        }
        KeyCode::Backspace => {
            app.backspace_active_input();
            update_city_search_if_location(app);
        }
        KeyCode::Delete => {
            app.delete_active_input();
            update_city_search_if_location(app);
        }
        KeyCode::Char(c) if !has_ctrl && accepts_char(app.active_input_kind(), c) => {
            app.insert_char_to_active_input(c);
            update_city_search_if_location(app);
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

fn update_city_search_if_location(app: &mut Model) {
    if matches!(app.active_tab, Tab::Location) && app.active_setting == 0 {
        app.form.city_search_results =
            crate::tui::cities::search_cities(app.form.city_search_input.value());
        app.form.city_search_selected_index = 0;
    }
}

fn accepts_char(kind: Option<ActiveInputKind>, c: char) -> bool {
    match kind {
        Some(ActiveInputKind::Decimal) => c.is_ascii_digit() || c == '.' || c == '-',
        Some(ActiveInputKind::Integer) => c.is_ascii_digit(),
        Some(ActiveInputKind::Time) => c.is_ascii_digit() || c == ':',
        Some(ActiveInputKind::Text | ActiveInputKind::Secret) => !c.is_control(),
        Some(ActiveInputKind::Toggle) | None => false,
    }
}
