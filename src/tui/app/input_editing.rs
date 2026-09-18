use crate::tui::model::{
    settings_index, ActiveInputKind, AutomationRegionFocus, InputMode, Model, MonitorPaneFocus, Tab,
};

/// Policy for initializing text input buffers when entering editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditBufferPolicy {
    /// Buffer retains current value and places cursor at the end.
    ExistingValue,
    /// Buffer is cleared for clean replacement (original preserved in snapshot for Esc).
    CleanReplacement,
}

impl Model {
    /// Determines the input initialization policy for the currently focused field.
    #[must_use]
    pub fn edit_buffer_policy(&self) -> EditBufferPolicy {
        if (matches!(self.active_tab, Tab::Location) && self.active_setting == 0)
            || (matches!(self.active_tab, Tab::Settings)
                && self.active_setting == settings_index::WEATHER_API_KEY)
        {
            EditBufferPolicy::CleanReplacement
        } else {
            EditBufferPolicy::ExistingValue
        }
    }

    pub fn start_editing(&mut self) {
        if matches!(self.active_tab, Tab::Monitors)
            && !matches!(self.monitor_pane_focus, MonitorPaneFocus::Detail)
        {
            return;
        }
        self.editing_form_snapshot = Some(self.form.clone());
        self.input_mode = InputMode::Editing;

        match self.edit_buffer_policy() {
            EditBufferPolicy::CleanReplacement => {
                if matches!(self.active_tab, Tab::Location) && self.active_setting == 0 {
                    self.form.city_search_input = tui_input::Input::default();
                    self.form.city_search_results.clear();
                    self.form.city_search_selected_index = 0;
                } else if matches!(self.active_tab, Tab::Settings)
                    && self.active_setting == settings_index::WEATHER_API_KEY
                {
                    self.form.api_key_input = tui_input::Input::default();
                }
            }
            EditBufferPolicy::ExistingValue => {
                if let Some(input) = self.active_input_mut() {
                    let len = input.value().chars().count();
                    input.handle(tui_input::InputRequest::SetCursor(len));
                }
            }
        }
    }

