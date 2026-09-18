use std::time::Instant;

use chrono::{DateTime, Duration, FixedOffset, Timelike};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    policy::{AutomationMilestone, MonitorMilestoneSchedule},
    solar::{self, Location},
    tui::{
        model::{AutomationRegionFocus, CurvePreview, CURVE_SAMPLE_MINUTES},
        theme::SemanticStyles,
        InputMode, Model,
    },
};

use super::{
    kit::{self, FieldKind, FieldState},
    light_cycle,
    monitors::{
        display_name, monitor_ids, monitor_limits, render_monitor_workspace_state,
        valid_applied_percent,
    },
};

const FIELD_LABEL_WIDTH: usize = 12;
const PANEL_GAP: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationLayout {
    ThreeRegion,
    Compact,
    Minimal,
}

#[derive(Debug, Clone, Default)]
pub(super) struct CycleAnchors {
    pub(super) dawn: Option<DateTime<FixedOffset>>,
    pub(super) sunrise: Option<DateTime<FixedOffset>>,
    pub(super) noon: Option<DateTime<FixedOffset>>,
    pub(super) sunset: Option<DateTime<FixedOffset>>,
    pub(super) dusk: Option<DateTime<FixedOffset>>,
}

/// Renders one selected monitor's controls, today's solar instrument, and
/// editable schedule. The visual layers deliberately keep astronomical timing
/// separate from the policy response sampled by the background preview job.
pub(super) fn render_automation(
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
        render_monitor_workspace_state(f, app, area, styles);
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

fn render_controls_panel(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
    compact: bool,
) {
    if area.width < 18 || area.height < 3 {
        return;
    }
    let focused = matches!(
        app.automation_focus,
        AutomationRegionFocus::Selector | AutomationRegionFocus::Curve
    ) && app.workspace_focused();
    let ids = monitor_ids(app);
    let selected = app.selected_monitor.min(ids.len().saturating_sub(1));
    let meta = if ids.is_empty() {
        Vec::new()
    } else {
        vec![Span::styled(
            format!("{} / {}", selected + 1, ids.len()),
            styles.text_muted,
        )]
    };
    let block = kit::with_meta(kit::panel("Automation", focused, styles), meta);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let Some((_, _, shape)) = monitor_limits(app, logical_id) else {
        f.render_widget(
            Paragraph::new(Span::styled("Not configured", styles.status_warning)),
            inner,
        );
        return;
    };

    if compact || inner.height < 18 {
        render_compact_controls(f, app, logical_id, inner, shape, styles);
    } else {
        render_full_controls(f, app, logical_id, inner, shape, styles);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_compact_controls(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    inner: Rect,
    shape: f64,
    styles: &SemanticStyles,
) {
    let mut lines = vec![monitor_selector_line(app, usize::from(inner.width), styles)];
    if inner.height >= 2 {
        lines.push(curve_shape_line(app, shape, styles));
    }
    if inner.height >= 3 {
        lines.push(target_line(app, logical_id, inner.width, styles));
    }
    f.render_widget(Paragraph::new(lines), inner);
    place_curve_cursor(f, app, inner, 1);
}

#[allow(clippy::too_many_arguments)]
fn render_full_controls(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    inner: Rect,
    shape: f64,
    styles: &SemanticStyles,
) {
    let width = inner.width;
    let row = |offset: u16, height: u16| Rect::new(inner.x, inner.y + offset, width, height);
    let roomy = inner.height >= 26;
    let gap = if roomy { 2 } else { 1 };

    f.render_widget(
        Paragraph::new(kit::heading("Monitor", width, styles)),
        row(0, 1),
    );
    render_monitor_box(f, app, row(1, 3), styles);
    let mut y = 4;
    if roomy {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  [ ] switch from anywhere",
                styles.text_muted,
            ))),
            row(y, 1),
        );
        y += 1;
    }

    y += gap;
    f.render_widget(
        Paragraph::new(kit::heading("Solar curvature", width, styles)),
        row(y, 1),
    );
    let curve_row = y + 1;
    f.render_widget(
        Paragraph::new(vec![
            curve_shape_line(app, shape, styles),
            shape_track_line(app, shape, width, styles),
            track_labels_line("Brighter", "Dimmer", width, styles),
        ]),
        row(curve_row, 3),
    );
    y = curve_row + 3;
    if roomy {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Lower stays bright longer",
                styles.text_muted,
            ))),
            row(y, 1),
        );
        y += 1;
    }

    let output_top = y + gap;
    let output_height = inner.height.saturating_sub(output_top).min(6);
    if output_height >= 2 {
        render_target_instrument(f, app, logical_id, row(output_top, output_height), styles);
    }
    place_curve_cursor(f, app, inner, curve_row);
}

/// The selected monitor in a rounded field, like a drop-down: `←` `→` cycle
/// it while the Monitor row has focus.
fn render_monitor_box(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    let ids = monitor_ids(app);
    let focused =
        matches!(app.automation_focus, AutomationRegionFocus::Selector) && app.workspace_focused();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            styles.border_focused
        } else {
            styles.border_normal
        });
    let inner = block.inner(area);
    f.render_widget(block, area);
    if ids.is_empty() || inner.width < 6 {
        f.render_widget(
            Paragraph::new(Span::styled(" No monitor", styles.text_muted)),
            inner,
        );
        return;
    }
    let selected = app.selected_monitor.min(ids.len() - 1);
    let many = ids.len() > 1;
    let switcher = if many {
        format!("‹ {}/{} ›", selected + 1, ids.len())
    } else {
        String::new()
    };
    let name_width = usize::from(inner.width).saturating_sub(kit::cell_width(&switcher) + 4);
    let name = super::truncate(&display_name(app, &ids[selected]), name_width);
    let name_style = if focused {
        styles.value_capsule_focused
    } else {
        styles.text_primary.add_modifier(Modifier::BOLD)
    };
    let used = kit::cell_width(&name) + 2 + 1 + kit::cell_width(&switcher);
    let spans = vec![
        Span::styled(format!(" {name} "), name_style),
        Span::raw(" ".repeat(usize::from(inner.width).saturating_sub(used))),
        Span::styled(
            switcher,
            if focused {
                styles.focus_marker
            } else {
                styles.text_muted
            },
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), inner);
}

