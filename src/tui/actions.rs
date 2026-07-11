use crate::config::MonitorMilestoneAdjustment;

use super::Model;

pub(crate) fn toggle_monitor_advanced(app: &mut Model) {
    app.monitor_advanced_open = !app.monitor_advanced_open;
    if let Some(schedule) = app.selected_monitor_schedule() {
        let max_index = schedule.milestones.len().saturating_sub(1);
        if app.selected_monitor_milestone > max_index {
            app.selected_monitor_milestone = max_index;
        }
    } else {
        app.selected_monitor_milestone = 0;
    }
}

pub(crate) fn select_previous_monitor_milestone(app: &mut Model) {
    if !app.monitor_advanced_open {
        return;
    }
    if app.selected_monitor_milestone > 0 {
        app.selected_monitor_milestone -= 1;
    }
}

pub(crate) fn select_next_monitor_milestone(app: &mut Model) {
    if !app.monitor_advanced_open {
        return;
    }
    let Some(schedule) = app.selected_monitor_schedule() else {
        return;
    };
    if app.selected_monitor_milestone + 1 < schedule.milestones.len() {
        app.selected_monitor_milestone += 1;
    }
}

pub(crate) fn adjust_selected_monitor_milestone(app: &mut Model, delta_minutes: i16) {
    let Some(schedule) = app.selected_monitor_schedule() else {
        return;
    };
    let Some(selected) = schedule.milestones.get(app.selected_monitor_milestone) else {
        return;
    };
    let milestone = selected.milestone;
    let Some(logical_id) = app.selected_monitor_logical_id().map(str::to_owned) else {
        return;
    };
    let Some(monitor) = app
        .config
        .monitors
        .iter_mut()
        .find(|monitor| monitor.logical_id == logical_id)
    else {
        return;
    };

    if let Some(existing) = monitor
        .milestone_adjustments
        .iter_mut()
        .find(|adjustment| adjustment.milestone == milestone)
    {
        existing.minutes_offset = existing
            .minutes_offset
            .saturating_add(delta_minutes)
            .clamp(-720, 720);
        if existing.minutes_offset == 0 {
            monitor
                .milestone_adjustments
                .retain(|adjustment| adjustment.milestone != milestone);
        }
    } else {
        monitor
            .milestone_adjustments
            .push(MonitorMilestoneAdjustment {
                milestone,
                minutes_offset: delta_minutes.clamp(-720, 720),
            });
    }

    app.reapply_milestone_offsets();
}

pub(crate) fn reset_selected_monitor_milestone(app: &mut Model) {
    let Some(schedule) = app.selected_monitor_schedule() else {
        return;
    };
    let Some(selected) = schedule.milestones.get(app.selected_monitor_milestone) else {
        return;
    };
    let milestone = selected.milestone;
    let Some(logical_id) = app.selected_monitor_logical_id().map(str::to_owned) else {
        return;
    };
    let Some(monitor) = app
        .config
        .monitors
        .iter_mut()
        .find(|monitor| monitor.logical_id == logical_id)
    else {
        return;
    };

    monitor
        .milestone_adjustments
        .retain(|adjustment| adjustment.milestone != milestone);

    app.reapply_milestone_offsets();
}
