use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::input::map_normal_key;
use super::model::ActiveModal;
use super::worker::IpcEvent;
use super::{InputMode, Model};

mod editing;
mod ipc;
mod navigation;
mod normal;

pub(crate) enum Message {
    Key(KeyEvent),
    Ipc(IpcEvent),
    Resize(u16, u16),
    Tick,
}

pub(crate) fn update(model: &mut Model, msg: Message) {
    match msg {
        Message::Key(key) => handle_key(key, model),
        Message::Ipc(event) => ipc::handle_ipc(event, model),
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

fn handle_key(key: KeyEvent, app: &mut Model) {
    if key.kind == crossterm::event::KeyEventKind::Release {
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'C'))
    {
        if app.config_dirty && !app.save_config() {
            return;
        }
        app.should_quit = true;
        return;
    }

    if !matches!(app.active_modal, ActiveModal::None) {
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
                normal::dispatch_normal_action(action, app);
            }
        }
        InputMode::Editing => {
            editing::handle_editing_key(key, app);
        }
    }
}

#[cfg(test)]
mod tests {

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use crate::policy::{AutomationMilestone, MonitorMilestone, MonitorMilestoneSchedule};
    use crate::tui::test_support::{find_in_buffer, press, two_monitor_model};
    use crate::tui::ui;
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    use super::{update, Message};
    use crate::tui::model::ActiveModal;
    use crate::tui::test_support::{dummy_status, key_event};
    use crate::tui::{InputMode, Model, Tab};

    #[test]
    fn test_help_modal_key_consumption() {
        let mut model = Model::new();
        model.show_help = true;
        model.active_tab = Tab::Monitors;
        model.help_scroll = 0;

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('j'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.help_scroll, 1);

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('2'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, Tab::Monitors);

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Esc,
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert!(!model.show_help);
    }