/// Gamma (`transition_gamma`) on a logarithmic track: 1.0 sits in the middle.
/// Lower values lift the whole daylight transition, so brightness rises
/// sooner in the morning and falls later in the evening; higher values do
/// the opposite.
fn shape_track_line(app: &Model, shape: f64, width: u16, styles: &SemanticStyles) -> Line<'static> {
    let track = usize::from(width).saturating_sub(4);
    if track < 6 {
        return Line::from("");
    }
    let focused =
        matches!(app.automation_focus, AutomationRegionFocus::Curve) && app.workspace_focused();
    let max = crate::config::MAX_TRANSITION_GAMMA.ln();
    let position = f64::midpoint(shape.max(0.01).ln() / max, 1.0).clamp(0.0, 1.0);
    let handle = (position * (track - 1) as f64).round() as usize;
    let middle = (track - 1) / 2;
    let mut spans = vec![Span::raw(kit::BLANK_CURSOR)];
    for index in 0..track {
        if index == handle {
            spans.push(Span::styled(
                kit::DOT,
                if focused {
                    styles.focus_marker
                } else {
                    styles.gauge_fill
                },
            ));
        } else if index < handle {
            spans.push(Span::styled("━", styles.gauge_fill));
        } else if index == middle {
            spans.push(Span::styled("┼", styles.gauge_track));
        } else {
            spans.push(Span::styled("─", styles.gauge_track));
        }
    }
    Line::from(spans)
}

fn track_labels_line(
    left: &str,
    right: &str,
    width: u16,
    styles: &SemanticStyles,
) -> Line<'static> {
    let track = usize::from(width).saturating_sub(4);
    let gap = track.saturating_sub(kit::cell_width(left) + kit::cell_width(right));
    Line::from(vec![
        Span::raw(kit::BLANK_CURSOR),
        Span::styled(left.to_owned(), styles.text_muted),
        Span::raw(" ".repeat(gap)),
        Span::styled(right.to_owned(), styles.text_muted),
    ])
}

fn monitor_selector_line(app: &Model, width: usize, styles: &SemanticStyles) -> Line<'static> {
    let ids = monitor_ids(app);
    if ids.is_empty() {
        return Line::from(Span::styled("No monitor selected", styles.text_muted));
    }
    let selected = app.selected_monitor.min(ids.len() - 1);
    let focused =
        matches!(app.automation_focus, AutomationRegionFocus::Selector) && app.workspace_focused();
    let many = ids.len() > 1;
    // The full control surface already has a Monitor heading. At the tightest
    // width, spend those cells on the complete friendly display name instead
    // of clipping it for a repeated inline label.
    let show_label = width >= 27;
    let show_count = width >= 34;
    let fixed =
        2 + usize::from(show_label) * 8 + usize::from(many) * 4 + 2 + usize::from(show_count) * 7;
    let name_width = width.saturating_sub(fixed).clamp(4, 24);
    let name = super::truncate(&display_name(app, &ids[selected]), name_width);
    let label_style = if focused {
        styles.text_primary.add_modifier(Modifier::BOLD)
    } else {
        styles.data_label
    };
    let value_style = if focused {
        styles.value_capsule_focused
    } else {
        styles.text_primary.add_modifier(Modifier::BOLD)
    };
    let arrow_style = if focused {
        styles.focus_marker
    } else {
        styles.text_muted
    };
    let mut spans = vec![kit::cursor_span(focused, styles)];
    if show_label {
        spans.push(Span::styled("Monitor ", label_style));
    }
    if many {
        spans.push(Span::styled("‹ ", arrow_style));
    }
    spans.push(Span::styled(format!(" {name} "), value_style));
    if many {
        spans.push(Span::styled(" ›", arrow_style));
    }
    if show_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} / {}", selected + 1, ids.len()),
            label_style,
        ));
    }
    Line::from(spans)
}

fn curve_shape_line(app: &Model, shape: f64, styles: &SemanticStyles) -> Line<'static> {
    let focused =
        matches!(app.automation_focus, AutomationRegionFocus::Curve) && app.workspace_focused();
    let editing = focused && matches!(app.input_mode, InputMode::Editing);
    let edit_text = app
        .active_input_ref()
        .map(|input| input.value().to_owned())
        .unwrap_or_default();
    kit::field_line(
        "Gamma",
        FIELD_LABEL_WIDTH,
        &format!("{shape:.2}"),
        FieldKind::Adjustable,
        FieldState::new(focused, editing),
        Some(&edit_text),
        styles,
    )
}

fn place_curve_cursor(f: &mut Frame, app: &Model, inner: Rect, row_offset: u16) {
    if matches!(app.automation_focus, AutomationRegionFocus::Curve)
        && matches!(app.input_mode, InputMode::Editing)
    {
        if let Some(input) = app.active_input_ref() {
            kit::place_field_cursor(
                f,
                Rect::new(inner.x, inner.y + row_offset, inner.width, 1),
                FIELD_LABEL_WIDTH,
                input.visual_cursor(),
            );
        }
    }
}

fn target_line(
    app: &Model,
    logical_id: &str,
    width: u16,
    styles: &SemanticStyles,
) -> Line<'static> {
    let pending = app.monitor_policy_preview_pending();
    let label = target_label(pending, width);
    let label_width = kit::cell_width(label).max(FIELD_LABEL_WIDTH);
    match target_preview(app, logical_id) {
        Some(target) => kit::data_line(
            label,
            label_width,
            vec![
                Span::styled(format!("{} ", kit::DOT), styles.current_marker),
                Span::styled(
                    format!("{}%", target.weather_percent),
                    if pending {
                        styles.text_muted
                    } else {
                        styles.data_value
                    },
                ),
            ],
            styles,
        ),
        None => kit::data_line(
            label,
            label_width,
            vec![Span::styled("Waiting", styles.text_muted)],
            styles,
        ),
    }
}

fn target_label(pending: bool, width: u16) -> &'static str {
    if pending && width >= 27 {
        "Target updating…"
    } else if pending {
        "Target…"
    } else {
        "Target now"
    }
}

