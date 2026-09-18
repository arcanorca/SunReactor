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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    use crate::tui::model::{ActionState, ActiveModal, ErrorCategory};
    use crate::tui::update::{self, Message};
    use crate::tui::{ui, InputMode, Model, Tab};

    #[test]
    fn test_editor_input_precedence_isolates_global_keys() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 3;
        model.form.timezone_input = tui_input::Input::default().with_value(String::from("UTC"));

        model.start_editing();
        assert_eq!(model.input_mode, InputMode::Editing);

        // Number keys switch tabs in Normal mode, but MUST be consumed as text in Editing mode!
        for digit in ['1', '2', '3', '4', '5'] {
            update::update(
                &mut model,
                Message::Key(KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE)),
            );
            // Tab must STILL be Location!
            assert_eq!(model.active_tab, Tab::Location);
        }
        assert_eq!(model.form.timezone_input.value(), "UTC12345");

        // 'q' must NOT quit SunReactor
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
        );
        assert_eq!(model.form.timezone_input.value(), "UTC12345q");

        // '?' must NOT open help modal
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE)),
        );
        assert!(matches!(model.active_modal, ActiveModal::None));

        // Up/Down must NOT navigate to different settings while editing
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        );
        assert_eq!(model.active_setting, 3);
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        );
        assert_eq!(model.active_setting, 3);

        // Esc cancels editing and restores original value
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        );
        assert_eq!(model.input_mode, InputMode::Normal);
        assert_eq!(model.form.timezone_input.value(), "UTC");
    }

    #[test]
    fn test_editor_validation_failure_keeps_editing_mode_with_dirty_input() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 1; // Latitude field
        model.form.lat_input = tui_input::Input::default().with_value(String::new());

        model.start_editing();

        // Type invalid latitude "999.0" (> 90.0)
        for c in "999.0".chars() {
            model.insert_char_to_active_input(c);
        }
        assert_eq!(model.form.lat_input.value(), "999.0");

        // Press Enter to commit
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        );

        // SUT requirement: on validation error, remain in Editing mode so user can correct it!
        assert_eq!(model.input_mode, InputMode::Editing);
        assert_eq!(model.form.lat_input.value(), "999.0");
        assert!(model.config_dirty);
        assert!(matches!(
            model.action_state,
            ActionState::Error {
                category: ErrorCategory::Validation,
                ..
            }
        ));
    }

    #[test]
    fn test_editor_responsive_rendering_across_viewports() {
        let dimensions = [(80, 24), (60, 20), (40, 16)];

        for (w, h) in dimensions {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut model = Model::new();
            model.active_tab = Tab::Location;
            model.active_setting = 3;
            model.form.timezone_input =
                tui_input::Input::default().with_value(String::from("Europe/Istanbul"));

            model.start_editing();
            assert_eq!(model.input_mode, InputMode::Editing);

            // Render editing state
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

            // Move cursor to middle and render
            model.move_cursor_left();
            model.move_cursor_left();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        }
    }
}
