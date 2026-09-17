//! Today's light cycle: the central Automation instrument.
//!
//! One shared time axis runs through every layer: the line chart of the
//! brightness target, the sunrise/noon/sunset anchors,
//! and the day-phase band. The chart uses the absolute 0–100 % scale so the
//! monitor's range is never exaggerated.

use std::time::Instant;

use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::{
    policy::{AutomationMilestone, MonitorMilestoneSchedule},
    solar::{self, Location},
    tui::{
        model::AutomationRegionFocus,
        theme::{mix, SemanticStyles},
        Model,
    },
};

use super::{
    automation::{
        curve_value_at, cycle_anchors, cycle_bounds, format_time, milestone, minute_of_day,
        render_small_cycle, target_preview, CycleAnchors,
    },
    kit,
    monitors::monitor_limits,
};

/// Columns reserved on the left of the chart for the percent labels.
const GUTTER: u16 = 5;
/// Tallest chart, in rows; spare height becomes breathing room instead.
const MAX_CHART_ROWS: u16 = 18;
const MIN_CHART_ROWS: u16 = 4;
/// Optional rows give way until the chart reaches this height.
const PREFERRED_CHART_ROWS: u16 = 9;

/// Lower block elements from empty to seven eighths.
pub(super) const EIGHTHS: [&str; 8] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇"];

const PHASE_NAMES: [&str; 6] = ["Night", "Dawn", "Morning", "Afternoon", "Dusk", "Night"];

pub(super) fn render_light_cycle_panel(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width < 24 || area.height < 4 {
        return;
    }
    let now = app.current_local_time();
    let block = kit::with_meta(
        kit::panel("Today's light cycle", false, styles),
        vec![Span::styled(
            now.format("%a %b %-d").to_string(),
            styles.text_muted,
        )],
    );
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if monitor_limits(app, logical_id).is_none() {
        f.render_widget(
            Paragraph::new(Span::styled("Policy unavailable", styles.text_muted)),
            inner,
        );
        return;
    }
    let schedule = app
        .monitor_milestones
        .iter()
        .find(|schedule| schedule.logical_id == logical_id);
    let anchors = cycle_anchors(app, schedule);
    if inner.height < 7 || inner.width < 30 {
        render_small_cycle(f, app, logical_id, schedule, &anchors, inner, styles);
        return;
    }

    let rows = CycleRows::plan(inner.height);
    let mut y = inner.y;
    let mut take = |height: u16| {
        let rect = Rect::new(inner.x, y, inner.width, height);
        y += height;
        rect
    };
    let legend = take(rows.legend);
    let chart = take(rows.chart);
    let axis = take(1);
    take(rows.gap_a);
    let events = take(rows.events);
    take(rows.gap_b);
    let band = take(1);
    let labels = take(rows.labels);
    let now_row = take(rows.now_row);
    take(rows.gap_c);
    let info = take(rows.info);

    let sky = SkyContext::new(app, &now);
    let plot = |rect: Rect| {
        Rect::new(
            rect.x + GUTTER,
            rect.y,
            rect.width.saturating_sub(GUTTER + 1),
            rect.height,
        )
    };
    render_legend(f, app, logical_id, legend, styles);
    render_chart(f, app, logical_id, schedule, &sky, chart, styles);
    render_axis(f, app, plot(axis), styles);
    render_events(f, app, schedule, &anchors, events, styles);
    render_phase_band(f, &sky, &anchors, plot(band), styles);
    render_phase_labels(f, &sky, &anchors, plot(labels), styles);
    render_now_marker(f, &sky, plot(now_row), styles);
    render_info(f, app, &anchors, info, styles);
}

/// Row budget for the panel, dropping optional rows before the chart shrinks
/// below a readable height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CycleRows {
    legend: u16,
    chart: u16,
    gap_a: u16,
    events: u16,
    gap_b: u16,
    labels: u16,
    now_row: u16,
    gap_c: u16,
    info: u16,
}

