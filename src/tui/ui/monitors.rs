use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use crate::ipc::MonitorStatus;
use crate::tui::model::{MonitorWorkspaceState, OperationalMode};
use crate::tui::motion::TransientKind;
use crate::tui::theme::SemanticStyles;
use crate::tui::{InputMode, Model, MonitorPaneFocus, Tab};

use super::kit::{self, FieldKind, FieldState};
use super::truncate;

/// Width of the label column in the monitor detail pane.
const LABEL_WIDTH: usize = 12;

/// Renders the Monitors workspace: a selector list and the selected monitor.
pub(super) fn render_monitors(f: &mut Frame, app: &mut Model, area: Rect, styles: &SemanticStyles) {
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
pub(super) fn monitor_ids(app: &Model) -> Vec<String> {
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

pub(super) fn display_name(app: &Model, logical_id: &str) -> String {
    super::monitor_selector::monitor_display_name(app, logical_id)
}

/// One-line selector used where there is no room for the list. `[` and `]`
/// frame the name because they are the keys that change it.
pub(super) fn render_monitor_switcher(
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
            lines.push(kit::detail_line(detail, styles));
            lines.push(Line::from(""));
        } else {
            first.push(Span::styled(format!(" {glyph}"), state_style));
            lines.push(Line::from(first));
        }
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A glyph, a short label, and a style for a monitor's availability.
fn monitor_state(
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
pub(super) fn valid_applied_percent(last_applied: Option<u8>) -> Option<u8> {
    last_applied.filter(|value| *value <= 100)
}

/// The configured range and gamma (`transition_gamma`), or `None` when the daemon reports a
/// monitor this configuration does not describe. Callers must not invent
/// placeholder limits for such a monitor.
pub(super) fn monitor_limits(app: &Model, logical_id: &str) -> Option<(u8, u8, f64)> {
    app.config
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == logical_id)
        .map(|monitor| (monitor.min_pct, monitor.max_pct, monitor.transition_gamma))
}

#[allow(clippy::too_many_lines)]
fn render_monitor_detail(
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
                vec![
                    Span::styled(format!("{} ", kit::DOT), styles.data_value),
                    Span::styled(format!("{value}%"), styles.data_value),
                ]
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

/// Shared empty-state presentation for Monitors and Automation, derived from
/// independent daemon, configuration, and discovery evidence.
#[allow(clippy::too_many_lines)]
pub(super) fn render_monitor_workspace_state(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let setup_focused = app.workspace_focused()
        && (matches!(app.active_tab, Tab::Monitors)
            && matches!(app.monitor_pane_focus, MonitorPaneFocus::List)
            || matches!(app.active_tab, Tab::Limits));
    let action = |label: String| {
        Line::from(vec![
            kit::cursor_span(setup_focused, styles),
            Span::styled(
                label,
                if setup_focused {
                    styles.value_capsule_focused
                } else {
                    styles.data_value
                },
            ),
        ])
    };
    let mut lines = Vec::new();

    let title = match app.monitor_workspace_state() {
        MonitorWorkspaceState::DaemonUnavailable => {
            lines.push(Line::from(Span::styled(
                format!("{} Daemon unavailable", kit::HOLLOW),
                styles.status_error.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "SunReactor is not responding. Start the daemon, then return here.",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::Connecting => {
            lines.push(Line::from(Span::styled(
                "Connecting to the SunReactor daemon…",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::Discovering => {
            lines.push(Line::from(Span::styled(
                "Looking for compatible monitors…",
                styles.text_primary.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Hardware discovery runs in the background.",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::DiscoveredButUnconfigured {
            compatible_count,
            importable_count,
            incomplete_ddc_probe,
        } => {
            lines.push(Line::from(Span::styled(
                format!("{compatible_count} compatible monitor(s) found"),
                styles.text_primary.add_modifier(Modifier::BOLD),
            )));
            if let Some(report) = app.monitor_discovery_report() {
                for target in report.viable_targets().into_iter().take(4) {
                    lines.push(kit::detail_line(
                        vec![Span::styled(target.label.clone(), styles.text_primary)],
                        styles,
                    ));
                }
            }
            lines.push(Line::from(""));
            if importable_count > 0 && !incomplete_ddc_probe {
                lines.push(action(format!(" Add {importable_count} monitor(s) ")));
                lines.push(Line::from(Span::styled(
                    "Saves verified selectors and reloads the daemon.",
                    styles.text_muted,
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    if incomplete_ddc_probe {
                        "A DDC capability check was incomplete. Verify all displays before adding them."
                    } else {
                        "No candidate can be added automatically without weakening topology safety."
                    },
                    styles.status_warning,
                )));
                lines.push(action(String::from(" Scan again ")));
            }
            "Set up monitors"
        }
        MonitorWorkspaceState::NoCompatibleHardware => {
            lines.push(Line::from(Span::styled(
                "No compatible monitors were discovered",
                styles.text_primary.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Connect a brightness-capable display, then scan again.",
                styles.text_muted,
            )));
            lines.push(Line::from(""));
            lines.push(action(String::from(" Scan again ")));
            "Set up monitors"
        }
        MonitorWorkspaceState::ConfiguredUnavailable => {
            lines.push(Line::from(Span::styled(
                format!(
                    "{} Configured monitors are temporarily unavailable",
                    kit::WARN
                ),
                styles.status_warning.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Check display power and connections. Your configuration has not been changed.",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::Ready => {
            lines.push(Line::from(Span::styled(
                "Monitor status is updating…",
                styles.text_muted,
            )));
            "Monitors"
        }
    };

    let block = kit::panel(title, setup_focused, styles);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .style(styles.base)
            .wrap(Wrap { trim: true })
            .block(block),
        area,
    );
}
