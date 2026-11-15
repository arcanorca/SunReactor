use std::time::Instant;

use chrono::{DateTime, FixedOffset};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::kit;
use super::super::monitors::monitor_limits;
use crate::{
    policy::MonitorMilestoneSchedule,
    tui::{model::AutomationRegionFocus, theme::SemanticStyles, Model},
};

#[derive(Debug, Clone, Copy)]
struct ScheduleColumns {
    event: usize,
    offset: Option<usize>,
    time: usize,
    target: usize,
}

fn schedule_columns(
    schedule: &MonitorMilestoneSchedule,
    width: usize,
    use_12h: bool,
) -> ScheduleColumns {
    let longest_event = schedule
        .milestones
        .iter()
        .map(|entry| kit::cell_width(entry.milestone.label()))
        .max()
        .unwrap_or(8);
    let time = if use_12h { 8 } else { 6 };
    let target = 6;
    let marker = 3;
    let gap = 2;
    let base_width = marker + longest_event + gap + time + gap + target;
    let offset_width = 7;
    let has_offsets = schedule
        .milestones
        .iter()
        .any(|entry| entry.minutes_offset != 0);
    let show_offset = has_offsets && width >= base_width + gap + offset_width;
    let reserve =
        marker + gap + time + gap + target + if show_offset { gap + offset_width } else { 0 };
    let event = longest_event.min(width.saturating_sub(reserve)).max(6);
    ScheduleColumns {
        event,
        offset: show_offset.then_some(offset_width),
        time,
        target,
    }
}

pub(super) fn render_schedule_panel(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let focused = matches!(app.automation_focus, AutomationRegionFocus::Milestones)
        && app.workspace_focused();
    let block = kit::with_meta(
        kit::panel("Schedule", focused, styles),
        vec![
            Span::styled(format!("{} ", kit::DOT), styles.current_marker),
            Span::styled("now", styles.text_muted),
        ],
    );
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let Some(schedule) = schedule_for_panel(app, logical_id, inner, f, styles) else {
        return;
    };
    if schedule.milestones.is_empty() {
        return;
    }

    let use_12h = app.config.tui.use_12h_time;
    let columns = schedule_columns(schedule, usize::from(inner.width), use_12h);
    let selected = app
        .selected_monitor_milestone
        .min(schedule.milestones.len() - 1);
    let current = current_milestone_index(schedule, &app.current_local_time());
    let viewport =
        ScheduleViewport::new(inner, schedule.milestones.len(), focused, selected, current);
    let lines = schedule_lines(
        app, logical_id, schedule, columns, focused, selected, current, viewport, use_12h, styles,
    );
    let used = lines.len() as u16;
    f.render_widget(Paragraph::new(lines), viewport.table);
    if viewport.footer {
        render_range_footer(
            f,
            app,
            logical_id,
            Rect::new(inner.x, inner.y + used + 1, inner.width, 1),
            styles,
        );
    }
}

fn schedule_for_panel<'a>(
    app: &'a Model,
    logical_id: &str,
    inner: Rect,
    f: &mut Frame,
    styles: &SemanticStyles,
) -> Option<&'a MonitorMilestoneSchedule> {
    if let Some(error) = &app.monitor_milestone_error {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("Schedule unavailable: {error}"),
                styles.status_error,
            )),
            inner,
        );
        return None;
    }
    let schedule = app
        .monitor_milestones
        .iter()
        .find(|schedule| schedule.logical_id == logical_id);
    if schedule.is_none() {
        f.render_widget(
            Paragraph::new(Span::styled("Schedule loading", styles.text_muted)),
            inner,
        );
    }
    schedule
}

#[derive(Debug, Clone, Copy)]
struct ScheduleViewport {
    footer: bool,
    table: Rect,
    spaced: bool,
    start: usize,
    shown: usize,
}