impl CycleRows {
    fn plan(height: u16) -> Self {
        let mut rows = Self {
            legend: 1,
            chart: 0,
            gap_a: 1,
            events: 2,
            gap_b: 1,
            labels: 1,
            now_row: 1,
            gap_c: 1,
            info: 3,
        };
        // Axis and phase band are always present.
        let fixed = |rows: &Self| {
            2 + rows.legend
                + rows.gap_a
                + rows.events
                + rows.gap_b
                + rows.labels
                + rows.now_row
                + rows.gap_c
                + rows.info
        };
        let drops: [fn(&mut Self); 7] = [
            |rows| {
                rows.info = 0;
                rows.gap_c = 0;
            },
            |rows| rows.gap_a = 0,
            |rows| rows.gap_b = 0,
            |rows| rows.now_row = 0,
            |rows| rows.events = 0,
            |rows| rows.legend = 0,
            |rows| rows.labels = 0,
        ];
        for drop in drops {
            let wanted = if rows.info > 0 || rows.gap_a > 0 || rows.gap_b > 0 {
                PREFERRED_CHART_ROWS
            } else {
                MIN_CHART_ROWS
            };
            if height >= fixed(&rows) + wanted {
                break;
            }
            drop(&mut rows);
        }
        let available = height.saturating_sub(fixed(&rows));
        rows.chart = available.min(MAX_CHART_ROWS);
        // Spare rows go around the anchors rather than stretching the chart.
        let spare = available - rows.chart;
        if rows.events > 0 {
            rows.gap_a += spare / 2;
            rows.gap_b += spare - spare / 2;
        } else {
            rows.gap_a += spare;
        }
        rows
    }
}

/// Solar geometry for today, shared by every layer that follows the sun.
struct SkyContext {
    location: Option<Location>,
    date: chrono::NaiveDate,
    now_minute: f64,
}

impl SkyContext {
    fn new(app: &Model, now: &DateTime<FixedOffset>) -> Self {
        Self {
            location: Location::from_timezone_name(
                app.config.location.latitude,
                app.config.location.longitude,
                &app.config.location.timezone,
            )
            .ok(),
            date: now.date_naive(),
            now_minute: minute_of_day(now),
        }
    }

    fn elevation(&self, minute: f64) -> Option<f64> {
        let location = self.location.as_ref()?;
        let minute = minute.clamp(0.0, 1439.0) as u32;
        let local: NaiveDateTime = self.date.and_hms_opt(minute / 60, minute % 60, 0)?;
        solar::get_solar_elevation(local, location).ok()
    }
}

fn render_legend(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.height == 0 {
        return;
    }
    let palette = styles.palette;
    let mut spans = vec![
        Span::styled("━ ", Style::default().fg(palette.accent)),
        Span::styled("Brightness target", styles.text_muted),
    ];
    if let Some(target) = target_preview(app, logical_id) {
        spans.push(Span::styled(
            format!(" {}%", target.weather_percent),
            if app.monitor_policy_preview_pending() {
                styles.text_muted
            } else {
                styles.data_value.add_modifier(Modifier::BOLD)
            },
        ));
    }
    spans.push(Span::raw(" "));
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Right),
        area,
    );
}

/// Row for a percentage on the absolute 0–100 % scale; row 0 is 100 %.
fn row_of(percent: f64, height: u16) -> u16 {
    let span = f64::from(height.saturating_sub(1));
    (span * (1.0 - percent.clamp(0.0, 100.0) / 100.0)).round() as u16
}