    #[test]
    fn test_key_precedence_ctrl_c() {
        let mut model = Model::new();
        assert!(!model.should_quit);

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            )),
        );
        assert!(model.should_quit);
    }

    #[test]
    fn test_key_precedence_release_event_ignored() {
        let mut model = Model::new();
        model.active_tab = Tab::Monitors;

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('2'),
                KeyModifiers::empty(),
                KeyEventKind::Release,
            )),
        );
        assert_eq!(model.active_tab, Tab::Monitors);
    }

    #[test]
    fn test_numbered_tabs_switch_in_normal_mode() {
        let mut model = Model::new();
        assert_eq!(model.active_tab, Tab::Monitors);

        for (key_char, expected_tab) in [
            ('2', Tab::Limits),
            ('3', Tab::Location),
            ('4', Tab::Weather),
            ('5', Tab::Settings),
            ('1', Tab::Monitors),
        ] {
            update(
                &mut model,
                Message::Key(key_event(
                    KeyCode::Char(key_char),
                    KeyModifiers::empty(),
                    KeyEventKind::Press,
                )),
            );
            assert_eq!(model.active_tab, expected_tab);
        }
    }

    #[test]
    fn test_editing_mode_absorbs_keys() {
        let mut model = Model::new();
        model.active_tab = Tab::Limits;
        model.input_mode = InputMode::Editing;

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('1'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, Tab::Limits);

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Esc,
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_theme_modal_consumes_keys() {
        let mut model = Model::new();
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(0));
        model.active_modal = ActiveModal::ThemeSelect(state, crate::tui::theme::Theme::Amber);

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('2'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, Tab::Monitors);

        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Esc,
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert!(matches!(model.active_modal, ActiveModal::None));
    }

    #[test]
    fn test_resize_event_handling() {
        let mut model = Model::new();
        model.status = Some(dummy_status(5));
        model.selected_monitor = 4;
        model.clamp_monitor_selection();

        update(&mut model, Message::Resize(40, 16));
        assert!(model.selected_monitor <= 4);

        model.status = Some(dummy_status(1));
        update(&mut model, Message::Resize(60, 20));
        assert_eq!(model.selected_monitor, 0);
    }

    #[test]
    fn test_navigation_monitors_and_automation_shortcuts() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.active_tab = Tab::Monitors;
        model.selected_monitor = 1;

        // Press 'a' on Monitors -> switches to Tab::Limits (Automation) preserving selected_monitor
        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('a'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, Tab::Limits);
        assert_eq!(model.selected_monitor, 1);

        // Press '1' -> switches back to Monitors
        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('1'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, Tab::Monitors);
        assert_eq!(model.selected_monitor, 1);

        // Press '2' -> switches to Automation
        update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char('2'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, Tab::Limits);
        assert_eq!(model.selected_monitor, 1);
    }

    #[test]
    fn test_phase9_modal_keys_open_close_and_q() {
        let mut model = Model::new();
        assert!(!model.show_help);

        // 1. '?' opens help
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(model.show_help);

        // 2. '?' again closes help
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(!model.show_help);

        // 3. '?' opens, Esc closes
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(model.show_help);
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Esc,
            )),
        );
        assert!(!model.show_help);

        // 4. '?' opens, 'q' closes help without quitting application
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(model.show_help);
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('q'),
            )),
        );
        assert!(!model.show_help);
        assert!(!model.should_quit); // Must NOT quit the app
    }

    #[test]
    fn test_phase9_1_input_precedence_editing_mode_absorbs_question_mark() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 3; // Timezone field (text)
        model.form.timezone_input = tui_input::Input::default().with_value(String::from("UTC"));

        // Enter editing mode
        model.start_editing();
        assert_eq!(model.input_mode, InputMode::Editing);
        assert!(!model.show_help);

        // Typing '?' into text input must be absorbed into buffer, NOT open help
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(!model.show_help);
        assert_eq!(model.input_mode, InputMode::Editing);
        assert_eq!(model.form.timezone_input.value(), "UTC?");

        // Numeric input rejects '?'
        model.stop_editing();
        model.active_tab = Tab::Settings;
        model.active_setting = crate::tui::model::settings_index::REFRESH_RATE;
        model.form.fps_input = tui_input::Input::default().with_value(String::from("60"));
        model.start_editing();

        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(!model.show_help);
        assert_eq!(model.input_mode, InputMode::Editing);
        assert_eq!(model.form.fps_input.value(), "60");

        // In edit mode, footer reflects edit commands with no global commands
        let footer_cmds = crate::tui::command::commands_for_footer(&model);
        assert!(footer_cmds.global_commands.is_empty());
    }

    #[test]
    fn test_phase9_1_modal_precedence_q_esc_question_mark_state_preservation() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.active_tab = Tab::Monitors;
        model.selected_monitor = 1;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
        model.monitor_control_index = 1;

        let snapshot_before = (
            model.active_tab,
            model.selected_monitor,
            model.monitor_pane_focus,
            model.monitor_control_index,
            model.should_quit,
        );

        // 1. Open Help via '?'
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(model.show_help);

        // 2. Close Help via 'q'
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('q'),
            )),
        );
        assert!(!model.show_help);
        let snapshot_after_q = (
            model.active_tab,
            model.selected_monitor,
            model.monitor_pane_focus,
            model.monitor_control_index,
            model.should_quit,
        );
        assert_eq!(snapshot_before, snapshot_after_q);

        // 3. Open Help via '?', close via Esc
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(model.show_help);
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Esc,
            )),
        );
        assert!(!model.show_help);
        let snapshot_after_esc = (
            model.active_tab,
            model.selected_monitor,
            model.monitor_pane_focus,
            model.monitor_control_index,
            model.should_quit,
        );
        assert_eq!(snapshot_before, snapshot_after_esc);

        // 4. Open Help via '?', close via '?'
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(model.show_help);
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('?'),
            )),
        );
        assert!(!model.show_help);
        let snapshot_after_qm = (
            model.active_tab,
            model.selected_monitor,
            model.monitor_pane_focus,
            model.monitor_control_index,
            model.should_quit,
        );
        assert_eq!(snapshot_before, snapshot_after_qm);
    }

    #[test]
    fn test_phase9_1_a_destination_shortcut_semantics() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.active_tab = Tab::Monitors;
        model.selected_monitor = 1;

        // 'a' from Monitors switches to Automation (Tab::Limits)
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('a'),
            )),
        );
        assert_eq!(model.active_tab, Tab::Limits);
        assert_eq!(model.selected_monitor, 1); // Selected monitor preserved

        // 'a' from Automation remains in Automation (no-op)
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('a'),
            )),
        );
        assert_eq!(model.active_tab, Tab::Limits); // Still Automation!
        assert_eq!(model.selected_monitor, 1);

        // '1' returns to Monitors
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('1'),
            )),
        );
        assert_eq!(model.active_tab, Tab::Monitors);
        assert_eq!(model.selected_monitor, 1);

        // '2' goes to Automation
        update(
            &mut model,
            Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('2'),
            )),
        );
        assert_eq!(model.active_tab, Tab::Limits);
        assert_eq!(model.selected_monitor, 1);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_phase9_1_command_spec_conformance_dispatch() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.config.monitors = vec![
            crate::config::MonitorConfig {
                logical_id: String::from("mon-0"),
                transition_gamma: 0.5,
                ..Default::default()
            },
            crate::config::MonitorConfig {
                logical_id: String::from("mon-1"),
                transition_gamma: 0.5,
                ..Default::default()
            },
        ];

        // 1. Monitors List
        {
            model.active_tab = Tab::Monitors;
            model.selected_monitor = 0;
            model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;

            // Down changes selection
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Down,
                )),
            );
            assert_eq!(model.selected_monitor, 1);

            // Enter focuses detail
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Enter,
                )),
            );
            assert_eq!(
                model.monitor_pane_focus,
                crate::tui::model::MonitorPaneFocus::Detail
            );
        }

        // 2. Monitors Detail
        {
            // Esc returns to list
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Esc,
                )),
            );
            assert_eq!(
                model.monitor_pane_focus,
                crate::tui::model::MonitorPaneFocus::List
            );

            // Enter again to detail
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Enter,
                )),
            );
            assert_eq!(
                model.monitor_pane_focus,
                crate::tui::model::MonitorPaneFocus::Detail
            );

            // Enter from detail starts editing
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Enter,
                )),
            );
            assert_eq!(model.input_mode, InputMode::Editing);
            model.stop_editing();

            // Curve shape has no hidden mutation path from Monitors.
            let initial_gamma = model.config.monitors[1].transition_gamma;
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Char('+'),
                )),
            );
            let gamma_after = model.config.monitors[1].transition_gamma;
            assert!((gamma_after - initial_gamma).abs() < 1e-4);
        }

        // 3. Automation
        {
            model.active_tab = Tab::Limits;
            model.selected_monitor = 0;
            model.selected_monitor_id = None;
            model.clamp_monitor_selection();
            model.selected_monitor_milestone = 0;
            model.automation_focus = crate::tui::model::AutomationRegionFocus::Curve;

            let initial_curve = model.config.monitors[0].transition_gamma;
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Right,
                )),
            );
            assert!(
                (model.config.monitors[0].transition_gamma - (initial_curve + 0.05)).abs() < 1e-4
            );

            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Down,
                )),
            );
            assert_eq!(
                model.automation_focus,
                crate::tui::model::AutomationRegionFocus::Milestones
            );

            let now = model.current_local_time();
            model.monitor_milestones = vec![MonitorMilestoneSchedule {
                logical_id: String::from("mon-0"),
                milestones: vec![
                    MonitorMilestone {
                        milestone: AutomationMilestone::RiseStart,
                        base_time_local: now,
                        adjusted_time_local: now,
                        target_percent: 10,
                        minutes_offset: 0,
                    },
                    MonitorMilestone {
                        milestone: AutomationMilestone::Rise25,
                        base_time_local: now,
                        adjusted_time_local: now,
                        target_percent: 30,
                        minutes_offset: 0,
                    },
                ],
            }];

            // Down selects next milestone
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Down,
                )),
            );
            assert_eq!(model.selected_monitor_milestone, 1);

            // Right adjusts offset +1m
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Right,
                )),
            );
            let offset = model
                .config
                .monitors
                .first()
                .and_then(|m| {
                    m.milestone_adjustments
                        .iter()
                        .find(|adj| adj.milestone == AutomationMilestone::Rise25)
                        .map(|adj| adj.minutes_offset)
                })
                .unwrap_or(0);
            assert_eq!(offset, 1);

            // 'r' resets offset
            update(
                &mut model,
                Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Char('r'),
                )),
            );
            let offset_reset = model
                .config
                .monitors
                .first()
                .and_then(|m| {
                    m.milestone_adjustments
                        .iter()
                        .find(|adj| adj.milestone == AutomationMilestone::Rise25)
                        .map(|adj| adj.minutes_offset)
                })
                .unwrap_or(0);
            assert_eq!(offset_reset, 0);
        }

        // 4. Weather context has no editable commands
        {
            model.active_tab = Tab::Weather;
            let weather_cmds = crate::tui::command::commands_for_workspace(&model);
            assert!(weather_cmds.current_commands.is_empty());
        }
    }

    #[test]
    fn test_tab_bar_takes_focus_and_uses_left_right() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Location;
        model.active_setting = 0;

        // ↑ from the first field reaches the tab bar.
        press(&mut model, KeyCode::Up, KeyModifiers::NONE);
        assert!(model.tabs_focused);

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let (_, tab_row, _) = find_in_buffer(&buffer, "3 Location").unwrap();
        assert_eq!(buffer.get(0, tab_row).symbol(), "❯");
        let (x, y, _) = find_in_buffer(&buffer, "3 Location").unwrap();
        assert_eq!(buffer.get(x, y).bg, model.config.tui.theme.palette().accent);
        assert!(find_in_buffer(&buffer, "Workspace").is_some());
        // The form no longer shows a cursor while the tab bar has focus.
        assert!(find_in_buffer(&buffer, "❯ City").is_none());

        press(&mut model, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(model.active_tab, Tab::Weather);
        assert!(
            model.tabs_focused,
            "focus stays on the tab bar while moving along it"
        );
        press(&mut model, KeyCode::Left, KeyModifiers::NONE);
        press(&mut model, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(model.active_tab, Tab::Limits);

        press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        assert!(!model.tabs_focused);
        assert_eq!(model.active_tab, Tab::Limits);
    }

    #[test]
    fn test_browser_style_workspace_keys() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Location;

        press(&mut model, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(model.active_tab, Tab::Weather);
        press(&mut model, KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(model.active_tab, Tab::Location);
        press(&mut model, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(model.active_tab, Tab::Limits);
        press(&mut model, KeyCode::Tab, KeyModifiers::CONTROL);
        assert_eq!(model.active_tab, Tab::Location);
        press(
            &mut model,
            KeyCode::Tab,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(model.active_tab, Tab::Limits);
        press(&mut model, KeyCode::PageDown, KeyModifiers::CONTROL);
        assert_eq!(model.active_tab, Tab::Location);
        press(&mut model, KeyCode::PageUp, KeyModifiers::CONTROL);
        assert_eq!(model.active_tab, Tab::Limits);
        // Plain PgUp stays a list key.
        press(&mut model, KeyCode::PageUp, KeyModifiers::NONE);
        assert_eq!(model.active_tab, Tab::Limits);

        let help = crate::tui::command::commands_for_workspace(&model);
        assert!(help
            .global_commands
            .iter()
            .any(|command| command.keys == "Tab / Shift+Tab"));
    }

    #[test]
    fn test_vertical_monitor_list_uses_up_down_and_right_enters_controls() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Monitors;
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        assert!(!model.monitor_selector_horizontal);

        press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(model.selected_monitor, 1);
        press(&mut model, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(
            model.monitor_pane_focus,
            crate::tui::model::MonitorPaneFocus::Detail
        );
        assert_eq!(model.selected_monitor, 1, "→ never changes the selection");

        press(&mut model, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            model.monitor_pane_focus,
            crate::tui::model::MonitorPaneFocus::List
        );
        press(&mut model, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(model.selected_monitor, 0);
        press(&mut model, KeyCode::Up, KeyModifiers::NONE);
        assert!(
            model.tabs_focused,
            "↑ from the first monitor reaches the tab bar"
        );
    }

    #[test]
    fn test_narrow_monitor_switcher_uses_left_right() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Monitors;
        let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        assert!(model.monitor_selector_horizontal);
        let commands = crate::tui::command::commands_for_footer(&model);
        assert_eq!(commands.current_commands[0].keys, "← →");

        press(&mut model, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(model.selected_monitor, 1);
        press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(
            model.monitor_pane_focus,
            crate::tui::model::MonitorPaneFocus::Detail
        );
        press(&mut model, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(
            model.monitor_pane_focus,
            crate::tui::model::MonitorPaneFocus::List
        );
    }

    #[test]
    fn test_monitors_down_from_tab_bar_returns_to_the_monitor_list() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Monitors;
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        press(&mut model, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(
            model.monitor_pane_focus,
            crate::tui::model::MonitorPaneFocus::Detail
        );
        press(&mut model, KeyCode::Up, KeyModifiers::NONE);
        assert!(
            model.tabs_focused,
            "↑ from the first range row reaches the tabs"
        );

        press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        assert!(!model.tabs_focused);
        assert_eq!(
            model.monitor_pane_focus,
            crate::tui::model::MonitorPaneFocus::List,
            "↓ from the tabs lands on the monitor list"
        );
        assert_eq!(model.selected_monitor, 1, "the selection is kept");
    }
}