    pub fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
        self.editing_form_snapshot = None;
    }

    pub fn cancel_editing(&mut self) {
        if let Some(snapshot) = self.editing_form_snapshot.take() {
            self.form = snapshot;
        }
        self.config_error = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn handle_input_request(&mut self, req: tui_input::InputRequest) -> bool {
        if let Some(input) = self.active_input_mut() {
            input.handle(req).is_some()
        } else {
            false
        }
    }

    pub fn insert_char_to_active_input(&mut self, c: char) {
        self.handle_input_request(tui_input::InputRequest::InsertChar(c));
    }

    pub fn backspace_active_input(&mut self) {
        self.handle_input_request(tui_input::InputRequest::DeletePrevChar);
    }

    pub fn delete_active_input(&mut self) {
        self.handle_input_request(tui_input::InputRequest::DeleteNextChar);
    }

    pub fn move_cursor_left(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToPrevChar);
    }

    pub fn move_cursor_right(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToNextChar);
    }

    pub fn move_cursor_start(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToStart);
    }

    pub fn move_cursor_end(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToEnd);
    }

    pub fn delete_prev_word_active_input(&mut self) {
        self.handle_input_request(tui_input::InputRequest::DeletePrevWord);
    }

    pub fn move_cursor_prev_word(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToPrevWord);
    }

    pub fn move_cursor_next_word(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToNextWord);
    }

    #[must_use]
    pub fn active_input_kind(&self) -> Option<ActiveInputKind> {
        if matches!(self.active_tab, Tab::Monitors) {
            if matches!(self.monitor_pane_focus, MonitorPaneFocus::Detail) {
                match self.monitor_control_index {
                    0 | 1 => Some(ActiveInputKind::Integer),
                    _ => None,
                }
            } else {
                None
            }
        } else if matches!(self.active_tab, Tab::Limits) {
            if matches!(self.automation_focus, AutomationRegionFocus::Curve) {
                Some(ActiveInputKind::Decimal)
            } else {
                None
            }
        } else {
            self.form
                .active_input_kind(self.active_tab, self.active_setting)
        }
    }

    #[must_use]
    pub fn active_input_ref(&self) -> Option<&tui_input::Input> {
        if matches!(self.active_tab, Tab::Monitors) {
            if matches!(self.monitor_pane_focus, MonitorPaneFocus::Detail) {
                let pair = self
                    .selected_config_monitor_index()
                    .and_then(|index| self.form.monitor_inputs.get(index));
                match self.monitor_control_index {
                    0 => pair.map(|pair| &pair.0),
                    1 => pair.map(|pair| &pair.1),
                    _ => None,
                }
            } else {
                None
            }
        } else if matches!(self.active_tab, Tab::Limits) {
            if matches!(self.automation_focus, AutomationRegionFocus::Curve) {
                self.selected_config_monitor_index()
                    .and_then(|index| self.form.monitor_curve_inputs.get(index))
            } else {
                None
            }
        } else {
            self.form
                .active_input_ref(self.active_tab, self.active_setting)
        }
    }

    pub fn active_input_mut(&mut self) -> Option<&mut tui_input::Input> {
        if matches!(self.active_tab, Tab::Monitors) {
            if matches!(self.monitor_pane_focus, MonitorPaneFocus::Detail) {
                let control_index = self.monitor_control_index;
                let monitor_index = self.selected_config_monitor_index()?;
                let pair = self.form.monitor_inputs.get_mut(monitor_index);
                match control_index {
                    0 => pair.map(|pair| &mut pair.0),
                    1 => pair.map(|pair| &mut pair.1),
                    _ => None,
                }
            } else {
                None
            }
        } else if matches!(self.active_tab, Tab::Limits) {
            if matches!(self.automation_focus, AutomationRegionFocus::Curve) {
                let monitor_index = self.selected_config_monitor_index()?;
                self.form.monitor_curve_inputs.get_mut(monitor_index)
            } else {
                None
            }
        } else {
            self.form
                .active_input_mut(self.active_tab, self.active_setting)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_buffer_policy_preserves_numeric_values_and_clears_secrets() {
        let mut model = Model::new();

        model.active_tab = Tab::Location;
        model.active_setting = 0;
        assert_eq!(
            model.edit_buffer_policy(),
            EditBufferPolicy::CleanReplacement
        );

        model.active_tab = Tab::Settings;
        model.active_setting = settings_index::WEATHER_API_KEY;
        assert_eq!(
            model.edit_buffer_policy(),
            EditBufferPolicy::CleanReplacement
        );

        model.active_setting = settings_index::REFRESH_RATE;
        assert_eq!(model.edit_buffer_policy(), EditBufferPolicy::ExistingValue);

        model.active_setting = settings_index::IDLE_DIM;
        assert_eq!(model.edit_buffer_policy(), EditBufferPolicy::ExistingValue);

        model.active_tab = Tab::Location;
        model.active_setting = 1;
        assert_eq!(model.edit_buffer_policy(), EditBufferPolicy::ExistingValue);

        model.active_setting = 3;
        assert_eq!(model.edit_buffer_policy(), EditBufferPolicy::ExistingValue);

        model.active_tab = Tab::Settings;
        model.active_setting = settings_index::REFRESH_RATE;
        model.form.fps_input = tui_input::Input::default().with_value(String::from("60"));
        model.start_editing();
        assert_eq!(model.form.fps_input.value(), "60");
        assert_eq!(model.form.fps_input.cursor(), 2);
        model.stop_editing();

        model.active_setting = settings_index::WEATHER_API_KEY;
        model.form.api_key_input =
            tui_input::Input::default().with_value(String::from("existing_secret_123"));
        model.start_editing();
        assert_eq!(model.form.api_key_input.value(), "");
        model.cancel_editing();
        assert_eq!(model.form.api_key_input.value(), "existing_secret_123");
    }
    #[test]
    fn test_editor_cursor_insert_middle_and_boundary() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 3; // Timezone field
        model.form.timezone_input = tui_input::Input::default().with_value(String::from("UTC"));

        model.start_editing();
        assert_eq!(model.input_mode, InputMode::Editing);
        // Cursor starts at end
        assert_eq!(model.form.timezone_input.cursor(), 3);

        // Home moves to start
        model.move_cursor_start();
        assert_eq!(model.form.timezone_input.cursor(), 0);

        // Insert at beginning
        model.insert_char_to_active_input('A');
        assert_eq!(model.form.timezone_input.value(), "AUTC");
        assert_eq!(model.form.timezone_input.cursor(), 1);

        // End moves to end
        model.move_cursor_end();
        assert_eq!(model.form.timezone_input.cursor(), 4);

        // Left moves back 1
        model.move_cursor_left();
        assert_eq!(model.form.timezone_input.cursor(), 3);

        // Insert in middle
        model.insert_char_to_active_input('B');
        assert_eq!(model.form.timezone_input.value(), "AUTBC");
        assert_eq!(model.form.timezone_input.cursor(), 4);

        // Backspace deletes 'B'
        model.backspace_active_input();
        assert_eq!(model.form.timezone_input.value(), "AUTC");
        assert_eq!(model.form.timezone_input.cursor(), 3);

        // Delete deletes 'C' at cursor
        model.delete_active_input();
        assert_eq!(model.form.timezone_input.value(), "AUT");
        assert_eq!(model.form.timezone_input.cursor(), 3);

        // Delete at end is a safe no-op
        model.delete_active_input();
        assert_eq!(model.form.timezone_input.value(), "AUT");
        assert_eq!(model.form.timezone_input.cursor(), 3);

        // Home then Backspace at start is a safe no-op
        model.move_cursor_start();
        assert_eq!(model.form.timezone_input.cursor(), 0);
        model.backspace_active_input();
        assert_eq!(model.form.timezone_input.value(), "AUT");
        assert_eq!(model.form.timezone_input.cursor(), 0);
    }

    #[test]
    fn test_editor_unicode_multibyte_correctness() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 3;

        // "İstanbul" contains 2-byte 'İ' (U+0130)
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("İstanbul"));
        model.start_editing();
        assert_eq!(model.form.timezone_input.cursor(), 8);

        // Move left across Unicode characters
        for _ in 0..7 {
            model.move_cursor_left();
        }
        // Cursor is now at index 1 (between 'İ' and 's')
        assert_eq!(model.form.timezone_input.cursor(), 1);

        // Insert character after 'İ'
        model.insert_char_to_active_input('X');
        assert_eq!(model.form.timezone_input.value(), "İXstanbul");
        assert_eq!(model.form.timezone_input.cursor(), 2);

        // Delete at cursor ('s')
        model.delete_active_input();
        assert_eq!(model.form.timezone_input.value(), "İXtanbul");

        // Backspace deletes 'X'
        model.backspace_active_input();
        assert_eq!(model.form.timezone_input.value(), "İtanbul");
        assert_eq!(model.form.timezone_input.cursor(), 1);

        // Backspace deletes 'İ'
        model.backspace_active_input();
        assert_eq!(model.form.timezone_input.value(), "tanbul");
        assert_eq!(model.form.timezone_input.cursor(), 0);

        // CJK and Umlaut test without panic
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("日本/München"));
        model.move_cursor_start();
        model.move_cursor_right(); // after '日'
        model.insert_char_to_active_input('★');
        assert_eq!(model.form.timezone_input.value(), "日★本/München");
    }

    #[test]
    fn test_editor_word_deletion_and_movement() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 3;

        // Test 1: "Europe/Istanbul"
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("Europe/Istanbul"));
        model.start_editing();
        assert_eq!(model.form.timezone_input.cursor(), 15);

        // Ctrl-W deletes "Istanbul"
        model.delete_prev_word_active_input();
        assert_eq!(model.form.timezone_input.value(), "Europe/");
        assert_eq!(model.form.timezone_input.cursor(), 7);

        // Ctrl-W deletes "Europe/"
        model.delete_prev_word_active_input();
        assert_eq!(model.form.timezone_input.value(), "");
        assert_eq!(model.form.timezone_input.cursor(), 0);

        // Test 2: "My Monitor"
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("My Monitor"));
        model.move_cursor_end();
        model.delete_prev_word_active_input();
        assert_eq!(model.form.timezone_input.value(), "My ");

        // Test 3: "foo   bar"
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("foo   bar"));
        model.move_cursor_end();
        model.delete_prev_word_active_input();
        assert_eq!(model.form.timezone_input.value(), "foo   ");

        // Test 4: Word navigation on "192.168.1.1"
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("192.168.1.1"));
        model.move_cursor_end();
        assert_eq!(model.form.timezone_input.cursor(), 11);

        model.move_cursor_prev_word();
        assert_eq!(model.form.timezone_input.cursor(), 10); // start of "1"

        model.move_cursor_prev_word();
        assert_eq!(model.form.timezone_input.cursor(), 8); // start of "1"

        model.move_cursor_prev_word();
        assert_eq!(model.form.timezone_input.cursor(), 4); // start of "168"

        model.move_cursor_prev_word();
        assert_eq!(model.form.timezone_input.cursor(), 0); // start of "192"

        model.move_cursor_next_word();
        assert_eq!(model.form.timezone_input.cursor(), 4);
    }
}