#[allow(clippy::too_many_lines)]
fn render_chart(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    schedule: Option<&MonitorMilestoneSchedule>,
    sky: &SkyContext,
    area: Rect,
    styles: &SemanticStyles,
) {
    let plot = Rect::new(
        area.x + GUTTER,
        area.y,
        area.width.saturating_sub(GUTTER + 1),
        area.height,
    );
    if plot.width < 8 || plot.height < 2 {
        return;
    }
    let palette = styles.palette;
    let base = styles.base;
    let grid_style = base.fg(mix(palette.border_inactive, palette.bg, 0.45));
    let body_top = mix(palette.accent, palette.bg, 0.05);
    let body_bottom = mix(palette.accent, palette.bg, 0.78);

    let curve = app
        .monitor_curves
        .iter()
        .find(|curve| curve.logical_id == logical_id);
    let instant = Instant::now();
    let value_at = |minute: f64| curve.map(|curve| curve_value_at(app, curve, minute, instant));

    let buffer = f.buffer_mut();
    let mut put = |column: u16, row: u16, symbol: &str, style: Style| {
        if column < plot.width && row < plot.height {
            buffer
                .get_mut(plot.x + column, plot.y + row)
                .set_symbol(symbol)
                .set_style(style);
        }
    };

    // Quiet quarter lines first; every later layer draws over them.
    for percent in [25.0, 50.0, 75.0, 100.0] {
        let row = row_of(percent, plot.height);
        for column in 0..plot.width {
            put(column, row, "┈", grid_style);
        }
    }

    // The target as a solid area with eighth-cell tops, shaded by height so
    // brighter hours read brighter.
    let total_eighths = u32::from(plot.height) * 8;
    let mut area_top: Vec<Option<u16>> = vec![None; usize::from(plot.width)];
    let mut area_colors: Vec<Vec<Option<Color>>> =
        vec![vec![None; usize::from(plot.height)]; usize::from(plot.width)];
    for column in 0..plot.width {
        let minute = (f64::from(column) + 0.5) / f64::from(plot.width) * 1440.0;
        let Some(value) = value_at(minute) else {
            continue;
        };
        let eighths = ((value.clamp(0.0, 100.0) / 100.0) * f64::from(total_eighths)).round() as u32;
        let eighths = eighths.max(1);
        let full = (eighths / 8) as u16;
        let partial = (eighths % 8) as usize;
        let color_at = |row: u16| {
            let fraction = f64::from(row) / f64::from(plot.height.max(2) - 1);
            mix(body_top, body_bottom, fraction)
        };
        for step in 0..full {
            let row = plot.height - 1 - step;
            let color = color_at(row);
            area_colors[usize::from(column)][usize::from(row)] = Some(color);
            put(column, row, "█", base.fg(color));
        }
        let top_row = if partial > 0 && full < plot.height {
            let row = plot.height - 1 - full;
            put(column, row, EIGHTHS[partial], base.fg(color_at(row)));
            row
        } else {
            plot.height - full
        };
        area_top[usize::from(column)] = Some(top_row);
    }
    let filled = |column: u16, row: u16| {
        area_colors
            .get(usize::from(column))
            .and_then(|rows| rows.get(usize::from(row)).copied().flatten())
    };

    // Now: a thin column through the area.
    let now_column = column_of(sky.now_minute, usize::from(plot.width)) as u16;
    for row in 0..plot.height {
        let dim = mix(palette.fg, palette.bg, 0.35);
        let style = filled(now_column, row).map_or(base.fg(dim), |bg| base.bg(bg).fg(palette.fg));
        put(now_column, row, "│", style);
    }

    // Markers float just above the area so they never cut into it. Only the
    // milestone selected in the schedule is marked; the table lists the rest.
    let above = |column: u16| {
        area_top
            .get(usize::from(column))
            .copied()
            .flatten()
            .map(|row| row.saturating_sub(1))
    };
    if let (Some(schedule), true) = (schedule, curve.is_some()) {
        let selected = matches!(app.automation_focus, AutomationRegionFocus::Milestones)
            && app.workspace_focused();
        let selected_index = app
            .selected_monitor_milestone
            .min(schedule.milestones.len().saturating_sub(1));
        let entry = schedule
            .milestones
            .get(selected_index)
            .filter(|entry| selected && entry.adjusted_time_local.date_naive() == sky.date);
        if let Some(entry) = entry {
            let minute = minute_of_day(&entry.adjusted_time_local);
            let column = column_of(minute, usize::from(plot.width)) as u16;
            if let Some(row) = above(column) {
                put(column, row, "◆", styles.focus_marker);
            }
        }
    }
    if let Some(row) = above(now_column) {
        put(
            now_column,
            row,
            kit::DOT,
            base.fg(palette.fg).add_modifier(Modifier::BOLD),
        );
    }

    // Percent labels on their grid rows.
    let mut used_rows = Vec::new();
    for percent in [100_u16, 75, 50, 25, 0] {
        let row = row_of(f64::from(percent), plot.height);
        if used_rows.contains(&row) {
            continue;
        }
        used_rows.push(row);
        f.render_widget(
            Paragraph::new(Span::styled(format!("{percent:>3}%"), styles.text_muted)),
            Rect::new(area.x, area.y + row, GUTTER - 1, 1),
        );
    }

    // "Now" rides on the top row when the line stays clear of it.
    let top_free = area_top.iter().flatten().all(|row| *row > 1);
    if top_free {
        let label = format!(
            " Now {} ",
            format_time(&app.current_local_time(), app.config.tui.use_12h_time)
        );
        let width = kit::cell_width(&label) as u16;
        if width + 2 < plot.width {
            let start = now_column
                .saturating_sub(width / 2)
                .min(plot.width.saturating_sub(width));
            f.render_widget(
                Paragraph::new(Span::styled(
                    label,
                    styles.text_primary.add_modifier(Modifier::BOLD),
                ))
                .style(base),
                Rect::new(plot.x + start, plot.y, width, 1),
            );
        }
    }
}

