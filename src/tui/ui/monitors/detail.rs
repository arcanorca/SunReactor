pub(crate) fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::kit::{self, FieldKind, FieldState};
use super::super::truncate;
use super::layout::display_name;
use crate::ipc::MonitorStatus;
use crate::tui::model::OperationalMode;
use crate::tui::motion::TransientKind;
use crate::tui::theme::SemanticStyles;
use crate::tui::{InputMode, Model, MonitorPaneFocus};

const LABEL_WIDTH: usize = 12;

/// A glyph, a short label, and a style for a monitor's availability.
pub(crate) fn monitor_state(
    monitor: Option<&MonitorStatus>,
    now_epoch_s: u64,
    styles: &SemanticStyles,
) -> (&'static str, String, ratatui::style::Style) {
    let Some(monitor) = monitor else {
        return (
            kit::HOLLOW,
            String::from("Waiting for daemon"),
            styles.text_muted,
        );
    };
    if !monitor.enabled {
        return (kit::HOLLOW, String::from("Disabled"), styles.text_muted);
    }
    if let Some(until) = monitor
        .backoff_until_epoch_s
        .filter(|until| *until > now_epoch_s)
    {
        return (
            kit::WARN,
            format!("Retrying in {}s", until - now_epoch_s),
            styles.status_warning,
        );
    }
    match monitor.topology.as_deref() {
        Some("present") | None => (kit::DOT, String::from("Ready"), styles.status_success),
        Some("temporarily_unavailable") => (
            kit::WARN,
            String::from("Unavailable"),
            styles.status_warning,
        ),
        Some("suppressed_proven_alias") => (
            kit::HOLLOW,
            String::from("Duplicate of another display"),
            styles.text_muted,
        ),
        Some("ambiguous_or_unsafe") => (
            kit::WARN,
            String::from("Needs selector review"),
            styles.status_error,
        ),
        Some("topology_retargeting_disabled") => (
            kit::WARN,
            String::from("Connector changed"),
            styles.status_warning,
        ),
        Some(_) => (kit::HOLLOW, String::from("Checking"), styles.text_muted),
    }
}

/// Returns only a physically meaningful applied brightness value. Runtime
/// state bounds normal writes, but a malformed or older IPC peer must not make
/// an impossible percentage look plausible.
pub(crate) fn valid_applied_percent(last_applied: Option<u8>) -> Option<u8> {
    last_applied.filter(|value| *value <= 100)
}

/// The configured range and gamma (`transition_gamma`), or `None` when the daemon reports a
/// monitor this configuration does not describe. Callers must not invent
/// placeholder limits for such a monitor.
pub(crate) fn monitor_limits(app: &Model, logical_id: &str) -> Option<(u8, u8, f64)> {
    app.config
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == logical_id)
        .map(|monitor| (monitor.min_pct, monitor.max_pct, monitor.transition_gamma))
}

