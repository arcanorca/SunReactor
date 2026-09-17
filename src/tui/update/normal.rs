use crate::tui::actions::reset_selected_monitor_milestone;
use crate::tui::input::UiAction;
use crate::tui::{ActiveInputKind, Model, Tab};

use super::navigation::{move_horizontal, move_vertical};

pub(super) fn dispatch_normal_action(action: UiAction, app: &mut Model) {
    use crate::tui::model::AutomationRegionFocus;

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
                    app.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;
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

fn activate_focused_control(app: &mut Model) {
    use crate::tui::model::{AutomationRegionFocus, MonitorPaneFocus};

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
    use crate::tui::model::{AutomationRegionFocus, MonitorPaneFocus};

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

pub(super) fn save_toggled_settings(app: &mut Model) {
    if matches!(app.active_modal, crate::tui::model::ActiveModal::None) {
        let _ = app.save_config();
    }
}

pub(super) fn mark_milestone_adjustment(app: &mut Model) {
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