/// Hours between axis labels so each label keeps a little air.
fn hour_step(plot_width: u16) -> usize {
    [2_usize, 3, 4, 6, 12]
        .into_iter()
        .find(|step| usize::from(plot_width) * step / 24 >= 8)
        .unwrap_or(12)
}

fn render_axis(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.width < 8 || area.height == 0 {
        return;
    }
    let width = usize::from(area.width);
    let mut row = vec![' '; width];
    let mut next_free = 0;
    for hour in (0..=24).step_by(hour_step(area.width)) {
        let label = if app.config.tui.use_12h_time {
            let display = match hour % 12 {
                0 => 12,
                other => other,
            };
            let suffix = if hour % 24 < 12 { "am" } else { "pm" };
            format!("{display}{suffix}")
        } else {
            format!("{hour:02}:00")
        };
        let length = label.chars().count();
        let center = ((f64::from(hour) / 24.0) * (width - 1) as f64).round() as usize;
        let start = center
            .saturating_sub(length / 2)
            .min(width.saturating_sub(length));
        if start < next_free {
            continue;
        }
        for (offset, character) in label.chars().enumerate() {
            row[start + offset] = character;
        }
        next_free = start + length + 1;
    }
    f.render_widget(
        Paragraph::new(Span::styled(
            row.into_iter().collect::<String>(),
            styles.text_muted,
        )),
        area,
    );
}

fn render_events(
    f: &mut Frame,
    app: &Model,
    schedule: Option<&MonitorMilestoneSchedule>,
    anchors: &CycleAnchors,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.height == 0 || area.width < 24 {
        return;
    }
    let use_12h = app.config.tui.use_12h_time;
    let time = |value: Option<&DateTime<FixedOffset>>| {
        value.map_or_else(|| String::from("—"), |time| format_time(time, use_12h))
    };
    let peak = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::Peak))
        .map(|entry| format!("  {}%", entry.target_percent))
        .unwrap_or_default();
    let items = [
        (
            "↑",
            "Sunrise",
            time(anchors.sunrise.as_ref()),
            String::new(),
        ),
        (
            kit::BRAND_MARK,
            "Solar noon",
            time(anchors.noon.as_ref()),
            peak,
        ),
        ("↓", "Sunset", time(anchors.sunset.as_ref()), String::new()),
    ];
    let column_width = area.width / 3;
    for (index, (icon, label, value, extra)) in items.into_iter().enumerate() {
        let x = area.x + column_width * index as u16;
        let width = if index == 2 {
            area.width - column_width * 2
        } else {
            column_width
        };
        let mut lines = vec![Line::from(vec![
            Span::styled(format!("{icon} "), styles.focus_marker),
            Span::styled(label, styles.text_muted),
        ])];
        if area.height >= 2 {
            lines.push(Line::from(vec![
                Span::styled(value, styles.text_primary.add_modifier(Modifier::BOLD)),
                Span::styled(extra, styles.data_value),
            ]));
        }
        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            Rect::new(x, area.y, width, area.height),
        );
        if index > 0 {
            for row in 0..area.height {
                f.buffer_mut()
                    .get_mut(x, area.y + row)
                    .set_symbol("│")
                    .set_style(styles.border_normal);
            }
        }
    }
}