#[allow(clippy::too_many_lines)]
pub(crate) fn render_monitor_detail(
    f: &mut Frame,
    app: &mut Model,
    monitor: &MonitorStatus,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let detail_focused =
        matches!(app.monitor_pane_focus, MonitorPaneFocus::Detail) && app.workspace_focused();
    let (glyph, state_text, state_style) =
        monitor_state(Some(monitor), now_epoch_seconds(), styles);
    let name = display_name(app, &monitor.logical_id);
    let block = kit::with_meta(
        kit::panel(
            &truncate(&name, usize::from(area.width.saturating_sub(30))),
            detail_focused,
            styles,
        ),
        vec![
            Span::styled(monitor.backend.display_label(), styles.text_muted),
            Span::styled(kit::SEPARATOR, styles.border_normal),
            Span::styled(format!("{glyph} {state_text}"), state_style),
        ],
    );
    let inner = block.inner(area);
    // Reset the whole pane so shorter live values cannot leave stale cells.
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let limits = monitor_limits(app, &monitor.logical_id);
    let applied = valid_applied_percent(monitor.last_applied_percent);
    let editing = matches!(app.input_mode, InputMode::Editing);
    let width = inner.width;

    // ── Brightness ────────────────────────────────────────────────────────
    let mut brightness = vec![kit::heading("Brightness", width, styles)];
    brightness.push(kit::data_line(
        "Applied",
        LABEL_WIDTH,
        applied.map_or_else(
            || vec![Span::styled("Unknown", styles.text_muted)],
            |value| {
                let mut value = vec![
                    Span::styled(format!("{} ", kit::DOT), styles.data_value),
                    Span::styled(format!("{value}%"), styles.data_value),
                ];
                if let Some(hold) = monitor.external_hold.filter(|hold| hold.remaining_s > 0) {
                    value.push(Span::styled(
                        format!("  external · {}m", hold.remaining_s.saturating_add(59) / 60),
                        styles.status_warning,
                    ));
                }
                value
            },
        ),
        styles,
    ));
    let target = app
        .monitor_targets_now
        .iter()
        .find(|target| target.logical_id == monitor.logical_id);
    if let Some(target) = target {
        let mut value = vec![Span::styled(
            format!("{}%", target.weather_percent),
            styles.text_primary,
        )];
        if app.monitor_policy_preview_pending() {
            value.push(Span::styled("  updating…", styles.text_muted));
        } else if target.weather_percent != target.solar_percent {
            value.push(Span::styled(
                format!("  {}% before weather", target.solar_percent),
                styles.text_muted,
            ));
        }
        brightness.push(kit::data_line("Target", LABEL_WIDTH, value, styles));
    }
    brightness.push(kit::data_line(
        "Mode",
        LABEL_WIDTH,
        vec![mode_span(app, monitor, styles)],
        styles,
    ));

    // ── Range ─────────────────────────────────────────────────────────────
    let min_focused = detail_focused && app.monitor_control_index == 0;
    let max_focused = detail_focused && app.monitor_control_index == 1;
    let edit_text = app
        .active_input_ref()
        .map(|input| input.value().to_owned())
        .unwrap_or_default();
    let mut range = vec![kit::heading("Brightness range", width, styles)];
    if let Some((min_pct, max_pct, _)) = limits {
        range.push(range_track(
            app,
            min_pct,
            max_pct,
            applied,
            (min_focused, max_focused),
            width,
            styles,
        ));
        range.push(kit::field_line(
            "Minimum",
            LABEL_WIDTH,
            &format!("{min_pct}%"),
            FieldKind::Adjustable,
            FieldState::new(min_focused, min_focused && editing),
            Some(&edit_text),
            styles,
        ));
        range.push(kit::field_line(
            "Maximum",
            LABEL_WIDTH,
            &format!("{max_pct}%"),
            FieldKind::Adjustable,
            FieldState::new(max_focused, max_focused && editing),
            Some(&edit_text),
            styles,
        ));
    } else {
        range.push(kit::data_line(
            "Range",
            LABEL_WIDTH,
            vec![Span::styled("Not in configuration", styles.status_warning)],
            styles,
        ));
    }

    // ── Automation ────────────────────────────────────────────────────────
    let mut automation = vec![kit::heading("Automation", width, styles)];
    let now_local = app.current_local_time();
    if let Some(next) = app.next_automation_milestone(&now_local) {
        let when = if next.is_tomorrow {
            format!("tomorrow {}", next.time_str)
        } else {
            next.time_str.clone()
        };
        automation.push(kit::data_line(
            "Next",
            LABEL_WIDTH,
            vec![
                Span::styled(next.milestone_label, styles.text_primary),
                Span::styled(format!(" at {when} → "), styles.text_muted),
                Span::styled(format!("{}%", next.target_percent), styles.text_primary),
            ],
            styles,
        ));
    }
    if let Some((_, _, gamma)) = limits {
        automation.push(kit::data_line(
            "Gamma",
            LABEL_WIDTH,
            vec![Span::styled(format!("{gamma:.2}"), styles.text_primary)],
            styles,
        ));
    }

    // Sections collapse from the bottom when the terminal is short.
    let sections = [brightness, range, automation];
    let mut lines: Vec<Line> = Vec::new();
    let mut range_row = None;
    for (index, section) in sections.into_iter().enumerate() {
        let gap = usize::from(!lines.is_empty());
        if lines.len() + gap + section.len() > usize::from(inner.height) {
            // The range controls are the interactive core; keep them even if
            // Brightness must shrink to its first rows.
            if index == 1 && lines.len() > 2 {
                let keep = usize::from(inner.height).saturating_sub(section.len() + 1);
                lines.truncate(keep.max(1));
            } else {
                break;
            }
        }
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        if index == 1 {
            range_row = Some(lines.len());
        }
        lines.extend(section);
    }
    f.render_widget(Paragraph::new(lines), inner);

    if detail_focused && editing {
        if let (Some(start), Some(input)) = (range_row, app.active_input_ref()) {
            let offset = 2 + app.monitor_control_index.min(1) as u16;
            let row = Rect::new(inner.x, inner.y + start as u16 + offset, inner.width, 1);
            if row.y < inner.y + inner.height {
                kit::place_field_cursor(f, row, LABEL_WIDTH, input.visual_cursor());
            }
        }
    }
}

