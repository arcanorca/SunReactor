use crate::tui::actions::{
    adjust_selected_monitor_milestone, select_next_monitor_milestone,
    select_previous_monitor_milestone,
};
use crate::tui::{ActiveInputKind, Model, Tab};

use super::normal::{mark_milestone_adjustment, save_toggled_settings};

fn select_monitor_step(app: &mut Model, delta: i8) {
    if delta > 0 {
        app.select_next_monitor();
    } else {
        app.select_previous_monitor();
    }
}

/// ←/→ act on horizontal arrangements: the narrow monitor switcher, the
/// Automation monitor chips, value steppers, and time offsets.
pub(super) fn move_horizontal(app: &mut Model, delta: i8) {
    use crate::tui::model::{AutomationRegionFocus, MonitorPaneFocus};

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
pub(super) fn move_vertical(app: &mut Model, delta: i8) {
    use crate::tui::model::{AutomationRegionFocus, MonitorPaneFocus};

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