/// Whether the target is about to rise, fall, or hold over the next quarter hour.
fn output_trend(app: &Model, logical_id: &str) -> Option<std::cmp::Ordering> {
    let curve = app
        .monitor_curves
        .iter()
        .find(|curve| curve.logical_id == logical_id)?;
    let minute = minute_of_day(&app.current_local_time());
    let now = Instant::now();
    let here = curve_value_at(app, curve, minute, now);
    let soon = curve_value_at(app, curve, (minute + 15.0).min(1440.0), now);
    Some(if soon - here >= 0.5 {
        std::cmp::Ordering::Greater
    } else if here - soon >= 0.5 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Equal
    })
}

/// The brightness the monitor is at: the confirmed applied value, or the
/// computed target until the daemon has written one. Full effects count
/// between values when a new write lands.
fn render_target_instrument(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    let applied = applied_percent(app, logical_id);
    let target = target_preview(app, logical_id).map(|target| target.weather_percent);
    if area.width < 16 || area.height < 5 {
        let value = applied
            .or(target)
            .map_or_else(|| String::from("—"), |v| format!("{v}%"));
        f.render_widget(
            Paragraph::new(vec![
                kit::heading("Output", area.width, styles),
                Line::from(vec![
                    Span::raw(kit::BLANK_CURSOR),
                    Span::styled(value, styles.text_heading.add_modifier(Modifier::BOLD)),
                ]),
            ]),
            area,
        );
        return;
    }

    let now = Instant::now();
    let roll = app.motion.output_roll(logical_id, now);
    let palette = styles.palette;
    let mut lines = vec![kit::heading("Output", area.width, styles)];
    match applied.or(target) {
        Some(value) => {
            let (shown, glow) = roll.map_or((value, None), |(shown, phase)| (shown, Some(phase)));
            // New values flash toward the foreground colour and settle back
            // to the accent as the count completes.
            let color = glow.map_or(palette.accent, |phase| {
                crate::tui::theme::mix(palette.fg, palette.accent, f64::from(phase))
            });
            let digit_style = if applied.is_some() {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                styles.text_muted
            };
            let rows = super::fonts::rounded_number_rows(&shown.to_string());
            for (index, row) in rows.into_iter().enumerate() {
                let mut spans = vec![Span::raw(kit::BLANK_CURSOR), Span::styled(row, digit_style)];
                if index == 2 {
                    spans.push(Span::styled(" %", digit_style));
                }
                lines.push(Line::from(spans));
            }
            let caption = if applied.is_none() {
                vec![Span::styled("Target · not written yet", styles.text_muted)]
            } else {
                match output_trend(app, logical_id) {
                    Some(std::cmp::Ordering::Greater) => vec![
                        Span::styled("▲ ", styles.focus_marker),
                        Span::styled("Brightening", styles.text_muted),
                    ],
                    Some(std::cmp::Ordering::Less) => vec![
                        Span::styled("▼ ", styles.focus_marker),
                        Span::styled("Dimming", styles.text_muted),
                    ],
                    _ => vec![
                        Span::styled(format!("{} ", kit::DOT), styles.current_marker),
                        Span::styled("Steady", styles.text_muted),
                    ],
                }
            };
            if area.height >= 6 {
                let mut spans = vec![Span::raw(kit::BLANK_CURSOR)];
                spans.extend(caption);
                lines.push(Line::from(spans));
            }
        }
        None => lines.push(Line::from(vec![
            Span::raw(kit::BLANK_CURSOR),
            Span::styled("Waiting for the daemon", styles.text_muted),
        ])),
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn applied_percent(app: &Model, logical_id: &str) -> Option<u8> {
    app.status
        .as_ref()
        .and_then(|status| {
            status
                .monitors
                .iter()
                .find(|monitor| monitor.logical_id == logical_id)
        })
        .and_then(|monitor| valid_applied_percent(monitor.last_applied_percent))
}

pub(super) fn render_small_cycle(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    schedule: Option<&MonitorMilestoneSchedule>,
    anchors: &CycleAnchors,
    area: Rect,
    styles: &SemanticStyles,
) {
    let noon = anchors
        .noon
        .as_ref()
        .map_or_else(|| String::from("—"), |time| format_time(time, false));
    let peak = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::Peak))
        .map_or_else(
            || String::from("—"),
            |entry| format!("{}%", entry.target_percent),
        );
    let target = target_preview(app, logical_id).map_or_else(
        || String::from("—"),
        |target| format!("{}%", target.weather_percent),
    );
    let text = format!("Solar noon {noon} · peak {peak}");
    let now = format_time(&app.current_local_time(), app.config.tui.use_12h_time);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                super::truncate(&text, usize::from(area.width)),
                styles.text_primary.add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(format!("{} NOW ", kit::DOT), styles.current_marker),
                Span::styled(now, styles.text_primary),
                Span::styled(kit::SEPARATOR, styles.border_normal),
                Span::styled("Target ", styles.data_label),
                Span::styled(target, styles.data_value),
            ]),
        ]),
        area,
    );
}

pub(super) fn cycle_anchors(
    app: &Model,
    schedule: Option<&MonitorMilestoneSchedule>,
) -> CycleAnchors {
    let status_sunrise = app
        .status
        .as_ref()
        .and_then(|status| status.sunrise_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let status_sunset = app
        .status
        .as_ref()
        .and_then(|status| status.sunset_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let date = app.current_local_time().date_naive();
    let solar_events = Location::from_timezone_name(
        app.config.location.latitude,
        app.config.location.longitude,
        &app.config.location.timezone,
    )
    .ok()
    .and_then(|location| solar::get_sun_events(date, &location).ok());

    let fallback_sunrise = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::RiseStart))
        .map(|entry| entry.base_time_local);
    let fallback_noon = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::Peak))
        .map(|entry| entry.base_time_local);
    let fallback_sunset = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::NightFloor))
        .map(|entry| entry.base_time_local - Duration::minutes(90));

    CycleAnchors {
        dawn: solar_events.as_ref().map(|events| events.dawn),
        sunrise: status_sunrise
            .or_else(|| solar_events.as_ref().map(|events| events.sunrise))
            .or(fallback_sunrise),
        noon: solar_events
            .as_ref()
            .map(|events| events.noon)
            .or(fallback_noon),
        sunset: status_sunset
            .or_else(|| solar_events.as_ref().map(|events| events.sunset))
            .or(fallback_sunset),
        dusk: solar_events.as_ref().map(|events| events.dusk),
    }
}