fn mode_span(app: &Model, monitor: &MonitorStatus, styles: &SemanticStyles) -> Span<'static> {
    if !monitor.enabled {
        return Span::styled("Disabled", styles.text_muted);
    }
    if let Some(value) = monitor.override_percent {
        return Span::styled(
            format!("Manual override · {value}%"),
            styles.status_warning.add_modifier(Modifier::BOLD),
        );
    }
    match app.operational_mode() {
        OperationalMode::Automatic => Span::styled("Automatic", styles.status_success),
        OperationalMode::Override => Span::styled(
            "Manual override",
            styles.status_warning.add_modifier(Modifier::BOLD),
        ),
        OperationalMode::Suspended => Span::styled(
            "Suspended · writes paused",
            styles.status_warning.add_modifier(Modifier::BOLD),
        ),
        OperationalMode::IdleDimmed => Span::styled("Idle dimmed", styles.status_warning),
        OperationalMode::Offline => Span::styled("Daemon offline", styles.status_error),
    }
}

/// One absolute 0–100 instrument: the allowed band between two handles, and
/// the applied brightness as `●` when it is known.
fn range_track(
    app: &Model,
    min_pct: u8,
    max_pct: u8,
    applied: Option<u8>,
    focus: (bool, bool),
    width: u16,
    styles: &SemanticStyles,
) -> Line<'static> {
    let track = usize::from(width).saturating_sub(2 + 2 + 4);
    if track < 10 {
        return Line::from("");
    }
    let position = |percent: u8| (usize::from(percent) * (track - 1) + 50) / 100;
    let min_index = position(min_pct);
    let max_index = position(max_pct).max(min_index);

    let now = Instant::now();
    let settling = |min: bool| {
        app.motion
            .active_transient
            .as_ref()
            .is_some_and(|transient| {
                matches!(transient.kind, TransientKind::RangeCommit { min: m } if m == min)
                    && transient.phase(now).is_some()
            })
    };

    let mut cells: Vec<(String, ratatui::style::Style)> = (0..track)
        .map(|index| {
            if index > min_index && index < max_index {
                (String::from("━"), styles.gauge_fill)
            } else {
                (String::from("─"), styles.gauge_track)
            }
        })
        .collect();
    if let Some(applied) = applied {
        cells[position(applied)] = (kit::DOT.to_owned(), styles.data_value);
    }
    let handle = |focused: bool, settle: bool| {
        if focused {
            (String::from("█"), styles.focus_marker)
        } else if settle {
            (String::from("┃"), styles.status_success)
        } else {
            (String::from("┃"), styles.data_value)
        }
    };
    cells[min_index] = handle(focus.0, settling(true));
    cells[max_index] = if max_index == min_index {
        handle(focus.0 || focus.1, settling(true) || settling(false))
    } else {
        handle(focus.1, settling(false))
    };

    let mut spans = vec![
        Span::raw(kit::BLANK_CURSOR),
        Span::styled("0 ", styles.text_muted),
    ];
    spans.extend(
        cells
            .into_iter()
            .map(|(symbol, style)| Span::styled(symbol, style)),
    );
    spans.push(Span::styled(" 100", styles.text_muted));
    Line::from(spans)
}
