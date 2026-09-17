use crate::config::MonitorMilestoneAdjustment;

use super::Model;

pub(crate) fn select_previous_monitor_milestone(app: &mut Model) {
    if !matches!(app.active_tab, super::Tab::Limits) {
        return;
    }
    if app.selected_monitor_milestone > 0 {
        app.selected_monitor_milestone -= 1;
    }
}

pub(crate) fn select_next_monitor_milestone(app: &mut Model) {
    if !matches!(app.active_tab, super::Tab::Limits) {
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
