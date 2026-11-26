use std::time::Instant;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use super::super::kit::{FieldKind, FieldState};
use super::super::monitors::{display_name, monitor_ids, monitor_limits, valid_applied_percent};
use super::super::{fonts, kit};
use super::cycle::{curve_value_at, minute_of_day, target_preview};
use crate::tui::{model::AutomationRegionFocus, theme::SemanticStyles, InputMode, Model};

const FIELD_LABEL_WIDTH: usize = 12;

pub(super) fn render_controls_panel(
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
    let name = super::super::truncate(&display_name(app, &ids[selected]), name_width);
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
    let name = super::super::truncate(&display_name(app, &ids[selected]), name_width);
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
            let rows = match app.config.tui.theme.glyph_style() {
                crate::theme::GlyphStyle::Pixel => {
                    fonts::weather_pixel_font_rows(&shown.to_string())
                }
                crate::theme::GlyphStyle::Rounded => fonts::rounded_number_rows(&shown.to_string()),
            };
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