pub(super) fn cycle_bounds(anchors: &CycleAnchors) -> [f64; 7] {
    let mut dawn = anchors.dawn.as_ref().map_or(300.0, minute_of_day);
    let mut sunrise = anchors.sunrise.as_ref().map_or(360.0, minute_of_day);
    let mut noon = anchors.noon.as_ref().map_or(720.0, minute_of_day);
    let mut sunset = anchors.sunset.as_ref().map_or(1080.0, minute_of_day);
    let mut dusk = anchors.dusk.as_ref().map_or(1140.0, minute_of_day);
    dawn = dawn.clamp(0.0, 1435.0);
    sunrise = sunrise.clamp(dawn + 1.0, 1436.0);
    noon = noon.clamp(sunrise + 1.0, 1437.0);
    sunset = sunset.clamp(noon + 1.0, 1438.0);
    dusk = dusk.clamp(sunset + 1.0, 1439.0);
    [0.0, dawn, sunrise, noon, sunset, dusk, 1440.0]
}

pub(super) fn milestone(
    schedule: &MonitorMilestoneSchedule,
    kind: AutomationMilestone,
) -> Option<&crate::policy::MonitorMilestone> {
    schedule
        .milestones
        .iter()
        .find(|entry| entry.milestone == kind)
}

pub(super) fn target_preview<'a>(
    app: &'a Model,
    logical_id: &str,
) -> Option<&'a crate::tui::model::TargetPreview> {
    app.monitor_targets_now
        .iter()
        .find(|target| target.logical_id == logical_id)
}

pub(super) fn minute_of_day(time: &DateTime<FixedOffset>) -> f64 {
    f64::from(time.hour() * 60 + time.minute())
}

pub(super) fn curve_value_at(app: &Model, curve: &CurvePreview, minute: f64, now: Instant) -> f64 {
    let current = curve_sample_value(curve, minute);
    let Some(phase) = app.motion.curve_morph_phase(now) else {
        return current;
    };
    let Some(previous) = app.curve_morph_from.as_ref() else {
        return current;
    };
    if previous.logical_id != curve.logical_id || previous.samples.len() != curve.samples.len() {
        return current;
    }
    let eased = f64::from(1.0 - (1.0 - phase).powi(3));
    let old = curve_sample_value(previous, minute);
    old + (current - old) * eased
}

fn curve_sample_value(curve: &CurvePreview, minute: f64) -> f64 {
    let minute = minute.clamp(0.0, 1440.0);
    let sample = minute / f64::from(CURVE_SAMPLE_MINUTES);
    let index = sample.floor() as usize;
    let fraction = sample - index as f64;
    match (curve.samples.get(index), curve.samples.get(index + 1)) {
        (Some(a), Some(b)) => f64::from(*a) + (f64::from(*b) - f64::from(*a)) * fraction,
        (Some(a), None) => f64::from(*a),
        _ => curve.samples.last().copied().map_or(0.0, f64::from),
    }
}

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

