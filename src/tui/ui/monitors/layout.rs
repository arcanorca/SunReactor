use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::truncate;
use super::super::{kit, monitor_selector};
use super::detail::{
    monitor_state, now_epoch_seconds, render_monitor_detail, valid_applied_percent,
};
use super::state::render_monitor_workspace_state;
use crate::tui::theme::SemanticStyles;
use crate::tui::{Model, MonitorPaneFocus};

/// Renders the Monitors workspace: a selector list and the selected monitor.
pub(crate) fn render_monitors(f: &mut Frame, app: &mut Model, area: Rect, styles: &SemanticStyles) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let monitor = app
        .status
        .as_ref()
        .and_then(|status| status.monitors.get(app.selected_monitor))
        .cloned();
    if monitor.is_none() && app.monitors_count() == 0 {
        render_monitor_workspace_state(f, app, area, styles);
        return;
    }

    // Panels are sized to their content so a tall terminal gets calm
    // whitespace below instead of empty boxes.
    let content_height = (15u16).max(monitor_ids(app).len() as u16 * 3 + 2);
    let area = Rect::new(area.x, area.y, area.width, area.height.min(content_height));
    // The model needs to know which arrow keys move the selector.
    app.monitor_selector_horizontal = !(area.width >= 72 && area.height >= 8);
    let (list_area, detail_area) = if area.width >= 72 && area.height >= 8 {
        let list_width = monitor_list_width(app).min(area.width / 3);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(list_width),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        (columns[0], columns[2])
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        render_monitor_switcher(f, app, rows[0], styles);
        (Rect::default(), rows[1])
    };

    if list_area.width > 0 {
        render_monitor_list(f, app, list_area, styles);
    }
    match monitor {
        Some(monitor) => render_monitor_detail(f, app, &monitor, detail_area, styles),
        None => render_monitor_workspace_state(f, app, detail_area, styles),
    }
}

/// Logical IDs in display order: daemon records when present, otherwise the
/// configured monitors so identities stay visible during a daemon restart.
pub(crate) fn monitor_ids(app: &Model) -> Vec<String> {
    app.status
        .as_ref()
        .filter(|status| !status.monitors.is_empty())
        .map_or_else(
            || {
                app.config
                    .monitors
                    .iter()
                    .map(|monitor| monitor.logical_id.clone())
                    .collect()
            },
            |status| {
                status
                    .monitors
                    .iter()
                    .map(|monitor| monitor.logical_id.clone())
                    .collect()
            },
        )
}

fn monitor_list_width(app: &Model) -> u16 {
    let longest = monitor_ids(app)
        .iter()
        .map(|id| kit::cell_width(&display_name(app, id)))
        .max()
        .unwrap_or(10);
    (longest as u16 + 8).clamp(24, 34)
}

pub(crate) fn display_name(app: &Model, logical_id: &str) -> String {
    monitor_selector::monitor_display_name(app, logical_id)
}

/// One-line selector used where there is no room for the list. `[` and `]`
/// frame the name because they are the keys that change it.
pub(crate) fn render_monitor_switcher(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let ids = monitor_ids(app);
    let count = ids.len();
    let index = app.selected_monitor.min(count.saturating_sub(1));
    let name = ids
        .get(index)
        .map_or_else(|| String::from("No monitor"), |id| display_name(app, id));
    let name_width = usize::from(area.width.saturating_sub(24)).max(6);
    let focused =
        matches!(app.monitor_pane_focus, MonitorPaneFocus::List) && app.workspace_focused();
    let arrow = if focused {
        styles.focus_marker
    } else {
        styles.text_muted
    };
    let mut spans = vec![
        kit::cursor_span(focused, styles),
        Span::styled("Monitor  ", styles.data_label),
        Span::styled("‹ ", arrow),
        Span::styled(
            format!(" {} ", truncate(&name, name_width)),
            if focused {
                styles.value_capsule_focused
            } else {
                styles.text_primary.add_modifier(Modifier::BOLD)
            },
        ),
        Span::styled(" ›", arrow),
    ];
    if count > 0 {
        spans.push(Span::styled(
            format!("  {} of {count}", index + 1),
            styles.text_muted,
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_monitor_list(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    let focused =
        matches!(app.monitor_pane_focus, MonitorPaneFocus::List) && app.workspace_focused();
    let ids = monitor_ids(app);
    let block = kit::with_meta(
        kit::panel("Monitors", focused, styles),
        vec![Span::styled(
            format!(
                "{} of {}",
                app.selected_monitor.min(ids.len().saturating_sub(1)) + 1,
                ids.len()
            ),
            styles.text_muted,
        )],
    );
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let two_line = usize::from(inner.height) >= ids.len() * 3;
    let now_epoch_s = now_epoch_seconds();
    let mut lines = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let selected = index == app.selected_monitor;
        let marker = match (selected, focused) {
            (true, true) => Span::styled(kit::CURSOR, styles.focus_marker),
            (true, false) => Span::styled(kit::RETAINED, styles.text_muted),
            (false, _) => Span::raw(kit::BLANK_CURSOR),
        };
        let name_style = if selected {
            styles.text_primary.add_modifier(Modifier::BOLD)
        } else {
            styles.text_primary
        };
        let status = app
            .status
            .as_ref()
            .and_then(|status| status.monitors.iter().find(|m| &m.logical_id == id));
        let (glyph, state_text, state_style) = monitor_state(status, now_epoch_s, styles);
        let name_width = usize::from(inner.width).saturating_sub(if two_line { 2 } else { 4 });
        let mut first = vec![
            marker,
            Span::styled(truncate(&display_name(app, id), name_width), name_style),
        ];
        if two_line {
            lines.push(Line::from(first));
            let applied = status
                .and_then(|monitor| valid_applied_percent(monitor.last_applied_percent))
                .map(|value| format!("{value}%"));
            let mut detail = vec![Span::styled(format!("{glyph} "), state_style)];
            match applied {
                Some(applied) if state_text == "Ready" => {
                    detail.push(Span::styled(applied, styles.text_primary));
                }
                _ => detail.push(Span::styled(state_text, styles.text_muted)),
            }
            if let Some(hold) = status
                .and_then(|monitor| monitor.external_hold)
                .filter(|hold| hold.remaining_s > 0)
            {
                detail.push(Span::styled(
                    format!("  external · {}m", hold.remaining_s.saturating_add(59) / 60),
                    styles.status_warning,
                ));
            }
            lines.push(kit::detail_line(detail, styles));
            lines.push(Line::from(""));
        } else {
            first.push(Span::styled(format!(" {glyph}"), state_style));
            lines.push(Line::from(first));
        }
    }
    f.render_widget(Paragraph::new(lines), inner);
}
