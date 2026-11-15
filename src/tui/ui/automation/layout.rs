use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::{kit, light_cycle, monitors};
use super::controls::render_controls_panel;
use super::schedule::format_time;
use super::schedule::render_schedule_panel;
use crate::tui::{theme::SemanticStyles, Model};

const PANEL_GAP: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationLayout {
    ThreeRegion,
    Compact,
    Minimal,
}

/// Renders one selected monitor's controls, today's solar instrument, and
/// editable schedule. The visual layers deliberately keep astronomical timing
/// separate from the policy response sampled by the background preview job.
pub(crate) fn render_automation(
    f: &mut Frame,
    app: &mut Model,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let monitor = app
        .status
        .as_ref()
        .and_then(|status| status.monitors.get(app.selected_monitor))
        .cloned();
    let Some(monitor) = monitor else {
        monitors::render_monitor_workspace_state(f, app, area, styles);
        return;
    };

    match automation_layout(area) {
        AutomationLayout::ThreeRegion => {
            render_three_region(f, app, &monitor.logical_id, area, styles);
        }
        AutomationLayout::Compact => render_compact(f, app, &monitor.logical_id, area, styles),
        AutomationLayout::Minimal => render_minimal(f, app, &monitor.logical_id, area, styles),
    }
}

fn automation_layout(area: Rect) -> AutomationLayout {
    // `body_area` removes a gutter, so a nominal 120-column terminal has
    // enough room for the three regions only once its usable width is 112.
    if area.width >= 112 && area.height >= 16 {
        AutomationLayout::ThreeRegion
    } else if area.width >= 68 && area.height >= 14 {
        AutomationLayout::Compact
    } else {
        AutomationLayout::Minimal
    }
}

fn render_three_region(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    // The middle is intentionally dominant. The side columns retain enough
    // width for one complete control surface and an easily scanned schedule.
    let left_width = (area.width.saturating_mul(23) / 100).clamp(28, 38);
    let right_width = (area.width.saturating_mul(26) / 100).clamp(35, 40);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_width),
            Constraint::Length(PANEL_GAP),
            Constraint::Min(36),
            Constraint::Length(PANEL_GAP),
            Constraint::Length(right_width),
        ])
        .split(area);

    render_controls_panel(f, app, logical_id, columns[0], styles, false);
    light_cycle::render_light_cycle_panel(f, app, logical_id, columns[2], styles);
    render_schedule_column(f, app, logical_id, columns[4], styles);
}

/// Schedule above a summary of what happens next, when there is room for both.
fn render_schedule_column(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    let milestones = app
        .monitor_milestones
        .iter()
        .find(|schedule| schedule.logical_id == logical_id)
        .map_or(9, |schedule| schedule.milestones.len()) as u16;
    // Borders, column header, its rule, and the range footer with its gap.
    let compact = milestones + 6;
    let spaced = compact + milestones.saturating_sub(1);
    // Two lines inside a border: the next milestone and the time until it.
    let next_height = 4;
    let (schedule_height, next) = if area.height >= spaced + PANEL_GAP + next_height {
        (spaced, true)
    } else if area.height >= compact + PANEL_GAP + next_height {
        (compact, true)
    } else {
        (area.height, false)
    };
    render_schedule_panel(
        f,
        app,
        logical_id,
        Rect::new(area.x, area.y, area.width, schedule_height),
        styles,
    );
    if next {
        render_next_event_panel(
            f,
            app,
            logical_id,
            Rect::new(
                area.x,
                area.y + schedule_height + PANEL_GAP,
                area.width,
                next_height,
            ),
            styles,
        );
    }
}

fn render_next_event_panel(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let block = kit::panel("Next event", false, styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    let now = app.current_local_time();
    let use_12h = app.config.tui.use_12h_time;
    let schedule = app
        .monitor_milestones
        .iter()
        .find(|schedule| schedule.logical_id == logical_id);
    let mut lines = Vec::new();
    let next = schedule.and_then(|schedule| {
        schedule
            .milestones
            .iter()
            .find(|entry| entry.adjusted_time_local > now)
            .or_else(|| schedule.milestones.first())
    });
    match next {
        Some(entry) => {
            let minutes = light_cycle::minutes_until(&now, &entry.adjusted_time_local);
            let wait = if minutes >= 60 {
                format!("in {}h {:02}m", minutes / 60, minutes % 60)
            } else {
                format!("in {minutes}m")
            };
            lines.push(Line::from(vec![
                Span::styled("→ ", styles.focus_marker),
                Span::styled(
                    entry.milestone.label(),
                    styles.text_primary.add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}%", entry.target_percent),
                    styles.data_value.add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "{wait} · {}",
                        format_time(&entry.adjusted_time_local, use_12h)
                    ),
                    styles.text_muted,
                ),
            ]));
        }
        None => lines.push(Line::from(Span::styled(
            "Schedule loading",
            styles.text_muted,
        ))),
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_compact(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width < 92 {
        let controls_height = 6.min(area.height.saturating_sub(4));
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(controls_height),
                Constraint::Length(PANEL_GAP),
                Constraint::Min(0),
            ])
            .split(area);
        render_controls_panel(f, app, logical_id, rows[0], styles, true);
        render_schedule_panel(f, app, logical_id, rows[2], styles);
        return;
    }

    let top_height = if area.height >= 22 { 11 } else { 8 };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_height.min(area.height.saturating_sub(4))),
            Constraint::Length(PANEL_GAP),
            Constraint::Min(0),
        ])
        .split(area);
    let controls_width = (area.width.saturating_mul(34) / 100).clamp(27, 34);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(controls_width),
            Constraint::Length(PANEL_GAP),
            Constraint::Min(28),
        ])
        .split(rows[0]);

    render_controls_panel(f, app, logical_id, top[0], styles, true);
    light_cycle::render_light_cycle_panel(f, app, logical_id, top[2], styles);
    render_schedule_panel(f, app, logical_id, rows[2], styles);
}

fn render_minimal(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    let controls_height = area.height.min(5);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(controls_height),
            Constraint::Length(u16::from(area.height > controls_height)),
            Constraint::Min(0),
        ])
        .split(area);
    render_controls_panel(f, app, logical_id, rows[0], styles, true);
    if rows[2].height >= 3 {
        render_schedule_panel(f, app, logical_id, rows[2], styles);
    }
}