fn render_schedule_panel(
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
pub(super) fn current_milestone_index(
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

pub(super) fn format_time(time: &DateTime<FixedOffset>, use_12h: bool) -> String {
    if use_12h {
        time.format("%I:%M %p").to_string()
    } else {
        time.format("%H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::tui::command::{
        commands_for_footer, commands_for_workspace, CommandId, UiCommandContext,
    };
    use crate::tui::motion::{TransientKind, UiMotionState};
    use chrono::{DateTime, FixedOffset, TimeZone};
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::time::{Duration, Instant};
    use tui_input::Input;

    use crate::backends::BackendKind;
    use crate::policy::{AutomationMilestone, MonitorMilestone, MonitorMilestoneSchedule};
    use crate::tui::actions;
    use crate::tui::app::ModelEnvironment;
    use crate::tui::form::FormState;
    use crate::tui::model::{AutomationRegionFocus, DaemonConnection, Tab};
    use crate::tui::test_support::{
        buffer_row, buffer_text, configure_monitor_fixture, dummy_status, find_in_buffer,
        named_monitor_config, press, two_monitor_model,
    };
    use crate::tui::update;
    use crate::tui::{ui, Model};
    #[test]
    fn test_advanced_milestones_table_structure_comfortable() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Milestones;

        // Provide milestone schedule
        let offset = FixedOffset::east_opt(3 * 3600).unwrap();
        let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
            offset
                .with_ymd_and_hms(2026, 9, 10, h, m, 0)
                .single()
                .unwrap()
        };
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![
                MonitorMilestone {
                    milestone: AutomationMilestone::RiseStart,
                    base_time_local: make_time(6, 11),
                    adjusted_time_local: make_time(6, 11),
                    target_percent: 7,
                    minutes_offset: 0,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::Rise25,
                    base_time_local: make_time(7, 56),
                    adjusted_time_local: make_time(8, 4),
                    target_percent: 34,
                    minutes_offset: 8,
                },
            ],
        }];

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. Column headers
        assert!(find_in_buffer(&buffer, "Event").is_some());
        assert!(find_in_buffer(&buffer, "Offset").is_some());
        assert!(find_in_buffer(&buffer, "Time").is_some());
        assert!(find_in_buffer(&buffer, "Target").is_some());

        // 2. Data rows
        assert!(find_in_buffer(&buffer, "Rise Start").is_some());
        assert!(find_in_buffer(&buffer, "06:11").is_some());
        assert!(find_in_buffer(&buffer, "Rise 25%").is_some());
        assert!(find_in_buffer(&buffer, "+8m").is_some());
        assert!(
            find_in_buffer(&buffer, "08:04").is_some(),
            "{}",
            buffer_text(&buffer)
        );
        assert!(find_in_buffer(&buffer, "34%").is_some());

        // 3. The whole schedule is one framed functional region.
        assert!(find_in_buffer(&buffer, "Schedule").is_some());

        // Columns are allocated from display widths with a two-cell gap, so
        // adjacent headers and values can never merge (e.g. "Night Floor20:45").
        assert!(buffer_text(&buffer)
            .lines()
            .any(|line| line.contains("Offset") && line.contains("Time")));
        assert!(!buffer_text(&buffer).contains("Rise Start06:11"));
    }

    #[test]
    fn test_advanced_milestones_selection_and_current_markers() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let now = model.current_local_time();
        let past = now - chrono::Duration::hours(1);
        let future = now + chrono::Duration::hours(1);
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![
                MonitorMilestone {
                    milestone: AutomationMilestone::RiseStart,
                    base_time_local: past,
                    adjusted_time_local: past, // In past so it is current
                    target_percent: 7,
                    minutes_offset: 0,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::Rise25,
                    base_time_local: future,
                    adjusted_time_local: future, // In future
                    target_percent: 34,
                    minutes_offset: 0,
                },
            ],
        }];

        // Row 0 is both selected and current
        model.selected_monitor_milestone = 0;
        model.automation_focus = AutomationRegionFocus::Milestones;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        assert!(
            find_in_buffer(&buffer, "▸● Rise Start").is_some(),
            "{}",
            buffer_text(&buffer)
        );

        // Switch selection to Row 1: the cursor and the current anchor separate.
        model.selected_monitor_milestone = 1;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer2 = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer2, "▸  Rise 25%").is_some());
        assert!(find_in_buffer(&buffer2, " ● Rise Start").is_some());
    }

    #[test]
    fn test_advanced_milestones_adjustment_and_reset() {
        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.selected_monitor = 0;
        model.selected_monitor_milestone = 0;
        model.config.monitors = vec![crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            ..Default::default()
        }];

        let offset = FixedOffset::east_opt(3 * 3600).unwrap();
        let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
            offset
                .with_ymd_and_hms(2026, 9, 10, h, m, 0)
                .single()
                .unwrap()
        };
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![MonitorMilestone {
                milestone: AutomationMilestone::RiseStart,
                base_time_local: make_time(6, 11),
                adjusted_time_local: make_time(6, 11),
                target_percent: 7,
                minutes_offset: 0,
            }],
        }];

        // 1. Adjust +1m
        actions::adjust_selected_monitor_milestone(&mut model, 1);
        let sched = model.selected_monitor_schedule().unwrap();
        assert_eq!(sched.milestones[0].minutes_offset, 1);
        assert_eq!(sched.milestones[0].base_time_local, make_time(6, 11)); // Base unchanged
        assert_eq!(sched.milestones[0].target_percent, 7); // Target unchanged

        // 2. Reset with r
        actions::reset_selected_monitor_milestone(&mut model);
        let sched_reset = model.selected_monitor_schedule().unwrap();
        assert_eq!(sched_reset.milestones[0].minutes_offset, 0);
        assert_eq!(
            sched_reset.milestones[0].adjusted_time_local,
            make_time(6, 11)
        );
    }

    #[test]
    fn test_advanced_milestones_responsive_compact_and_minimal() {
        // Compact: 60x18
        {
            let backend = TestBackend::new(60, 18);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Limits;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());
        }

        // Minimal: 42x12
        {
            let backend = TestBackend::new(42, 12);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Limits;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());
        }
    }

    #[test]
    fn test_advanced_milestones_non_amber_theme() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = crate::config::Theme::Nord;
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }

    #[test]
    fn test_automation_instrument_smoke_across_all_themes() {
        use ratatui::style::Color;

        for theme in crate::tui::theme::Theme::ALL {
            let mut model = Model::with_environment(ModelEnvironment::isolated());
            configure_monitor_fixture(
                &mut model,
                vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
            );
            model.config.location.city = String::from("Istanbul, TR");
            model.config.location.latitude = 41.0082;
            model.config.location.longitude = 28.9784;
            model.config.location.timezone = String::from("Europe/Istanbul");
            model.config.tui.theme = theme;
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Limits;
            model.automation_focus = AutomationRegionFocus::Curve;
            model.motion.level = crate::config::MotionLevel::Off;
            model.refresh_monitor_milestones();

            let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();
            let text = buffer_text(&buffer);
            assert!(text.contains("Today's light cycle"), "theme {theme:?}");
            assert!(text.contains("Solar noon"), "theme {theme:?}");
            assert!(text.contains("Brightness target"), "theme {theme:?}");
            assert!(text.contains("Day length"), "theme {theme:?}");
            assert!(text.contains("Next event"), "theme {theme:?}");
            assert!(text.contains("Sunrise"), "theme {theme:?}");
            assert!(text.contains("Schedule"), "theme {theme:?}");
            assert!(text.contains("Gamma"), "theme {theme:?}");
            assert!(text.contains("Output"), "theme {theme:?}");
            assert!(
                text.contains("Steady") || text.contains("Brightening") || text.contains("Dimming"),
                "theme {theme:?}"
            );
            assert!(!text.contains("Arc solar time"), "theme {theme:?}");
            assert!(!text.contains("ribbon brighter"), "theme {theme:?}");

            let palette = theme.palette();
            let area = buffer
                .content
                .iter()
                .filter(|cell| cell.symbol() == "█" && cell.fg != Color::Reset)
                .count();
            assert!(area > 20, "theme {theme:?} lost the target area");
            assert!(
                buffer.content.iter().all(|cell| !cell
                    .symbol()
                    .chars()
                    .any(|character| ('\u{2801}'..='\u{28ff}').contains(&character))),
                "theme {theme:?} Automation should not use dot-matrix glyphs"
            );

            let (x, y, cell) = find_in_buffer(&buffer, "0.50").expect("focused Curve value");
            assert_eq!(cell.bg, palette.accent, "theme {theme:?}");
            assert_ne!(cell.bg, cell.fg, "theme {theme:?} focus lost contrast");
            assert_ne!(cell.fg, Color::Reset, "theme {theme:?} reset foreground");
            assert!(x < buffer.area.width && y < buffer.area.height);
        }
    }

    #[test]
    fn test_automation_chart_survives_comfortable_masthead() {
        let mut model = Model::with_environment(ModelEnvironment::isolated());
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.0082;
        model.config.location.longitude = 28.9784;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.motion.level = crate::config::MotionLevel::Off;
        model.refresh_monitor_milestones();

        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("Today's light cycle"), "{text}");
        assert!(text.contains("Brightness target"), "{text}");
        assert!(text.contains("Sunrise"), "{text}");
        assert!(
            text.contains("Afternoon") || text.contains("Morning") || text.contains("Night"),
            "{text}"
        );
    }

    #[test]
    fn test_automation_target_instrument_never_invents_applied_brightness() {
        let mut model = Model::with_environment(ModelEnvironment::isolated());
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        let mut status = dummy_status(1);
        status.monitors[0].last_applied_percent = Some(44);
        model.status = Some(status);
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.motion.level = crate::config::MotionLevel::Off;
        model.refresh_monitor_milestones();

        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
        let known = buffer_text(terminal.backend().buffer());
        // 44 in the three-row digit font: `╷ ╷ ╷ ╷` over `╰─┤ ╰─┤`.
        assert!(known.contains("Output"), "{known}");
        assert!(known.contains("╰─┤ ╰─┤"), "{known}");
        assert!(!known.contains("not written yet"), "{known}");

        model.status.as_mut().expect("fixture status").monitors[0].last_applied_percent = None;
        terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
        let unknown = buffer_text(terminal.backend().buffer());
        assert!(unknown.contains("Target · not written yet"), "{unknown}");
        assert!(!unknown.contains("╰─┤ ╰─┤"), "{unknown}");
    }

    #[test]
    fn test_automation_responsive_priority_drops_instrument_before_identity() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Limits;
        model.motion.level = crate::config::MotionLevel::Off;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let text = buffer_text(terminal.backend().buffer());
        assert!(
            text.contains("Mi Monitor"),
            "selected monitor must remain readable"
        );
        assert!(text.contains("Gamma"));
        assert!(text.contains("Target now"));
        assert!(text.contains("Schedule"));
        assert!(!text.contains("Today's light cycle"));
    }

    #[test]
    fn test_preflight_constrained_milestone_detection() {
        let base = FixedOffset::east_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 10, 6, 0, 0)
            .unwrap();

        // 1. Unconstrained positive offset: base 06:00, offset +15m -> adjusted 06:15
        let m1 = MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: base,
            adjusted_time_local: base + chrono::Duration::minutes(15),
            target_percent: 7,
            minutes_offset: 15,
        };
        assert!(!m1.is_constrained(15));

        // 2. Unconstrained negative offset: base 06:00, offset -15m -> adjusted 05:45
        let m2 = MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: base,
            adjusted_time_local: base - chrono::Duration::minutes(15),
            target_percent: 7,
            minutes_offset: -15,
        };
        assert!(!m2.is_constrained(-15));

        // 3. Constrained by adjacent milestone ordering:
        // Requested offset is +90m (07:30), but adjusted time was clamped to 07:00
        let m3 = MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: base,
            adjusted_time_local: base + chrono::Duration::minutes(60),
            target_percent: 7,
            minutes_offset: 60,
        };
        assert!(m3.is_constrained(90));

        // 4. Day boundary constraint:
        // Requested offset is -30m (yesterday 23:30), but clamped to day start 00:00
        let m4 = MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: base,
            adjusted_time_local: base - chrono::Duration::minutes(10),
            target_percent: 7,
            minutes_offset: -10,
        };
        assert!(m4.is_constrained(-30));

        // 5. Offset reset:
        // User reset offset to 0, but milestone is clamped forward by another event
        let m5 = MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: base,
            adjusted_time_local: base + chrono::Duration::minutes(10),
            target_percent: 7,
            minutes_offset: 10,
        };
        assert!(m5.is_constrained(0));
    }

    #[test]
    fn test_limits_workspace_comfortable_multiple_monitors() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![
                named_monitor_config("mon-0", "Mi Monitor", 5, 60),
                named_monitor_config("mon-1", "Lenovo P24h-20", 5, 60),
            ],
        );
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let offset = FixedOffset::east_opt(3 * 3600).unwrap();
        let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
            offset
                .with_ymd_and_hms(2026, 9, 10, h, m, 0)
                .single()
                .unwrap()
        };
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![
                MonitorMilestone {
                    milestone: AutomationMilestone::RiseStart,
                    base_time_local: make_time(6, 11),
                    adjusted_time_local: make_time(6, 11),
                    minutes_offset: 0,
                    target_percent: 7,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::Peak,
                    base_time_local: make_time(12, 45),
                    adjusted_time_local: make_time(12, 45),
                    minutes_offset: 0,
                    target_percent: 60,
                },
            ],
        }];

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();

        // Tab 2 is a full-width Automation workspace with explicit monitor context.
        assert!(find_in_buffer(&buffer, "Automation").is_some());
        assert!(find_in_buffer(&buffer, "Monitor").is_some());
        assert!(find_in_buffer(&buffer, "Gamma").is_some());
        assert!(find_in_buffer(&buffer, "Event").is_some());
        assert!(find_in_buffer(&buffer, "Time").is_some());
        assert!(find_in_buffer(&buffer, "Target").is_some());
    }

    #[test]
    fn test_limits_workspace_long_and_unicode_names() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.monitors = vec![
            crate::config::MonitorConfig {
                logical_id: String::from(
                    "very-long-monitor-name-that-exceeds-column-width-and-must-truncate",
                ),
                backend: BackendKind::Ddc,
                min_pct: 10,
                max_pct: 80,
                ..Default::default()
            },
            crate::config::MonitorConfig {
                logical_id: String::from("İstanbul-Ekranı-Çalışma"),
                backend: BackendKind::Backlight,
                min_pct: 5,
                max_pct: 50,
                ..Default::default()
            },
        ];
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Limits;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }

    #[test]
    fn test_phase9_4_automation_monitor_switching_cycle() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.selected_monitor = 0;

        // Press ']' to switch to next monitor
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(']'),
            )),
        );
        assert_eq!(model.selected_monitor, 1);

        // Press '[' to switch back
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('['),
            )),
        );
        assert_eq!(model.selected_monitor, 0);

        // Context sharing: switch to Tab 1 retains selected_monitor
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(']'),
            )),
        );
        assert_eq!(model.selected_monitor, 1);

        model.active_tab = Tab::Monitors;
        assert_eq!(model.selected_monitor, 1);
    }

    #[test]
    fn test_phase9_4_automation_curve_shape_interactive_control() {
        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![
                named_monitor_config("mon-0", "Mi Monitor", 5, 60),
                named_monitor_config("mon-1", "LEN P24h-20", 8, 90),
            ],
        );
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Milestones;
        model.selected_monitor_milestone = 0;

        // Up from milestone 0 moves focus to Curve shape
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Up,
            )),
        );
        assert_eq!(model.automation_focus, AutomationRegionFocus::Curve);

        // Right on curve steps gamma by +0.05
        let prev_gamma = model.config.monitors[0].transition_gamma;
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );
        let new_gamma = model.config.monitors[0].transition_gamma;
        assert!((new_gamma - (prev_gamma + 0.05)).abs() < 1e-4);

        // Down moves back to Milestones
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Down,
            )),
        );
        assert_eq!(model.automation_focus, AutomationRegionFocus::Milestones);
    }

    #[test]
    fn test_phase9_4_automation_switch_updates_context_curve_and_target() {
        let mut model = Model::new();
        model.config.monitors = vec![
            crate::config::MonitorConfig {
                logical_id: String::from("mon-0"),
                min_pct: 7,
                max_pct: 60,
                transition_gamma: 0.5,
                ..Default::default()
            },
            crate::config::MonitorConfig {
                logical_id: String::from("mon-1"),
                min_pct: 15,
                max_pct: 90,
                transition_gamma: 2.0,
                ..Default::default()
            },
        ];
        model.form = FormState::new(&model.config);
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
            .single()
            .unwrap();
        model.monitor_milestones = vec![
            MonitorMilestoneSchedule {
                logical_id: String::from("mon-0"),
                milestones: vec![MonitorMilestone {
                    milestone: AutomationMilestone::Rise50,
                    base_time_local: time,
                    adjusted_time_local: time,
                    target_percent: 22,
                    minutes_offset: 0,
                }],
            },
            MonitorMilestoneSchedule {
                logical_id: String::from("mon-1"),
                milestones: vec![MonitorMilestone {
                    milestone: AutomationMilestone::Rise50,
                    base_time_local: time,
                    adjusted_time_local: time,
                    target_percent: 78,
                    minutes_offset: 0,
                }],
            },
        ];

        let mut first_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        first_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let first = first_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&first, "mon-0").is_some());
        assert!(find_in_buffer(&first, "0.50").is_some());
        assert!(find_in_buffer(&first, "22%").is_some());

        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(']'),
            )),
        );
        assert_eq!(model.selected_monitor, 1);

        let mut second_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        second_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let second = second_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&second, "mon-1").is_some());
        assert!(find_in_buffer(&second, "2.00").is_some());
        assert!(find_in_buffer(&second, "78%").is_some());
    }

    #[test]
    fn test_automation_monitor_chips_switch_with_left_right() {
        let mut model = two_monitor_model();
        model.switch_to_tab(Tab::Limits);
        assert_eq!(
            model.automation_focus,
            AutomationRegionFocus::Selector,
            "with several monitors Automation opens on the monitor chips"
        );

        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let (_, row, _) = find_in_buffer(&buffer, "Mi Monitor").unwrap();
        assert!(buffer_row(&buffer, row).contains("Mi Monitor"));
        assert!(buffer_row(&buffer, row).contains("›"), "{buffer:?}");
        assert!(find_in_buffer(&buffer, "Brightness target").is_some());
        assert!(find_in_buffer(&buffer, "Output").is_some());
        let footer = buffer_row(&buffer, 31);
        assert!(footer.contains("Monitor"), "{footer}");

        press(&mut model, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(model.selected_monitor, 1);
        press(&mut model, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(model.selected_monitor, 0);

        press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(model.automation_focus, AutomationRegionFocus::Curve);
        press(&mut model, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(model.automation_focus, AutomationRegionFocus::Selector);
        press(&mut model, KeyCode::Up, KeyModifiers::NONE);
        assert!(model.tabs_focused);
    }

    #[test]
    fn test_automation_three_region_layout_survives_the_chrome_gutter() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Limits;
        model.motion.level = crate::config::MotionLevel::Off;
        let mut terminal = Terminal::new(TestBackend::new(114, 32)).unwrap();

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let (cycle_x, cycle_y, _) = find_in_buffer(&buffer, "Today's light cycle").expect("cycle");
        let (schedule_x, schedule_y, _) = find_in_buffer(&buffer, "Schedule").expect("schedule");

        assert_eq!(
            cycle_y, schedule_y,
            "schedule should remain beside the cycle"
        );
        assert!(
            schedule_x > cycle_x,
            "schedule should be the right-hand region"
        );
    }

    #[test]
    fn test_next_automation_event_calculations() {
        let mut model = Model::new();
        let offset = FixedOffset::east_opt(3 * 3600).unwrap();
        let make_time = |hour: u32, minute: u32| -> DateTime<FixedOffset> {
            offset
                .with_ymd_and_hms(2026, 9, 10, hour, minute, 0)
                .single()
                .unwrap()
        };

        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![
                MonitorMilestone {
                    milestone: AutomationMilestone::RiseStart,
                    base_time_local: make_time(6, 0),
                    adjusted_time_local: make_time(6, 0),
                    target_percent: 7,
                    minutes_offset: 0,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::Rise25,
                    base_time_local: make_time(7, 0),
                    adjusted_time_local: make_time(7, 15),
                    target_percent: 25,
                    minutes_offset: 15,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::Peak,
                    base_time_local: make_time(12, 0),
                    adjusted_time_local: make_time(12, 0),
                    target_percent: 60,
                    minutes_offset: 0,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::NightFloor,
                    base_time_local: make_time(21, 0),
                    adjusted_time_local: make_time(21, 0),
                    target_percent: 7,
                    minutes_offset: 0,
                },
            ],
        }];

        let next_dawn = model
            .next_automation_milestone(&make_time(4, 0))
            .expect("next milestone found");
        assert_eq!(next_dawn.milestone_label, "Rise Start");
        assert_eq!(next_dawn.time_str, "06:00");
        assert_eq!(next_dawn.target_percent, 7);
        assert!(!next_dawn.is_tomorrow);

        let next_sunrise = model
            .next_automation_milestone(&make_time(6, 30))
            .expect("next milestone found");
        assert_eq!(next_sunrise.milestone_label, "Rise 25%");
        assert_eq!(next_sunrise.time_str, "07:15");
        assert_eq!(next_sunrise.minutes_offset, 15);
        assert_eq!(next_sunrise.target_percent, 25);
        assert!(!next_sunrise.is_tomorrow);

        let next_peak = model
            .next_automation_milestone(&make_time(7, 15))
            .expect("next milestone found");
        assert_eq!(next_peak.milestone_label, "Peak");
        assert_eq!(next_peak.time_str, "12:00");
        assert_eq!(next_peak.target_percent, 60);

        let next_tomorrow = model
            .next_automation_milestone(&make_time(22, 0))
            .expect("next milestone found");
        assert_eq!(next_tomorrow.milestone_label, "Rise Start");
        assert_eq!(next_tomorrow.time_str, "06:00");
        assert!(next_tomorrow.is_tomorrow);
    }

    #[test]
    fn test_phase5_6_automation_milestone_micro_settle() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let now = Instant::now();
        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.motion = UiMotionState::new_at(now);

        // 1. Milestone adjustment triggers micro-settle for milestone index 0
        model.motion.trigger(
            TransientKind::MilestoneAdjust { index: 0 },
            now,
            Duration::from_millis(250),
        );
        assert!(model.motion.milestone_adjust_phase(now, 0).is_some());
        assert!(model.motion.milestone_adjust_phase(now, 1).is_none());

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        // 2. After 250ms, micro-settle self-expires
        model.motion.tick(now + Duration::from_millis(300));
        assert!(model
            .motion
            .milestone_adjust_phase(now + Duration::from_millis(300), 0)
            .is_none());
    }

    #[test]
    fn test_phase9_4_curve_upper_bound_uses_config_authority() {
        let mut model = Model::new();
        model.config.monitors = vec![crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            transition_gamma: crate::config::MAX_TRANSITION_GAMMA - 0.05,
            ..Default::default()
        }];
        model.form = FormState::new(&model.config);
        model.status = Some(dummy_status(1));

        model.step_monitor_curve(0.05);
        assert!(
            (model.config.monitors[0].transition_gamma - crate::config::MAX_TRANSITION_GAMMA).abs()
                < f64::EPSILON
        );

        // Repeating the key at the cap is a no-op rather than creating an invalid config.
        model.step_monitor_curve(0.05);
        assert!(
            (model.config.monitors[0].transition_gamma - crate::config::MAX_TRANSITION_GAMMA).abs()
                < f64::EPSILON
        );

        model.form.monitor_curve_inputs[0] = Input::default().with_value(String::from("4.01"));
        let error = model
            .form
            .validate_values(&model.config)
            .expect_err("exact entry above config cap must fail");
        assert!(error.contains("0 < gamma <= 4"));
    }

    #[test]
    fn test_phase9_4_shared_policy_produces_monitor_specific_targets() {
        let policy = crate::config::SolarPolicyConfig {
            twilight_elevation_start: -6.0,
            day_elevation_full: 20.0,
            use_adaptive_zenith: false,
            ..Default::default()
        };
        let monitors = vec![
            crate::config::MonitorConfig {
                logical_id: String::from("mon-0"),
                min_pct: 7,
                max_pct: 60,
                transition_gamma: 0.5,
                ..Default::default()
            },
            crate::config::MonitorConfig {
                logical_id: String::from("mon-1"),
                min_pct: 15,
                max_pct: 90,
                transition_gamma: 2.0,
                ..Default::default()
            },
        ];
        let location =
            crate::solar::Location::from_timezone_name(41.0082, 28.9784, "Europe/Istanbul")
                .expect("valid test location");
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
            .single()
            .expect("valid fixed instant");

        let output = crate::policy::compute_policy_for_elevation(
            &crate::policy::PolicyContext {
                now_utc: now,
                location: &location,
                config: &policy,
                monitors: &monitors,
                weather_multiplier: None,
            },
            7.0,
        )
        .expect("shared policy accepts valid monitor fixtures");

        assert_eq!(output.targets.len(), 2);
        assert_ne!(output.targets[0].percent, output.targets[1].percent);
        assert!(
            (output.targets[0].effective_daylight_factor
                - output.targets[1].effective_daylight_factor)
                .abs()
                > f64::EPSILON
        );
    }

    #[test]
    fn test_phase9_4_curve_focus_footer_and_help_are_truthful() {
        let mut model = Model::new();
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Curve;

        let commands = commands_for_footer(&model);
        assert_eq!(commands.context, UiCommandContext::AutomationCurve);
        assert!(commands
            .current_commands
            .iter()
            .any(|command| command.id == CommandId::AdjustGamma));
        assert!(!commands
            .current_commands
            .iter()
            .any(|command| command.id == CommandId::AdjustMilestoneOffset));

        model.show_help = true;
        let help_commands = commands_for_workspace(&model);
        assert_eq!(help_commands.context, UiCommandContext::AutomationCurve);
        assert!(help_commands
            .current_commands
            .iter()
            .any(|command| command.action == "Adjust gamma by 0.05"));
    }

    #[test]
    fn test_phase9_4_curve_focus_hides_milestone_only_inspector() {
        let mut model = Model::new();
        model.config.monitors = vec![crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            min_pct: 7,
            max_pct: 60,
            transition_gamma: 0.5,
            ..Default::default()
        }];
        model.form = FormState::new(&model.config);
        model.status = Some(dummy_status(1));
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Curve;

        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
            .single()
            .unwrap();
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![MonitorMilestone {
                milestone: AutomationMilestone::Rise50,
                base_time_local: time,
                adjusted_time_local: time,
                target_percent: 22,
                minutes_offset: 0,
            }],
        }];

        let mut curve_terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
        curve_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let curve_buffer = curve_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&curve_buffer, "Gamma").is_some());
        assert!(find_in_buffer(&curve_buffer, "±1 min").is_none());
        assert!(find_in_buffer(&curve_buffer, "r Reset").is_none());

        model.automation_focus = AutomationRegionFocus::Milestones;
        let mut milestone_terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
        milestone_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let milestone_buffer = milestone_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&milestone_buffer, "±1 min").is_some());
        assert!(find_in_buffer(&milestone_buffer, "r Reset").is_some());
    }
}