/// Phase index for a minute of the day: night, dawn, morning, afternoon,
/// dusk, night.
pub(super) fn phase_at(minute: f64, bounds: &[f64; 7]) -> usize {
    bounds
        .windows(2)
        .position(|window| minute >= window[0] && minute < window[1])
        .unwrap_or(5)
}

/// A band coloured by how high the sun is, so twilight glows between night
/// and day instead of switching abruptly.
fn render_phase_band(
    f: &mut Frame,
    sky: &SkyContext,
    anchors: &CycleAnchors,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width < 8 || area.height == 0 {
        return;
    }
    let palette = styles.palette;
    let bounds = cycle_bounds(anchors);
    let width = usize::from(area.width);
    let now_column = column_of(sky.now_minute, width);
    let mut spans = Vec::with_capacity(width);
    for column in 0..width {
        let minute = (column as f64 + 0.5) / width as f64 * 1440.0;
        let light = sky.elevation(minute).map_or_else(
            || {
                if (bounds[2]..bounds[4]).contains(&minute) {
                    1.0
                } else {
                    0.0
                }
            },
            |elevation| smoothstep(((elevation + 6.0) / 16.0).clamp(0.0, 1.0)),
        );
        let color = if light < 0.5 {
            mix(
                palette.border_inactive,
                palette.secondary_accent,
                light * 2.0,
            )
        } else {
            mix(
                palette.secondary_accent,
                palette.accent,
                (light - 0.5) * 2.0,
            )
        };
        if column == now_column {
            spans.push(Span::styled("┃", Style::default().fg(palette.fg)));
        } else {
            spans.push(Span::styled("▆", Style::default().fg(color)));
        }
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_phase_labels(
    f: &mut Frame,
    sky: &SkyContext,
    anchors: &CycleAnchors,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width < 8 || area.height == 0 {
        return;
    }
    let bounds = cycle_bounds(anchors);
    let width = usize::from(area.width);
    let current = phase_at(sky.now_minute, &bounds);
    let mut occupied = vec![false; width];
    let mut placed: Vec<(usize, usize)> = Vec::new();
    // Broad phases claim space first; short twilight labels fill in if they fit.
    for index in [2, 3, 0, 5, 1, 4] {
        let label = PHASE_NAMES[index];
        let length = label.chars().count();
        let start_column = column_of(bounds[index], width);
        let end_column = column_of(bounds[index + 1], width);
        let center = usize::midpoint(start_column, end_column);
        let start = center
            .saturating_sub(length / 2)
            .min(width.saturating_sub(length));
        let lo = start.saturating_sub(1);
        let hi = (start + length + 1).min(width);
        if length > width || occupied[lo..hi].iter().any(|taken| *taken) {
            continue;
        }
        occupied[start..start + length].fill(true);
        placed.push((start, index));
    }
    placed.sort_unstable();
    let mut spans = Vec::new();
    let mut cursor = 0;
    for (start, index) in placed {
        spans.push(Span::raw(" ".repeat(start - cursor)));
        let label = PHASE_NAMES[index];
        let style =
            if index == current || (current == 5 && index == 0 && sky.now_minute < bounds[1]) {
                styles.text_primary.add_modifier(Modifier::BOLD)
            } else {
                styles.text_muted
            };
        spans.push(Span::styled(label, style));
        cursor = start + label.chars().count();
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_now_marker(f: &mut Frame, sky: &SkyContext, area: Rect, styles: &SemanticStyles) {
    if area.width < 8 || area.height == 0 {
        return;
    }
    let width = usize::from(area.width);
    let column = column_of(sky.now_minute, width);
    let line = if column + 5 <= width {
        Line::from(vec![
            Span::raw(" ".repeat(column)),
            Span::styled("▲", styles.focus_marker),
            Span::styled(" Now", styles.text_muted),
        ])
    } else {
        Line::from(vec![
            Span::raw(" ".repeat(column.saturating_sub(4))),
            Span::styled("Now ", styles.text_muted),
            Span::styled("▲", styles.focus_marker),
        ])
    };
    f.render_widget(Paragraph::new(line), area);
}

fn render_info(
    f: &mut Frame,
    app: &Model,
    anchors: &CycleAnchors,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.height < 3 || area.width < 30 {
        return;
    }
    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(usize::from(area.width)),
            styles.border_normal,
        )),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let location = &app.config.location;
    let city = if location.city.trim().is_empty() {
        String::from("Configured location")
    } else {
        location.city.clone()
    };
    let latitude = format!(
        "{:.1}° {}",
        location.latitude.abs(),
        if location.latitude < 0.0 { "S" } else { "N" }
    );
    let longitude = format!(
        "{:.1}° {}",
        location.longitude.abs(),
        if location.longitude < 0.0 { "W" } else { "E" }
    );
    let half = area.width / 2;
    let left = vec![
        Line::from(Span::styled(
            super::truncate(&city, usize::from(half.saturating_sub(3))),
            styles.text_primary.add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("{latitude} · {longitude}"),
            styles.text_muted,
        )),
    ];

    let use_12h = app.config.tui.use_12h_time;
    let day_length = match (&anchors.sunrise, &anchors.sunset) {
        (Some(sunrise), Some(sunset)) if sunset > sunrise => {
            let minutes = (*sunset - *sunrise).num_minutes();
            format!("{}h {:02}m", minutes / 60, minutes % 60)
        }
        _ => String::from("—"),
    };
    let twilight = match (&anchors.dawn, &anchors.dusk) {
        (Some(dawn), Some(dusk)) => format!(
            "Twilight {}–{}",
            format_time(dawn, use_12h),
            format_time(dusk, use_12h)
        ),
        _ => String::new(),
    };
    let right = vec![
        Line::from(vec![
            Span::styled("Day length ", styles.text_muted),
            Span::styled(day_length, styles.text_primary.add_modifier(Modifier::BOLD)),
        ]),
        Line::from(Span::styled(twilight, styles.text_muted)),
    ];
    let body = Rect::new(area.x, area.y + 1, area.width, 2);
    f.render_widget(
        Paragraph::new(left),
        Rect::new(body.x + 1, body.y, half.saturating_sub(2), 2),
    );
    for row in 0..2 {
        f.buffer_mut()
            .get_mut(body.x + half, body.y + row)
            .set_symbol("│")
            .set_style(styles.border_normal);
    }
    f.render_widget(
        Paragraph::new(right),
        Rect::new(body.x + half + 2, body.y, area.width - half - 2, 2),
    );
}

/// Minutes until `time` from `now`, rolling a passed time over to tomorrow.
pub(super) fn minutes_until(now: &DateTime<FixedOffset>, time: &DateTime<FixedOffset>) -> i64 {
    let mut target = *time;
    while target <= *now {
        target += Duration::days(1);
    }
    (target - *now).num_minutes()
}

fn column_of(minute: f64, width: usize) -> usize {
    ((minute.clamp(0.0, 1440.0) / 1440.0) * width.saturating_sub(1) as f64).round() as usize
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::{hour_step, phase_at, CycleRows};

    #[test]
    fn row_plan_keeps_the_chart_readable_and_caps_its_height() {
        for height in 7..60 {
            let rows = CycleRows::plan(height);
            let total = rows.legend
                + rows.chart
                + 1
                + rows.gap_a
                + rows.events
                + rows.gap_b
                + 1
                + rows.labels
                + rows.now_row
                + rows.gap_c
                + rows.info;
            assert!(total <= height, "height {height}: {rows:?}");
            assert!(rows.chart >= 2, "height {height}: {rows:?}");
            assert!(rows.chart <= super::MAX_CHART_ROWS);
        }
        let tall = CycleRows::plan(35);
        assert_eq!(tall.info, 3);
        assert_eq!(tall.events, 2);
    }

    #[test]
    fn axis_labels_never_crowd() {
        assert_eq!(hour_step(120), 2);
        assert_eq!(hour_step(60), 4);
        assert_eq!(hour_step(20), 12);
    }

    #[test]
    fn phases_follow_the_bounds() {
        let bounds = [0.0, 300.0, 330.0, 700.0, 1070.0, 1100.0, 1440.0];
        assert_eq!(phase_at(10.0, &bounds), 0);
        assert_eq!(phase_at(310.0, &bounds), 1);
        assert_eq!(phase_at(800.0, &bounds), 3);
        assert_eq!(phase_at(1200.0, &bounds), 5);
    }
}