impl ScheduleViewport {
    fn new(
        inner: Rect,
        total: usize,
        focused: bool,
        selected: usize,
        current: Option<usize>,
    ) -> Self {
        // Taller terminals get breathing rows rather than rules between every item.
        let footer = usize::from(inner.height) >= 2 + total + 2;
        let table = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            inner.height - if footer { 2 } else { 0 },
        );
        let spaced = usize::from(table.height) >= 2 + total * 2 - 1;
        let row_stride = if spaced { 2 } else { 1 };
        let visible_rows = usize::from(table.height.saturating_sub(2))
            .saturating_add(usize::from(spaced))
            / row_stride;
        let anchor = if focused {
            selected
        } else {
            current.unwrap_or(0)
        };
        let start = if total <= visible_rows {
            0
        } else {
            anchor
                .saturating_sub(visible_rows / 2)
                .min(total - visible_rows)
        };
        Self {
            footer,
            table,
            spaced,
            start,
            shown: total.saturating_sub(start).min(visible_rows),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn schedule_lines(
    app: &Model,
    logical_id: &str,
    schedule: &MonitorMilestoneSchedule,
    columns: ScheduleColumns,
    focused: bool,
    selected: usize,
    current: Option<usize>,
    viewport: ScheduleViewport,
    use_12h: bool,
    styles: &SemanticStyles,
) -> Vec<Line<'static>> {
    let mut lines = vec![schedule_header(columns, styles)];
    lines.push(Line::from(Span::styled(
        "─".repeat(usize::from(viewport.table.width)),
        styles.border_normal,
    )));
    let now = Instant::now();
    for (visible_index, (index, entry)) in schedule
        .milestones
        .iter()
        .enumerate()
        .skip(viewport.start)
        .take(viewport.shown)
        .enumerate()
    {
        lines.push(schedule_row(
            app, logical_id, entry, index, columns, focused, selected, current, use_12h, now,
            styles,
        ));
        if viewport.spaced && visible_index + 1 < viewport.shown {
            lines.push(Line::from(""));
        }
    }
    lines
}

fn schedule_header(columns: ScheduleColumns, styles: &SemanticStyles) -> Line<'static> {
    let mut header = vec![
        Span::raw("   "),
        Span::styled(kit::pad_to("Event", columns.event), styles.data_label),
    ];
    if let Some(offset) = columns.offset {
        header.push(Span::raw("  "));
        header.push(Span::styled(
            kit::pad_left("Offset", offset),
            styles.data_label,
        ));
    }
    header.push(Span::raw("  "));
    header.push(Span::styled(
        kit::pad_to("Time", columns.time),
        styles.data_label,
    ));
    header.push(Span::raw("  "));
    header.push(Span::styled(
        kit::pad_left("Target", columns.target),
        styles.data_label,
    ));
    Line::from(header)
}

#[allow(clippy::too_many_arguments)]
fn schedule_row(
    app: &Model,
    logical_id: &str,
    entry: &crate::policy::MonitorMilestone,
    index: usize,
    columns: ScheduleColumns,
    focused: bool,
    selected: usize,
    current: Option<usize>,
    use_12h: bool,
    now: Instant,
    styles: &SemanticStyles,
) -> Line<'static> {
    let is_selected = focused && index == selected;
    let is_current = current == Some(index);
    let adjusting = app.motion.milestone_adjust_phase(now, index).is_some();
    let row_style = if is_selected {
        styles.item_focused.add_modifier(Modifier::BOLD)
    } else {
        styles.text_primary
    };
    let mut spans = vec![
        if is_selected {
            Span::styled("▸", styles.focus_marker)
        } else {
            Span::raw(" ")
        },
        if is_current {
            Span::styled(kit::DOT, styles.current_marker)
        } else {
            Span::raw(" ")
        },
        Span::raw(" "),
        Span::styled(
            kit::pad_to(entry.milestone.label(), columns.event),
            row_style,
        ),
    ];
    if let Some(width) = columns.offset {
        let (offset, offset_style) = if entry.minutes_offset == 0 {
            (String::from("—"), styles.text_muted)
        } else if adjusting {
            (format!("{:+}m", entry.minutes_offset), styles.item_editing)
        } else {
            (
                format!("{:+}m", entry.minutes_offset),
                styles.item_modified.add_modifier(Modifier::BOLD),
            )
        };
        spans.push(Span::raw("  "));
        spans.push(Span::styled(kit::pad_left(&offset, width), offset_style));
    }
    let mut time = format_time(&entry.adjusted_time_local, use_12h);
    if app.is_milestone_constrained(logical_id, entry) {
        time.push('*');
    }
    spans.push(Span::raw("  "));
    spans.push(Span::styled(kit::pad_to(&time, columns.time), row_style));
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        kit::pad_left(&format!("{}%", entry.target_percent), columns.target),
        if app.monitor_policy_preview_pending() {
            styles.text_muted
        } else {
            row_style
        },
    ));
    Line::from(spans)
}

fn render_range_footer(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    let Some((min_pct, max_pct, _)) = monitor_limits(app, logical_id) else {
        return;
    };
    let mut spans = vec![
        Span::raw("   "),
        Span::styled("Range ", styles.text_muted),
        Span::styled(format!("{min_pct}–{max_pct}%"), styles.text_primary),
    ];
    let hint = "  · set in Monitors";
    let used: usize = spans.iter().map(Span::width).sum();
    if used + kit::cell_width(hint) <= usize::from(area.width) {
        spans.push(Span::styled(hint, styles.text_muted));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Index of the most recently reached milestone today, if any.
pub(crate) fn current_milestone_index(
    schedule: &MonitorMilestoneSchedule,
    now_local: &DateTime<FixedOffset>,
) -> Option<usize> {
    schedule
        .milestones
        .iter()
        .enumerate()
        .filter(|(_, milestone)| milestone.adjusted_time_local <= *now_local)
        .map(|(index, _)| index)
        .next_back()
}

pub(crate) fn format_time(time: &DateTime<FixedOffset>, use_12h: bool) -> String {
    if use_12h {
        time.format("%I:%M %p").to_string()
    } else {
        time.format("%H:%M").to_string()
    }
}
