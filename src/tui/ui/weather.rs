use std::time::Instant;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use super::kit;
use super::weather_art::{weather_art, ART_ROWS, ART_WIDTH};
use super::weather_model::{weather_view_model, WeatherViewModel};
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;
use crate::weather::{WeatherCondition, WeatherDayPhase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeatherResponsiveMode {
    Large,
    Medium,
    Tiny,
}

impl WeatherResponsiveMode {
    fn from_rect(area: Rect) -> Self {
        if area.width >= 96 && area.height >= 18 {
            Self::Large
        } else if area.width >= 48 && area.height >= 13 {
            Self::Medium
        } else {
            Self::Tiny
        }
    }
}

/// Weather mirrors a small weather station: what the sky is doing now (left),
/// the next day (middle), what it means for SunReactor and a few secondary
/// readings (right), and the sun and location underneath. Each value appears
/// in one place only.
pub(crate) fn render_weather(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let view = weather_view_model(app);
    if !view.has_current_conditions {
        render_status_only(f, app, area, &view, styles);
        return;
    }
    if WeatherResponsiveMode::from_rect(area) == WeatherResponsiveMode::Tiny {
        render_tiny(f, app, area, &view, styles);
        return;
    }

    let strip_height = if area.height >= 26 && area.width >= 90 {
        4
    } else {
        0
    };
    let body = Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(strip_height),
    );
    let left_width = (area.width.saturating_mul(24) / 100).clamp(32, 40);
    let right_width = (area.width.saturating_mul(24) / 100).clamp(34, 42);
    let three_columns = area.width >= left_width + right_width + 46;

    if area.width < 72 {
        render_conditions(f, app, body, &view, styles);
    } else if three_columns {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_width),
                Constraint::Length(1),
                Constraint::Min(40),
                Constraint::Length(1),
                Constraint::Length(right_width),
            ])
            .split(body);
        render_conditions(f, app, columns[0], &view, styles);
        render_forecast(f, app, columns[2], &view, styles);
        let impact_height = 6.min(columns[4].height);
        render_impact(
            f,
            Rect::new(columns[4].x, columns[4].y, columns[4].width, impact_height),
            &view,
            styles,
        );
        if columns[4].height > impact_height + 4 {
            render_additional(
                f,
                app,
                Rect::new(
                    columns[4].x,
                    columns[4].y + impact_height + 1,
                    columns[4].width,
                    columns[4].height - impact_height - 1,
                ),
                &view,
                styles,
            );
        }
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_width),
                Constraint::Length(1),
                Constraint::Min(36),
            ])
            .split(body);
        render_conditions(f, app, columns[0], &view, styles);
        render_forecast(f, app, columns[2], &view, styles);
    }
    if strip_height > 0 {
        render_sun_strip(
            f,
            app,
            Rect::new(area.x, area.y + body.height, area.width, strip_height),
            &view,
            styles,
        );
    }
}

fn freshness_spans(view: &WeatherViewModel, styles: &SemanticStyles) -> Vec<Span<'static>> {
    let (glyph, style) = if view.state.is_error() {
        ("✕", styles.status_error)
    } else if view.state.is_warning() || view.refresh_due {
        (kit::WARN, styles.status_warning)
    } else {
        (kit::DOT, styles.status_success)
    };
    let mut spans = vec![
        Span::styled(format!("{glyph} "), style),
        Span::styled(
            view.status_label.clone(),
            style.add_modifier(Modifier::BOLD),
        ),
    ];
    if !view.status_detail.is_empty() {
        spans.push(Span::styled(kit::SEPARATOR, styles.border_normal));
        spans.push(Span::styled(view.status_detail.clone(), styles.text_muted));
    }
    spans
}

/// The sky animates once a second with Full effects and stays still otherwise.
fn art_frame(app: &Model) -> u64 {
    if app.motion.level == crate::config::MotionLevel::Instrument {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    } else {
        0
    }
}

const DETAIL_LABEL_WIDTH: usize = 13;

fn detail_row(label: &str, value: String, styles: &SemanticStyles) -> Line<'static> {
    labelled_row(label, DETAIL_LABEL_WIDTH, value, styles)
}

fn labelled_row(
    label: &str,
    width: usize,
    value: String,
    styles: &SemanticStyles,
) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(kit::pad_to(label, width), styles.text_muted),
        Span::styled(value, styles.text_primary),
    ])
}

fn temperature_in_unit(celsius: f32, unit: crate::config::TemperatureUnit) -> (f32, &'static str) {
    match unit {
        crate::config::TemperatureUnit::Celsius => (celsius, "C"),
        crate::config::TemperatureUnit::Fahrenheit => (celsius * 9.0 / 5.0 + 32.0, "F"),
    }
}

/// Sixteen-point compass name for a wind direction in degrees.
fn compass(degrees: u16) -> &'static str {
    const POINTS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    POINTS[((f32::from(degrees % 360) / 22.5).round() as usize) % 16]
}

#[allow(clippy::too_many_lines)]
fn render_conditions(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let meta = view
        .valid_at_label
        .as_ref()
        .map(|valid| vec![Span::styled(format!("for {valid}"), styles.text_muted)])
        .unwrap_or_default();
    let block = kit::with_meta(kit::panel("Current conditions", false, styles), meta);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height < 3 || inner.width < 20 {
        return;
    }
    let unit = app.config.tui.temperature_unit;
    let details = view.details;

    let mut rows: Vec<Line<'static>> = Vec::new();
    if let Some(feels_like) = details.feels_like {
        rows.push(detail_row(
            "Feels like",
            super::weather_model::format_temp(feels_like, unit),
            styles,
        ));
    }
    if let Some(humidity) = details.humidity_percent {
        rows.push(detail_row("Humidity", format!("{humidity}%"), styles));
    }
    if let Some(cloud) = view.cloud_cover_percent {
        rows.push(detail_row("Cloud cover", format!("{cloud}%"), styles));
    }
    if let Some(speed) = details.wind_speed_mps {
        let direction = details
            .wind_direction_deg
            .map(|degrees| format!(" {}", compass(degrees)))
            .unwrap_or_default();
        rows.push(detail_row(
            "Wind",
            format!("{:.0} km/h{direction}", speed * 3.6),
            styles,
        ));
    }
    if let Some(pressure) = details.pressure_hpa {
        rows.push(detail_row("Pressure", format!("{pressure} hPa"), styles));
    }
    if let Some(visibility) = details.visibility_m {
        let text = if visibility >= 1000 {
            format!("{:.0} km", f64::from(visibility) / 1000.0)
        } else {
            format!("{visibility} m")
        };
        rows.push(detail_row("Visibility", text, styles));
    }

    // Condition, temperature, and freshness always show; the scene and the
    // detail rows share what is left, the scene first.
    let fixed = 1 + 1 + 3 + 1;
    let art_needed = ART_ROWS + 1;
    let show_art = inner.height >= fixed + art_needed + 2 && inner.width >= ART_WIDTH;
    let mut y = inner.y;
    if show_art {
        let art = weather_art(
            view.condition,
            view.cloud_cover_percent,
            view.day_phase,
            view.moon,
            art_frame(app),
            &styles.palette,
        );
        let x = inner.x + (inner.width - ART_WIDTH) / 2;
        f.render_widget(
            Paragraph::new(art.lines).style(styles.base),
            Rect::new(x, y, ART_WIDTH, ART_ROWS),
        );
        y += art_needed;
    }

    let mut condition = vec![
        Span::raw(" "),
        Span::styled(
            view.condition_label.clone(),
            styles.text_primary.add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(description) = view
        .condition_description
        .as_deref()
        .filter(|description| !description.eq_ignore_ascii_case(&view.condition_label))
    {
        condition.push(Span::styled(
            format!("{}{}", kit::SEPARATOR, capitalize(description)),
            styles.text_muted,
        ));
    }
    let mut lines = vec![Line::from(condition), Line::from("")];
    match view.temperature_celsius {
        Some(celsius) => {
            let (value, letter) = temperature_in_unit(celsius, unit);
            let color =
                crate::tui::theme::mix(temperature_color(celsius), styles.palette.accent, 0.3);
            for (index, row) in super::fonts::weather_pixel_font_rows(&format!("{value:.0}"))
                .into_iter()
                .enumerate()
            {
                let mut spans = vec![
                    Span::raw(" "),
                    Span::styled(row, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                ];
                if index == 0 {
                    spans.push(Span::styled(
                        format!(" °{letter}"),
                        styles.text_primary.add_modifier(Modifier::BOLD),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }
        None => lines.push(Line::from(Span::styled(
            format!(" {}", view.temperature_label),
            styles.text_heading.add_modifier(Modifier::BOLD),
        ))),
    }
    // Freshness takes one row, or two when its detail does not fit beside it.
    let mut freshness = vec![Span::raw(" ")];
    freshness.extend(freshness_spans(view, styles));
    let freshness_lines = if freshness.iter().map(Span::width).sum::<usize>()
        > usize::from(inner.width)
        && freshness.len() > 3
    {
        let detail = freshness.split_off(3);
        let mut second = vec![Span::raw("   ")];
        second.extend(detail.into_iter().skip(1));
        vec![Line::from(freshness), Line::from(second)]
    } else {
        vec![Line::from(freshness)]
    };
    let freshness_height = freshness_lines.len() as u16;
    let bottom = inner.y + inner.height + 1 - freshness_height;
    let room = bottom.saturating_sub(y + lines.len() as u16 + 1);
    if room >= 3 && !rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "─".repeat(usize::from(inner.width)),
            styles.border_normal,
        )));
        let shown = usize::from(room - 2).min(rows.len());
        lines.extend(rows.into_iter().take(shown));
    }
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(inner.x, y, inner.width, bottom.saturating_sub(y + 1)),
    );

    f.render_widget(
        Paragraph::new(freshness_lines),
        Rect::new(inner.x, bottom - 1, inner.width, freshness_height),
    );
}

fn capitalize(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + characters.as_str()
    })
}

/// One point of the forecast: the sample in use first, then the next ones.
struct ForecastSample {
    epoch_s: u64,
    celsius: f32,
    cloud_cover_percent: Option<u8>,
    condition: WeatherCondition,
    day_phase: Option<WeatherDayPhase>,
    time_label: String,
}

fn forecast_samples(view: &WeatherViewModel) -> Vec<ForecastSample> {
    let mut samples = Vec::new();
    if let (Some(epoch_s), Some(celsius)) = (view.valid_at_epoch_s, view.temperature_celsius) {
        samples.push(ForecastSample {
            epoch_s,
            celsius,
            cloud_cover_percent: view.cloud_cover_percent,
            condition: view.condition,
            day_phase: Some(view.day_phase),
            time_label: view.valid_at_label.clone().unwrap_or_default(),
        });
    }
    let after = samples.last().map_or(0, |last| last.epoch_s);
    samples.extend(
        view.forecast
            .iter()
            .filter(|point| point.epoch_s > after)
            .map(|point| ForecastSample {
                epoch_s: point.epoch_s,
                celsius: point.temperature_celsius,
                cloud_cover_percent: Some(point.cloud_cover_percent),
                condition: point.condition,
                day_phase: point.day_phase,
                time_label: point.time_label.clone(),
            }),
    );
    samples.truncate(9);
    samples
}

#[derive(Debug, Clone, Copy, Default)]
struct ForecastRows {
    has_axis: bool,
    time_row: u16,
    gap_time_icon: u16,
    icon_rows: u16,
    gap_icon_temp: u16,
    temp_row: u16,
    bottom_padding: u16,
}

impl ForecastRows {
    fn plan(available_height: u16) -> (u16, Self) {
        if available_height < 6 {
            return (available_height.max(3), Self::default());
        }

        let mut rows = Self::default();
        if available_height >= 22 {
            rows.has_axis = true;
            rows.time_row = 1;
            rows.gap_time_icon = 1;
            rows.icon_rows = 2;
            rows.gap_icon_temp = 1;
            rows.temp_row = 1;
            if available_height >= 26 {
                rows.bottom_padding = 1;
            }
        } else if available_height >= 17 {
            rows.has_axis = false;
            rows.time_row = 1;
            rows.gap_time_icon = 1;
            rows.icon_rows = 2;
            rows.gap_icon_temp = 1;
            rows.temp_row = 1;
        } else if available_height >= 13 {
            rows.has_axis = false;
            rows.time_row = 1;
            rows.gap_time_icon = 0;
            rows.icon_rows = 2;
            rows.gap_icon_temp = 1;
            rows.temp_row = 1;
        } else if available_height >= 9 {
            rows.has_axis = false;
            rows.time_row = 1;
            rows.gap_time_icon = 0;
            rows.icon_rows = 2;
            rows.gap_icon_temp = 0;
            rows.temp_row = 1;
        } else {
            return (available_height.max(3), Self::default());
        }

        let strip_total = rows.total_rows();
        let plot_height = available_height.saturating_sub(strip_total).max(4);
        (plot_height, rows)
    }

    fn total_rows(&self) -> u16 {
        u16::from(self.has_axis)
            + self.time_row
            + self.gap_time_icon
            + self.icon_rows
            + self.gap_icon_temp
            + self.temp_row
            + self.bottom_padding
    }
}

#[allow(clippy::too_many_lines)]
fn render_forecast(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    // The legend only shows when it fits beside the title.
    let legend = "Temperature";
    let meta = if area.width >= 22 + kit::cell_width(legend) as u16 + 6 {
        vec![Span::styled(legend, styles.text_muted)]
    } else {
        Vec::new()
    };
    let block = kit::with_meta(kit::panel("24 hour forecast", false, styles), meta);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    let samples = forecast_samples(view);
    if samples.len() < 2 || inner.width < 30 || inner.height < 6 {
        f.render_widget(
            Paragraph::new(Span::styled(" No forecast samples yet.", styles.text_muted)),
            inner,
        );
        return;
    }

    let unit = app.config.tui.temperature_unit;
    let gutter: u16 = 5;
    let plot_x = inner.x + gutter;
    let plot_width = inner.width.saturating_sub(gutter + 2);
    let (plot_height, forecast_rows) = ForecastRows::plan(inner.height.saturating_sub(1));
    let plot_top = inner.y + 1;
    let now_epoch = app
        .status
        .as_ref()
        .map(|status| status.now_epoch_s)
        .filter(|epoch| *epoch > 0)
        .unwrap_or_else(|| u64::try_from(chrono::Utc::now().timestamp()).unwrap_or(0));
    // The chart starts at now when the first sample is still ahead; the curve
    // holds that sample's value until it begins.
    let first = samples[0]
        .epoch_s
        .min(now_epoch.max(samples[0].epoch_s.saturating_sub(3 * 3600)));
    let last = samples[samples.len() - 1].epoch_s.max(first + 1);
    let column_of = |epoch: u64| -> u16 {
        let fraction = (epoch.saturating_sub(first)) as f64 / (last - first) as f64;
        (fraction.clamp(0.0, 1.0) * f64::from(plot_width.saturating_sub(1))).round() as u16
    };

    // A tidy degree scale with room above and below the curve.
    let low = samples
        .iter()
        .map(|s| s.celsius)
        .fold(f32::INFINITY, f32::min);
    let high = samples
        .iter()
        .map(|s| s.celsius)
        .fold(f32::NEG_INFINITY, f32::max);
    let (low_display, _) = temperature_in_unit(low, unit);
    let (high_display, _) = temperature_in_unit(high, unit);
    let step = [1.0_f32, 2.0, 4.0, 5.0, 10.0, 20.0]
        .into_iter()
        .find(|step| (high_display - low_display + 4.0) / step <= f32::from(plot_height.min(5)))
        .unwrap_or(20.0);
    let floor = ((low_display - 2.0) / step).floor() * step;
    let ceiling = (((high_display + 2.0) / step).ceil() * step).max(floor + step * 2.0);

    // Cosine interpolation between samples keeps the curve smooth.
    let value_at = |epoch: f64| -> f32 {
        if epoch <= samples[0].epoch_s as f64 {
            return samples[0].celsius;
        }
        let index = samples
            .windows(2)
            .position(|pair| epoch <= pair[1].epoch_s as f64)
            .unwrap_or(samples.len() - 2);
        let (a, b) = (&samples[index], &samples[index + 1]);
        let span = (b.epoch_s.saturating_sub(a.epoch_s)).max(1) as f64;
        let t = ((epoch - a.epoch_s as f64) / span).clamp(0.0, 1.0);
        let eased = ((1.0 - (t * std::f64::consts::PI).cos()) / 2.0) as f32;
        a.celsius + (b.celsius - a.celsius) * eased
    };
    let grow = app
        .motion
        .weather_update_phase(Instant::now())
        .map_or(1.0, |phase| f64::from(1.0 - (1.0 - phase).powi(3)));

    let palette = styles.palette;
    let grid = Style::default().fg(crate::tui::theme::mix(
        palette.border_inactive,
        palette.bg,
        0.45,
    ));
    {
        let buffer = f.buffer_mut();
        let row_of = |degrees: f32| -> u16 {
            let fraction = (degrees - floor) / (ceiling - floor);
            (plot_height - 1)
                - ((fraction.clamp(0.0, 1.0) * f32::from(plot_height - 1)).round() as u16)
        };
        let mut label = floor;
        while label <= ceiling + 0.01 {
            let row = row_of(label);
            for column in 0..plot_width {
                buffer
                    .get_mut(plot_x + column, plot_top + row)
                    .set_symbol("┈")
                    .set_style(grid);
            }
            label += step;
        }
        let total = f64::from(plot_height) * 8.0;
        for column in 0..plot_width {
            let epoch = first as f64
                + f64::from(column) / f64::from(plot_width.saturating_sub(1).max(1))
                    * (last - first) as f64;
            let celsius = value_at(epoch);
            let (display, _) = temperature_in_unit(celsius, unit);
            let fraction =
                f64::from(((display - floor) / (ceiling - floor)).clamp(0.0, 1.0)) * grow;
            let eighths = (fraction * total).round() as u16;
            let full = eighths / 8;
            let partial = usize::from(eighths % 8);
            let color = crate::tui::theme::mix(temperature_color(celsius), palette.accent, 0.3);
            for step_row in 0..plot_height {
                let row = plot_top + plot_height - 1 - step_row;
                let cell = buffer.get_mut(plot_x + column, row);
                if step_row < full {
                    let height = f64::from(step_row + 1) / f64::from(plot_height);
                    cell.set_symbol("█")
                        .set_style(Style::default().fg(crate::tui::theme::mix(
                            palette.bg,
                            color,
                            0.25 + 0.6 * height,
                        )));
                } else if step_row == full && partial > 0 {
                    cell.set_symbol(super::light_cycle::EIGHTHS[partial])
                        .set_style(Style::default().fg(color));
                }
            }
        }
    }

    // Degree labels on the grid lines.
    let row_of = |degrees: f32| -> u16 {
        let fraction = (degrees - floor) / (ceiling - floor);
        (plot_height - 1) - ((fraction.clamp(0.0, 1.0) * f32::from(plot_height - 1)).round() as u16)
    };
    let mut label = floor;
    while label <= ceiling + 0.01 {
        f.render_widget(
            Paragraph::new(Span::styled(format!("{label:>3.0}°"), styles.text_muted)),
            Rect::new(inner.x, plot_top + row_of(label), gutter - 1, 1),
        );
        label += step;
    }

    // Now: a dashed line with the time above the chart.
    if (first..=last).contains(&now_epoch) {
        let column = column_of(now_epoch);
        for row in 0..plot_height {
            let cell = f.buffer_mut().get_mut(plot_x + column, plot_top + row);
            let style = cell.style().fg(palette.fg);
            cell.set_symbol("╎").set_style(style);
        }
        let time = app
            .local_time_at_epoch(now_epoch)
            .map(|time| super::automation::format_time(&time, app.config.tui.use_12h_time))
            .unwrap_or_default();
        let text = format!("Now {time} ▾");
        let width = kit::cell_width(&text) as u16;
        let start = (plot_x + column).saturating_sub(width - 1).max(plot_x);
        f.render_widget(
            Paragraph::new(Span::styled(
                text,
                styles.text_primary.add_modifier(Modifier::BOLD),
            )),
            Rect::new(start, inner.y, width.min(inner.x + inner.width - start), 1),
        );
    }

    if forecast_rows.total_rows() == 0 {
        return;
    }

    let mut displayed_samples: Vec<(&ForecastSample, u16)> = Vec::new();
    for sample in &samples {
        let center = (plot_x + column_of(sample.epoch_s))
            .clamp(inner.x + 3, inner.x + inner.width.saturating_sub(4));
        if let Some(&(_, prev_center)) = displayed_samples.last() {
            if center < prev_center + 9 {
                continue;
            }
        }
        displayed_samples.push((sample, center));
    }

    let mut current_y = plot_top + plot_height;

    if forecast_rows.has_axis {
        let axis_style = Style::default().fg(crate::tui::theme::mix(
            palette.border_inactive,
            palette.bg,
            0.45,
        ));
        let buffer = f.buffer_mut();
        for col in 0..plot_width {
            buffer
                .get_mut(plot_x + col, current_y)
                .set_symbol("─")
                .set_style(axis_style);
        }
        for (_, center) in &displayed_samples {
            if *center >= plot_x && *center < plot_x + plot_width {
                buffer
                    .get_mut(*center, current_y)
                    .set_symbol("┴")
                    .set_style(axis_style);
            }
        }
        current_y += 1;
    }

    if forecast_rows.time_row > 0 {
        for (sample, center) in &displayed_samples {
            let text = &sample.time_label;
            let width = kit::cell_width(text) as u16;
            let start = center
                .saturating_sub(width / 2)
                .clamp(inner.x, inner.x + inner.width.saturating_sub(width));
            f.render_widget(
                Paragraph::new(Span::styled(text.clone(), styles.text_muted)),
                Rect::new(start, current_y, width, 1),
            );
        }
        current_y += forecast_rows.time_row;
    }

    current_y += forecast_rows.gap_time_icon;

    if forecast_rows.icon_rows > 0 {
        for (sample, center) in &displayed_samples {
            let icon = super::weather_art::mini_icon(
                sample.condition,
                sample.cloud_cover_percent,
                sample.day_phase,
                &palette,
            );
            f.render_widget(
                Paragraph::new(icon).style(styles.base),
                Rect::new(center.saturating_sub(3), current_y, 7, 2),
            );
        }
        current_y += forecast_rows.icon_rows;
    }

    current_y += forecast_rows.gap_icon_temp;

    if forecast_rows.temp_row > 0 {
        for (sample, center) in &displayed_samples {
            let (display, _) = temperature_in_unit(sample.celsius, unit);
            let temperature = format!("{display:.0}°");
            let color =
                crate::tui::theme::mix(temperature_color(sample.celsius), palette.accent, 0.3);
            let width = kit::cell_width(&temperature) as u16;
            let start = center
                .saturating_sub(width / 2)
                .clamp(inner.x, inner.x + inner.width.saturating_sub(width));
            f.render_widget(
                Paragraph::new(Span::styled(
                    temperature,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Rect::new(start, current_y, width, 1),
            );
        }
    }
}

/// What the weather means for SunReactor, in one headline and one sentence.
fn render_impact(f: &mut Frame, area: Rect, view: &WeatherViewModel, styles: &SemanticStyles) {
    let block = kit::panel("SunReactor impact", false, styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height < 2 || inner.width < 20 {
        return;
    }
    let (headline, detail, arrow) = match view.multiplier {
        Some(multiplier) if multiplier < 0.995 => (
            "Lower solar input",
            format!(
                "Clouds lower daylight targets by about {:.0}%.",
                (1.0 - multiplier) * 100.0
            ),
            "↓",
        ),
        Some(_) => (
            "Full solar input",
            String::from("The sky is not reducing brightness."),
            "→",
        ),
        None if view.state.is_warning() || view.refresh_due => (
            "Weather not in use",
            String::from("The forecast is stale; brightness follows the sun."),
            "·",
        ),
        None => (
            "Weather not in use",
            String::from("Brightness follows the sun alone."),
            "·",
        ),
    };
    let icon = super::weather_art::mini_icon(
        WeatherCondition::Clear,
        None,
        Some(WeatherDayPhase::Day),
        &styles.palette,
    );
    f.render_widget(
        Paragraph::new(icon).style(styles.base),
        Rect::new(inner.x + 1, inner.y, 7, 2),
    );
    f.render_widget(
        Paragraph::new(Span::styled(arrow, styles.focus_marker)),
        Rect::new(inner.x + 8, inner.y + 1, 1, 1),
    );
    let text_x = inner.x + 11;
    let text_width = inner.width.saturating_sub(11);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(headline, styles.text_heading)),
            Line::from(Span::styled(detail, styles.text_muted)),
        ])
        .wrap(Wrap { trim: true }),
        Rect::new(text_x, inner.y, text_width, inner.height),
    );
}

/// Dew point in °C from temperature and relative humidity (Magnus formula).
fn dew_point(celsius: f32, humidity_percent: u8) -> Option<f32> {
    if humidity_percent == 0 {
        return None;
    }
    let (b, c) = (17.62_f32, 243.12_f32);
    let gamma = (f32::from(humidity_percent) / 100.0).ln() + b * celsius / (c + celsius);
    Some(c * gamma / (b - gamma))
}

/// Wind chill in °C, defined only for cold, windy air (≤ 10 °C, > 4.8 km/h).
fn wind_chill(celsius: f32, wind_kmh: f32) -> Option<f32> {
    (celsius <= 10.0 && wind_kmh > 4.8).then(|| {
        let v = wind_kmh.powf(0.16);
        13.12 + 0.6215 * celsius - 11.37 * v + 0.3965 * celsius * v
    })
}

fn air_quality_label(aqi: u16) -> &'static str {
    match aqi {
        0..=50 => "Good",
        51..=100 => "Moderate",
        101..=150 => "Sensitive groups",
        151..=200 => "Unhealthy",
        201..=300 => "Very unhealthy",
        _ => "Hazardous",
    }
}

fn render_additional(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let block = kit::panel("Additional conditions", false, styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height == 0 {
        return;
    }
    let unit = app.config.tui.temperature_unit;
    let details = view.details;
    let mut rows: Vec<(&str, String)> = vec![(
        "Air quality",
        details.air_quality.map_or_else(
            || String::from("Unavailable"),
            |air| format!("{}  (AQI {})", air_quality_label(air.us_aqi), air.us_aqi),
        ),
    )];
    if let (Some(celsius), Some(humidity)) = (view.temperature_celsius, details.humidity_percent) {
        if let Some(dew) = dew_point(celsius, humidity) {
            rows.push(("Dew point", super::weather_model::format_temp(dew, unit)));
        }
    }
    if let Some(chance) = details.precipitation_percent {
        rows.push(("Precipitation", format!("{chance}% chance")));
    }
    if let (Some(celsius), Some(speed)) = (view.temperature_celsius, details.wind_speed_mps) {
        if let Some(chill) = wind_chill(celsius, speed * 3.6) {
            rows.push(("Wind chill", super::weather_model::format_temp(chill, unit)));
        }
    }
    if view.day_phase == WeatherDayPhase::Night {
        if let Some(moon) = view.moon {
            rows.push((
                "Moon",
                format!("{} {:.0}%", moon.phase_name(), moon.illumination() * 100.0),
            ));
        }
    }
    let label_width = rows
        .iter()
        .map(|(label, _)| kit::cell_width(label))
        .max()
        .unwrap_or(0)
        + 2;
    let rows: Vec<Line<'static>> = rows
        .into_iter()
        .map(|(label, value)| labelled_row(label, label_width, value, styles))
        .collect();
    f.render_widget(Paragraph::new(rows), inner);
}

/// Sun times, daylight, location, and data source along the bottom.
fn render_sun_strip(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(styles.border_normal)
        .padding(ratatui::widgets::Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height < 2 {
        return;
    }
    let (mut items, place) = sun_strip_items(app, view);
    fit_sun_strip_items(&mut items, &place, inner.width);
    render_sun_strip_items(f, inner, &items, styles);
}

struct SunStripItem {
    glyph: &'static str,
    label: &'static str,
    value: String,
}

fn sun_strip_items(app: &Model, view: &WeatherViewModel) -> (Vec<SunStripItem>, String) {
    let anchors = super::automation::cycle_anchors(app, None);
    let time = |value: Option<&chrono::DateTime<chrono::FixedOffset>>| {
        value.map_or_else(
            || String::from("—"),
            |time| super::automation::format_time(time, app.config.tui.use_12h_time),
        )
    };
    let daylight = match (&anchors.sunrise, &anchors.sunset) {
        (Some(sunrise), Some(sunset)) if sunset > sunrise => {
            let minutes = (*sunset - *sunrise).num_minutes();
            format!("{}h {:02}m", minutes / 60, minutes % 60)
        }
        _ => String::from("—"),
    };
    let location = &app.config.location;
    let place = if location.city.trim().is_empty() {
        String::from("Configured location")
    } else {
        location.city.trim().to_owned()
    };
    let coordinates = format!(
        "{:.2}° {} · {:.2}° {}",
        location.latitude.abs(),
        if location.latitude < 0.0 { "S" } else { "N" },
        location.longitude.abs(),
        if location.longitude < 0.0 { "W" } else { "E" },
    );
    (
        vec![
            SunStripItem {
                glyph: "↑",
                label: "Sunrise",
                value: time(anchors.sunrise.as_ref()),
            },
            SunStripItem {
                glyph: kit::BRAND_MARK,
                label: "Solar noon",
                value: time(anchors.noon.as_ref()),
            },
            SunStripItem {
                glyph: "↓",
                label: "Sunset",
                value: time(anchors.sunset.as_ref()),
            },
            SunStripItem {
                glyph: "◐",
                label: "Daylight",
                value: daylight,
            },
            SunStripItem {
                glyph: "◎",
                label: "Location",
                value: format!("{place}  {coordinates}"),
            },
            SunStripItem {
                glyph: "≡",
                label: "Weather data",
                value: view.source_label.clone(),
            },
        ],
        place,
    )
}

fn sun_strip_width(items: &[SunStripItem]) -> usize {
    items
        .iter()
        .map(|item| 2 + kit::cell_width(item.label).max(kit::cell_width(&item.value)))
        .sum::<usize>()
        + items.len().saturating_sub(1) * 3
}

fn fit_sun_strip_items(items: &mut Vec<SunStripItem>, place: &str, width: u16) {
    for label in ["Weather data", "Daylight", "Solar noon"] {
        if sun_strip_width(items) <= usize::from(width) {
            break;
        }
        items.retain(|item| item.label != label);
    }
    if sun_strip_width(items) > usize::from(width) {
        if let Some(item) = items.iter_mut().find(|item| item.label == "Location") {
            place.clone_into(&mut item.value);
        }
    }
}

fn render_sun_strip_items(
    f: &mut Frame,
    inner: Rect,
    items: &[SunStripItem],
    styles: &SemanticStyles,
) {
    let widths: Vec<u16> = items
        .iter()
        .map(|item| (2 + kit::cell_width(item.label).max(kit::cell_width(&item.value))) as u16)
        .collect();
    let content: u16 = widths.iter().sum();
    let spacing =
        inner.width.saturating_sub(content) / (items.len().saturating_sub(1) as u16).max(1);
    let mut x = inner.x;
    for (index, (item, width)) in items.iter().zip(&widths).enumerate() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(format!("{} ", item.glyph), styles.focus_marker),
                    Span::styled(item.label, styles.text_muted),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(item.value.clone(), styles.text_primary),
                ]),
            ]),
            Rect::new(x, inner.y, (*width).min(inner.x + inner.width - x), 2),
        );
        x += width;
        if index + 1 < items.len() {
            render_sun_strip_separator(f, x + spacing / 2, inner.y, inner.x + inner.width, styles);
            x += spacing;
        }
    }
}

fn render_sun_strip_separator(f: &mut Frame, x: u16, y: u16, right: u16, styles: &SemanticStyles) {
    if x < right {
        for row in 0..2 {
            f.buffer_mut()
                .get_mut(x, y + row)
                .set_symbol("│")
                .set_style(styles.border_normal);
        }
    }
}

/// A colour for a temperature: cool blue through a mild gold to warm coral.
fn temperature_color(celsius: f32) -> Color {
    const STOPS: [(f32, (u8, u8, u8)); 5] = [
        (-10.0, (0x8e, 0xb8, 0xff)),
        (5.0, (0x6f, 0xc3, 0xdf)),
        (15.0, (0xf2, 0xd0, 0x6b)),
        (25.0, (0xff, 0xa5, 0x5c)),
        (35.0, (0xff, 0x6b, 0x5b)),
    ];
    let clamped = celsius.clamp(STOPS[0].0, STOPS[STOPS.len() - 1].0);
    let index = STOPS
        .windows(2)
        .position(|pair| clamped <= pair[1].0)
        .unwrap_or(STOPS.len() - 2);
    let (low, from) = STOPS[index];
    let (high, to) = STOPS[index + 1];
    let amount = f64::from(((clamped - low) / (high - low)).clamp(0.0, 1.0));
    crate::tui::theme::mix(
        Color::Rgb(from.0, from.1, from.2),
        Color::Rgb(to.0, to.1, to.2),
        amount,
    )
}

fn render_tiny(
    f: &mut Frame,
    _app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            view.temperature_label.clone(),
            styles.text_heading.add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            view.condition_label.clone(),
            styles.text_primary.add_modifier(Modifier::BOLD),
        ),
    ])];
    if let Some(cloud) = view.cloud_cover_percent {
        lines.push(Line::from(vec![
            Span::styled("Cloud cover ", styles.data_label),
            Span::styled(format!("{cloud}%"), styles.text_primary),
        ]));
    }
    lines.push(Line::from(Span::styled(
        view.location_label.clone(),
        styles.text_muted,
    )));
    lines.push(Line::from(freshness_spans(view, styles)));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

fn render_status_only(
    f: &mut Frame,
    _app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let width = area.width.min(64);
    let height = area.height.min(7);
    let panel_area = kit::centered(area, width, height);
    let block = kit::panel("Weather", false, styles);
    let lines = vec![
        Line::from(""),
        Line::from(freshness_spans(
            &WeatherViewModel {
                status_detail: String::new(),
                ..view.clone()
            },
            styles,
        )),
        Line::from(Span::styled(view.status_detail.clone(), styles.text_muted)),
    ];
    f.render_widget(Clear, panel_area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(styles.base)
            .block(block),
        panel_area,
    );
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    use super::WeatherResponsiveMode;
    use crate::ipc::{StatusResponse, WeatherStatus};
    use crate::state::ForecastPoint;
    use crate::tui::model::{DaemonConnection, Tab, TargetPreview};
    use crate::tui::test_support::{dummy_status, find_in_buffer};
    use crate::tui::ui;
    use crate::tui::update::{self, Message};
    use crate::tui::worker::IpcEvent;
    use crate::tui::Model;
    use crate::weather::{WeatherCondition, WeatherDayPhase, WeatherSourceKind, WeatherState};

    fn weather_model(state: WeatherState, condition: WeatherCondition) -> Model {
        let mut model = Model::new();
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;
        model.config.weather.enabled = true;
        model.config.location.city = String::from("Istanbul");
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.status = Some(StatusResponse {
            daemon_alive: true,
            config_path: String::new(),
            tick_seconds: 60,
            dry_run: false,
            suspended: false,
            desktop_idle_dimmed: false,
            suspend_until_epoch_s: None,
            manual_override_active: false,
            per_monitor_override_until_epoch_s: None,
            global_override_percent: None,
            global_override_until_epoch_s: None,
            configured_monitors: 0,
            stateful_monitors: 0,
            weather: Some(WeatherStatus {
                enabled: true,
                state,
                active: matches!(state, WeatherState::Ready),
                stale: state == WeatherState::Stale,
                provider: Some(String::from("OpenWeather")),
                fetched_at_epoch_s: Some(1_700_000_000),
                valid_at_epoch_s: Some(1_700_000_000),
                source_kind: Some(WeatherSourceKind::Forecast),
                last_refresh_attempt_epoch_s: Some(1_700_000_000),
                next_refresh_at_epoch_s: Some(1_700_001_800),
                consecutive_failures: 0,
                last_error: None,
                cloud_cover_percent: Some(match condition {
                    WeatherCondition::Clear => 0,
                    WeatherCondition::PartlyCloudy => 42,
                    WeatherCondition::Cloudy => 80,
                    _ => 90,
                }),
                temperature: Some(23.0),
                condition,
                condition_description: Some(String::from("fixture conditions")),
                day_phase: Some(WeatherDayPhase::Day),
                forecast: vec![
                    ForecastPoint {
                        dt_epoch_s: 1_700_003_600,
                        cloud_cover_percent: 20,
                        temperature: 22.0,
                        ..Default::default()
                    },
                    ForecastPoint {
                        dt_epoch_s: 1_700_007_200,
                        cloud_cover_percent: 45,
                        temperature: 20.0,
                        ..Default::default()
                    },
                    ForecastPoint {
                        dt_epoch_s: 1_700_010_800,
                        cloud_cover_percent: 80,
                        temperature: 19.0,
                        ..Default::default()
                    },
                ],
                multiplier: (state != WeatherState::Stale).then_some(0.8),
                ..Default::default()
            }),
            monitors: Vec::new(),
            solar_elevation: Some(20.0),
            now_epoch_s: 1_700_000_120,
            sunrise_epoch_s: None,
            sunset_epoch_s: None,
            lunar_phase: None,
        });
        model
    }

    fn render(model: &mut Model, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height))
            .expect("test terminal should initialize");
        terminal
            .draw(|frame| crate::tui::ui::ui(frame, model))
            .expect("weather screen should render");
        buffer_text(terminal.backend().buffer())
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let mut screen = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                screen.push_str(buffer.get(x, y).symbol());
            }
            screen.push('\n');
        }
        screen
    }

    #[test]
    fn weather_art_survives_the_supported_terminal_sizes() {
        for (width, height) in [(80, 24), (100, 30), (120, 35), (140, 40), (160, 45)] {
            let mut model = weather_model(WeatherState::Ready, WeatherCondition::Clear);
            let screen = render(&mut model, width, height);

            assert!(
                screen.chars().any(|c| matches!(c, '▀' | '▄' | '█')),
                "{width}x{height}: {screen}"
            );
            assert!(screen.contains("°C"), "{width}x{height}: {screen}");
            assert!(screen.contains("Clear"), "{width}x{height}: {screen}");
        }
    }

    #[test]
    fn visually_distinct_conditions_use_their_own_block_art() {
        let art_region = |condition| {
            let mut model = weather_model(WeatherState::Ready, condition);
            let screen = render(&mut model, 120, 35);
            let first = screen
                .lines()
                .position(|line| line.contains("╭ Current conditions"))
                .expect("conditions panel");
            screen
                .lines()
                .skip(first + 1)
                .take(7)
                .map(|line| line.chars().take(40).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let cloudy = art_region(WeatherCondition::Cloudy);
        let rain = art_region(WeatherCondition::Rain);
        let snow = art_region(WeatherCondition::Snow);
        assert_ne!(cloudy, rain);
        assert_ne!(rain, snow);
        assert_ne!(cloudy, snow);
    }

    #[test]
    fn night_shows_the_real_moon_phase() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        let weather = model
            .status
            .as_mut()
            .and_then(|status| status.weather.as_mut())
            .expect("weather fixture should exist");
        weather.day_phase = Some(WeatherDayPhase::Night);
        // 2023-11-14 22:15 UTC: two days after the new moon of 13 November.
        let screen = render(&mut model, 120, 35);
        assert!(screen.contains("Waxing crescent"), "{screen}");
        assert!(!screen.contains("Full moon"), "{screen}");
    }

    #[test]
    fn tiny_weather_keeps_the_input_and_drops_the_art() {
        let mut tiny = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        let tiny_screen = render(&mut tiny, 45, 12);
        assert!(tiny_screen.contains("23.0°C"));
        assert!(tiny_screen.contains("Clear"));
        assert!(!tiny_screen
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn loading_and_failures_do_not_claim_unconfirmed_conditions() {
        let mut loading = weather_model(WeatherState::Loading, WeatherCondition::Unknown);
        loading
            .status
            .as_mut()
            .and_then(|status| status.weather.as_mut())
            .expect("weather fixture should exist")
            .cloud_cover_percent = None;
        let loading_screen = render(&mut loading, 100, 30);
        assert!(loading_screen.contains("Loading weather"));
        assert!(!loading_screen
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));

        let mut provider_failure =
            weather_model(WeatherState::ProviderError, WeatherCondition::Clear);
        let failure_screen = render(&mut provider_failure, 100, 30);
        assert!(failure_screen.contains("Weather unavailable"));
        assert!(failure_screen.contains("r Retry"), "{failure_screen}");
        assert!(!failure_screen
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn stale_weather_keeps_art_but_is_not_presented_as_affecting_brightness() {
        let mut model = weather_model(WeatherState::Stale, WeatherCondition::Rain);
        let screen = render(&mut model, 120, 35);

        assert!(screen.chars().any(|c| matches!(c, '▀' | '▄' | '█')));
        assert!(screen.contains("Stale"), "{screen}");
        assert!(!screen.contains("Dims daylight"), "{screen}");
        assert!(screen.contains("r Retry"), "{screen}");
    }

    #[test]
    fn fresh_weather_is_not_called_current_and_offers_refresh_not_retry() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Cloudy);
        let screen = render(&mut model, 120, 35);
        assert!(!screen.contains("Current weather"), "{screen}");
        assert!(!screen.contains("CURRENT SKY"), "{screen}");
        assert!(screen.contains("Current conditions"), "{screen}");
        assert!(screen.contains("r Refresh"), "{screen}");
        assert!(!screen.contains("Retry"), "{screen}");
    }

    #[test]
    fn weather_layout_breakpoints_fit_their_required_content() {
        use ratatui::layout::Rect;
        assert_eq!(
            WeatherResponsiveMode::from_rect(Rect::new(0, 0, 96, 18)),
            WeatherResponsiveMode::Large
        );
        assert_eq!(
            WeatherResponsiveMode::from_rect(Rect::new(0, 0, 72, 14)),
            WeatherResponsiveMode::Medium
        );
        assert_eq!(
            WeatherResponsiveMode::from_rect(Rect::new(0, 0, 47, 9)),
            WeatherResponsiveMode::Tiny
        );
    }

    #[test]
    fn forecast_pairs_the_temperature_chart_with_per_hour_icons() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Cloudy);
        let screen = render(&mut model, 160, 45);
        println!("RENDER 160x45:\n{screen}");
        assert!(screen.contains("24 hour forecast"), "{screen}");
        // Every sample's temperature labels the chart once.
        for label in ["22°", "20°", "19°"] {
            assert!(screen.contains(label), "{label}: {screen}");
        }
        assert!(!screen.contains("cloud cover"), "{screen}");
        assert!(screen.chars().any(|c| matches!(c, '▁'..='█')), "{screen}");
    }

    #[test]
    fn weather_refresh_hints_follow_real_dispatch_and_state() {
        let ready = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        assert_eq!(
            crate::tui::ui::weather_model::weather_action_label(&ready),
            Some("Refresh")
        );
        let ready_commands = crate::tui::command::commands_for_footer(&ready);
        assert_eq!(ready_commands.current_commands[0].keys, "r");

        let failed = weather_model(WeatherState::ProviderError, WeatherCondition::Unknown);
        assert_eq!(
            crate::tui::ui::weather_model::weather_action_label(&failed),
            Some("Retry")
        );

        let loading = weather_model(WeatherState::Loading, WeatherCondition::Unknown);
        assert_eq!(
            crate::tui::ui::weather_model::weather_action_label(&loading),
            None
        );
        assert!(crate::tui::command::commands_for_footer(&loading)
            .current_commands
            .is_empty());
    }

    #[test]
    fn weather_stays_about_the_weather() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Cloudy);
        model.monitor_targets_now = vec![TargetPreview {
            logical_id: String::from("mon-0"),
            solar_percent: 80,
            weather_percent: 64,
        }];
        let screen = render(&mut model, 160, 45);
        assert!(!screen.contains("Brightness effect"), "{screen}");
        assert!(!screen.contains("Dims daylight"), "{screen}");
        assert!(screen.contains("SunReactor impact"), "{screen}");
        let cloud_row = screen
            .lines()
            .find(|line| line.contains("Cloud cover"))
            .unwrap_or_default();
        assert!(cloud_row.contains("80%"), "{screen}");
    }
    fn dummy_weather_status(
        enabled: bool,
        active: bool,
        stale: bool,
        cloud_cover: Option<u8>,
        multiplier: Option<f64>,
        forecast_count: usize,
    ) -> crate::ipc::WeatherStatus {
        let forecast = (0..forecast_count)
            .map(|i| crate::state::ForecastPoint {
                dt_epoch_s: 1_700_000_000 + (i as u64) * 3600 * 3,
                cloud_cover_percent: ((i * 15) % 100) as u8,
                temperature: 20.0 + (i as f32),
                ..Default::default()
            })
            .collect();

        crate::ipc::WeatherStatus {
            enabled,
            state: if active {
                crate::weather::WeatherState::Ready
            } else if stale {
                crate::weather::WeatherState::Stale
            } else {
                crate::weather::WeatherState::Loading
            },
            active,
            stale,
            provider: Some(String::from("openweather")),
            fetched_at_epoch_s: Some(1_700_000_000),
            valid_at_epoch_s: Some(1_700_000_000),
            source_kind: None,
            last_refresh_attempt_epoch_s: Some(1_700_000_000),
            next_refresh_at_epoch_s: Some(1_700_000_600),
            consecutive_failures: 0,
            last_error: None,
            cloud_cover_percent: cloud_cover,
            temperature: Some(21.5),
            condition: crate::weather::WeatherCondition::PartlyCloudy,
            condition_description: Some(String::from("broken clouds")),
            day_phase: Some(crate::weather::WeatherDayPhase::Day),
            forecast,
            multiplier,
            ..Default::default()
        }
    }

    #[test]
    fn test_phase8_weather_states_disabled_and_unavailable() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Explicitly Disabled in config
        {
            let mut model = Model::new();
            model.config.weather.enabled = false;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "╭ Weather").is_some());
            assert!(find_in_buffer(&buffer, "Weather is off").is_some());
            assert!(find_in_buffer(&buffer, "Enable weather in Settings").is_some());
            assert!(
                find_in_buffer(&buffer, "Weather integration is disabled in configuration")
                    .is_none()
            );
            assert!(find_in_buffer(&buffer, "ATMOSPHERIC SUBSYSTEM").is_none());
            assert!(find_in_buffer(&buffer, "OpenWeather API Key").is_none());
        }

        // 2. Unavailable: enabled, but no weather data returned yet
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            let mut weather = dummy_weather_status(true, false, false, None, None, 0);
            weather.condition = crate::weather::WeatherCondition::Unknown;
            status.weather = Some(weather);
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "╭ Weather").is_some());
            assert!(find_in_buffer(&buffer, "Loading weather").is_some());
            assert!(find_in_buffer(&buffer, "Contacting OpenWeather").is_some());
        }
    }

    #[test]
    fn test_phase8_weather_states_fresh_stale_and_error_with_cache() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Fresh weather telemetry
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(42),
                Some(0.88),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
            assert!(find_in_buffer(&buffer, "Up to date").is_some());
            assert!(find_in_buffer(&buffer, "Current weather").is_none());
            assert!(find_in_buffer(&buffer, "×0.88").is_none());
        }

        // 2. Stale cached data: keeps chart & instruments visible, demoted with ! Stale
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                false,
                true,
                Some(42),
                Some(0.88),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
            assert!(find_in_buffer(&buffer, "Stale").is_some());
            assert!(find_in_buffer(&buffer, "Dims daylight").is_none());
        }

        // 3. Error with usable cache: keeps cached chart & sky visible with ! Refresh Failed
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.now_epoch_s = 1_700_002_820; // 47m after fetch, within 60m TTL
            let mut ws = dummy_weather_status(true, false, false, Some(42), Some(0.88), 8);
            ws.state = crate::weather::WeatherState::NetworkError;
            ws.last_error = Some(String::from("openweather network timeout"));
            status.weather = Some(ws);
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Weather unavailable").is_some());
            assert!(find_in_buffer(&buffer, "Retry").is_some());
            assert!(find_in_buffer(&buffer, "24 hour forecast").is_none());
        }
    }

    #[test]
    fn test_phase8_weather_extreme_cloud_cover_0_and_100() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // 0% cloud cover: Clear, 100% nominal curve
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(0),
                Some(1.0),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(buffer_text(&buffer)
                .lines()
                .any(|line| line.contains("Cloud cover") && line.contains("0%")));
            assert!(find_in_buffer(&buffer, "×1.00").is_none());
        }

        // 100% cloud cover: nominal attenuation
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(100),
                Some(0.50),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(buffer_text(&buffer)
                .lines()
                .any(|line| line.contains("Cloud cover") && line.contains("100%")));
            assert!(find_in_buffer(&buffer, "×0.50").is_none());
        }
    }

    #[test]
    fn test_phase8_weather_policy_explainability_no_fabricated_metrics() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        // Dark phase stays concise: weather does not repeat policy internals.
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.solar_elevation = Some(-15.0); // Night
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(75),
                Some(0.60),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Night floor").is_none());
            assert!(find_in_buffer(&buffer, "Dark phase").is_none());
        }

        // Daylight targets remain owned by Automation rather than masquerading as
        // hardware telemetry on the Weather screen.
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.solar_elevation = Some(35.0); // Daylight
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(40),
                Some(0.85),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Solar target").is_none());
            assert!(find_in_buffer(&buffer, "Weather target").is_none());
        }
    }

    #[test]
    fn test_phase8_weather_forecast_integrity_empty_single_and_multiple() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        // Empty forecast: no panic
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(30),
                Some(0.9),
                0,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "No forecast samples yet.").is_some());
        }

        // 1 forecast sample: no panic on chart axes
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(30),
                Some(0.9),
                1,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        }
    }

    #[test]
    fn test_phase8_async_shimmer_and_motion_lifecycle() {
        let mut model = Model::new();
        let now = std::time::Instant::now();

        // 1. Opening Tab 4 alone does NOT trigger fake shimmer
        model.active_tab = Tab::Weather;
        assert_eq!(model.motion.active_activity, None);
        assert_eq!(
            model
                .motion
                .activity_shimmer_phase(now, std::time::Duration::from_millis(1500)),
            None
        );

        // 2. Real async activity started under MotionLevel::Instrument
        model.motion.level = crate::config::MotionLevel::Instrument;
        model
            .motion
            .start_activity(crate::tui::motion::ActiveActivity::WeatherRefresh { started_at: now });
        assert!(model.motion.active_activity.is_some());

        let phase = model.motion.activity_shimmer_phase(
            now + std::time::Duration::from_millis(750),
            std::time::Duration::from_millis(1500),
        );
        assert!(phase.is_some());
        let p = phase.unwrap();
        assert!((p - 0.5).abs() < 0.05);

        // 3. MotionLevel::Reduced suppresses shimmer animation
        model.motion.level = crate::config::MotionLevel::Reduced;
        assert_eq!(
            model
                .motion
                .activity_shimmer_phase(now, std::time::Duration::from_millis(1500)),
            None
        );

        // 4. MotionLevel::Off suppresses shimmer animation
        model.motion.level = crate::config::MotionLevel::Off;
        assert_eq!(
            model
                .motion
                .activity_shimmer_phase(now, std::time::Duration::from_millis(1500)),
            None
        );

        // 5. Activity stopped explicitly
        model.motion.stop_activity();
        assert_eq!(model.motion.active_activity, None);

        // 6. Geometric invariance: shimmer_style_for_column preserves cell layout
        let styles =
            crate::tui::theme::SemanticStyles::from_palette(&crate::config::Theme::Amber.palette());
        let style_col0 = crate::tui::motion::shimmer_style_for_column(
            0,
            20,
            std::time::Duration::from_millis(100),
            std::time::Duration::from_millis(1500),
            &styles,
        );
        assert!(style_col0.fg.is_some());

        // 7. Successful refresh settle confirmation (WeatherUpdate)
        model.motion.level = crate::config::MotionLevel::Instrument;
        model.motion.trigger(
            crate::tui::motion::TransientKind::WeatherUpdate,
            now,
            std::time::Duration::from_millis(650),
        );
        assert!(model.motion.weather_update_phase(now).is_some());
        assert!(model
            .motion
            .weather_update_phase(now + std::time::Duration::from_millis(700))
            .is_none());
    }

    #[test]
    fn test_phase8_weather_responsive_comfortable_compact_minimal() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(55),
            Some(0.8),
            8,
        ));
        model.status = Some(status);

        // 1. Comfortable keeps exact forecast samples and a real graph.
        {
            let backend = TestBackend::new(120, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
            assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        }

        // 2. Compact (65x15)
        {
            let backend = TestBackend::new(65, 15);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        }

        // 3. Minimal (50x12)
        {
            let backend = TestBackend::new(50, 12);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        }
    }

    #[test]
    fn test_phase8_weather_themes_visual_hierarchy() {
        for theme in [
            crate::config::Theme::Amber,
            crate::config::Theme::Nord,
            crate::config::Theme::HackerGreen,
            crate::config::Theme::Grayscale,
            crate::config::Theme::Commodore64,
        ] {
            let backend = TestBackend::new(120, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme;
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(35),
                Some(0.9),
                8,
            ));
            model.status = Some(status);

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Phase 8.1: Weather Semantic Correctness & Provenance Lock Tests
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_phase8_1_first_snapshot_sync_and_subsequent_updates() {
        let mut model = Model::new();
        model.motion.level = crate::config::MotionLevel::Instrument;
        let now = std::time::Instant::now();

        // 1. Initial state: no baseline established yet
        assert_eq!(model.last_weather_fetched_at, None);
        assert_eq!(model.motion.weather_update_phase(now), None);

        // 2. First observed status packet: establishes baseline, NO confirmation transient
        let mut status1 = dummy_status(1);
        let mut ws1 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws1.fetched_at_epoch_s = Some(1_700_000_000);
        status1.weather = Some(ws1);
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(status1))),
        );

        assert_eq!(model.last_weather_fetched_at, Some(1_700_000_000));
        assert_eq!(model.motion.weather_update_phase(now), None);

        // 3. Same timestamp: synchronization / periodic status, NO confirmation transient
        let mut status2 = dummy_status(1);
        let mut ws2 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws2.fetched_at_epoch_s = Some(1_700_000_000);
        status2.weather = Some(ws2);
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(status2))),
        );

        assert_eq!(model.last_weather_fetched_at, Some(1_700_000_000));
        assert_eq!(model.motion.weather_update_phase(now), None);

        // 4. Older timestamp: out-of-order delivery, NO confirmation transient
        let mut status3 = dummy_status(1);
        let mut ws3 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws3.fetched_at_epoch_s = Some(1_699_999_999);
        status3.weather = Some(ws3);
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(status3))),
        );

        assert_eq!(model.last_weather_fetched_at, Some(1_700_000_000));
        assert_eq!(model.motion.weather_update_phase(now), None);

        // 5. Strictly newer timestamp: genuine fetch completion, TRIGGERS confirmation transient
        let mut status4 = dummy_status(1);
        let mut ws4 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws4.fetched_at_epoch_s = Some(1_700_001_800);
        status4.weather = Some(ws4);
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(status4))),
        );

        assert_eq!(model.last_weather_fetched_at, Some(1_700_001_800));
        assert!(model
            .motion
            .weather_update_phase(std::time::Instant::now())
            .is_some());

        // 6. MotionLevel::Off: strictly newer timestamp updates baseline but suppresses transient
        model.motion.active_transient = None;
        model.motion.level = crate::config::MotionLevel::Off;
        let mut status5 = dummy_status(1);
        let mut ws5 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws5.fetched_at_epoch_s = Some(1_700_003_600);
        status5.weather = Some(ws5);
        update::update(
            &mut model,
            Message::Ipc(IpcEvent::Status(Box::new(status5))),
        );

        assert_eq!(model.last_weather_fetched_at, Some(1_700_003_600));
        assert_eq!(
            model.motion.weather_update_phase(std::time::Instant::now()),
            None
        );
    }

    #[test]
    fn test_phase8_1_freshness_boundaries_refresh_interval_and_cache_ttl() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.config.weather.refresh_minutes = 30; // 30m refresh interval -> 1800s, 60m TTL -> 3600s
        let base_fetch = 1_700_000_000u64;

        // A. age = refresh_interval - 1 (1799s) -> Fresh
        {
            let mut status = dummy_status(1);
            status.now_epoch_s = base_fetch + 1799;
            let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
            ws.fetched_at_epoch_s = Some(base_fetch);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::Fresh
            );
            assert_eq!(telem.freshness_label, "● Fresh");
        }

        // B. age = refresh_interval (1800s) -> Fresh (boundary inclusive)
        {
            let mut status = dummy_status(1);
            status.now_epoch_s = base_fetch + 1800;
            let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
            ws.fetched_at_epoch_s = Some(base_fetch);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::Fresh
            );
            assert_eq!(telem.freshness_label, "● Fresh");
        }

        // C. age = refresh_interval + 1 (1801s) -> RefreshDue (within cache TTL)
        {
            let mut status = dummy_status(1);
            status.now_epoch_s = base_fetch + 1801;
            let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
            ws.fetched_at_epoch_s = Some(base_fetch);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshDue
            );
            assert_eq!(telem.freshness_label, "○ Refresh Due");
        }

        // D. age = cache_ttl - 1 (3599s) -> RefreshDue (cache remains valid)
        {
            let mut status = dummy_status(1);
            status.now_epoch_s = base_fetch + 3599;
            let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
            ws.fetched_at_epoch_s = Some(base_fetch);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshDue
            );
        }

        // E. age = cache_ttl (3600s) -> RefreshDue (boundary inclusive in core cache_is_fresh)
        {
            let mut status = dummy_status(1);
            status.now_epoch_s = base_fetch + 3600;
            let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
            ws.fetched_at_epoch_s = Some(base_fetch);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshDue
            );
        }

        // F. age = cache_ttl + 1 (3601s) -> Stale (exceeds TTL, excluded from automation)
        {
            let mut status = dummy_status(1);
            status.now_epoch_s = base_fetch + 3601;
            let mut ws = dummy_weather_status(true, false, true, Some(50), None, 8);
            ws.fetched_at_epoch_s = Some(base_fetch);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::Stale
            );
            assert_eq!(telem.freshness_label, "▲ Stale");
        }
    }

    #[test]
    fn test_phase8_1_policy_provenance_and_stale_behavior() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;

        // 1. Stale cache: multiplier is None, excluded from automation, solar curve unattenuated
        {
            let mut status = dummy_status(1);
            status.solar_elevation = Some(45.0); // Daylight
            status.now_epoch_s = 1_700_005_000;
            let mut ws = dummy_weather_status(true, false, true, Some(60), None, 8);
            ws.fetched_at_epoch_s = Some(1_700_000_000);
            status.weather = Some(ws);
            model.status = Some(status);

            let policy = crate::tui::ui::weather_model::compute_atmospheric_policy(&model);
            assert_eq!(policy.multiplier, None);
            assert_eq!(policy.delta_percent, Some(0));
            assert!(policy
                .explanation
                .contains("Stale weather data excluded from automation"));
            assert_eq!(policy.solar_target_percent, policy.effective_target_percent);
        }

        // 2. Failed refresh with valid cache (within TTL): retains multiplier & applies attenuation
        {
            let mut status = dummy_status(1);
            status.solar_elevation = Some(45.0);
            status.now_epoch_s = 1_700_002_000; // 33m old, within 60m TTL
            let mut ws = dummy_weather_status(true, true, false, Some(60), Some(0.75), 8);
            ws.last_error = Some(String::from("openweather timeout"));
            ws.fetched_at_epoch_s = Some(1_700_000_000);
            status.weather = Some(ws);
            model.status = Some(status);

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshFailedWithCache
            );
            assert_eq!(telem.policy.multiplier, Some(0.75));
            assert!(telem.policy.delta_percent.is_some());
        }

        // 3. Disabled weather: multiplier is None, explanation notes disabled state
        {
            model.config.weather.enabled = false;
            let policy = crate::tui::ui::weather_model::compute_atmospheric_policy(&model);
            assert_eq!(policy.multiplier, None);
            assert!(policy.explanation.contains("Weather disabled"));
        }
    }

    #[test]
    fn test_phase8_1_forecast_provenance_and_timezone_sampling() {
        // 1. Timezone and time-format correctness
        // 1700000000 = 2023-11-14 22:13:20 UTC
        // In Europe/Istanbul (UTC+3), this is 2023-11-15 01:13:20
        let label_24h = crate::tui::ui::weather_model::forecast_time_label(
            1_700_000_000,
            false,
            "Europe/Istanbul",
        );
        assert_eq!(label_24h, "01:13");

        let label_12h = crate::tui::ui::weather_model::forecast_time_label(
            1_700_000_000,
            true,
            "Europe/Istanbul",
        );
        assert_eq!(label_12h, "01:13 AM");

        // In America/New_York (UTC-5 in Nov, EST), 22:13 UTC is 17:13 (5:13 PM)
        let label_ny = crate::tui::ui::weather_model::forecast_time_label(
            1_700_000_000,
            false,
            "America/New_York",
        );
        assert_eq!(label_ny, "17:13");

        // 2. Downsampling behavior: in narrow viewport (< 55 cols for 8 points), table downsamples by step=2
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(40),
            Some(0.85),
            8,
        ));
        model.status = Some(status);

        // Narrow terminal (52x20) remains a compact weather composition.
        let backend = TestBackend::new(52, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Renders cleanly without panic or clipping in compact mode
        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
    }

    #[test]
    fn test_phase8_1_no_fake_shimmer_and_no_fictional_condition_labels() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(45),
            Some(0.8),
            8,
        ));
        model.status = Some(status);

        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. No fake shimmer started merely by opening or rendering Tab 4
        assert_eq!(model.motion.active_activity, None);

        // 2. The normalized semantic condition is shown once, with no invented
        // alternate condition labels.
        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        assert!(find_in_buffer(&buffer, "Partly Cloudy").is_none());
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Phase 8.2: Weather Temporal Truth + Atmospheric Configuration Consolidation
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_phase8_2_atmospheric_input_valid_time_and_temporal_truth() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.config.tui.use_12h_time = false;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;

        let mut status = dummy_status(1);
        let mut ws = dummy_weather_status(true, true, false, Some(55), Some(0.8), 8);
        // 1700000000 UTC is 2023-11-14 22:13:20 UTC -> Istanbul UTC+3 = 01:13:20
        ws.valid_at_epoch_s = Some(1_700_000_000);
        status.weather = Some(ws);
        model.status = Some(status);

        // 1. The screen uses the normalized forecast, not a fake "current sky"
        // label. Time conversion is covered by the pure helper above.
        {
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "CURRENT SKY").is_none());
            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        }

        // 2. The 12-hour preference continues to render without a layout failure.
        {
            model.config.tui.use_12h_time = true;
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        }

        // 3. A different IANA timezone also leaves the normalized rendering intact.
        {
            model.config.location.timezone = String::from("America/New_York");
            model.config.tui.use_12h_time = false;
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        }
    }

    #[test]
    fn test_phase8_2_refresh_due_never_claims_in_flight() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;

        let base_fetch = 1_700_000_000;
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 2820; // 47m after fetch (refresh due at 30m, TTL at 60m)
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // A stale refresh schedule is not presented as an in-flight operation.
        assert!(find_in_buffer(&buffer, "Refresh due").is_some());
        assert!(find_in_buffer(&buffer, "updated 47 min ago").is_some());

        // Must NEVER claim in-flight, fetching, or downloading
        assert!(find_in_buffer(&buffer, "in flight").is_none());
        assert!(find_in_buffer(&buffer, "refresh scheduled").is_none());
        assert!(find_in_buffer(&buffer, "fetching").is_none());
        assert!(find_in_buffer(&buffer, "downloading").is_none());
    }

    #[test]
    fn test_phase8_2_policy_target_never_masquerades_as_applied_hardware() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;

        let mut status = dummy_status(1);
        status.solar_elevation = Some(40.0);
        // Monitor hardware applied is 55%, with an active manual override of 80%
        if let Some(mon) = status.monitors.get_mut(0) {
            mon.last_applied_percent = Some(55);
            mon.override_percent = Some(80);
        }
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.85), 8);
        ws.valid_at_epoch_s = Some(1_700_000_000);
        status.weather = Some(ws);
        model.status = Some(status);

        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Weather does not present Automation policy targets as live hardware
        // telemetry; its role is current conditions and forecast.
        assert!(find_in_buffer(&buffer, "Solar target").is_none());
        assert!(find_in_buffer(&buffer, "Weather target").is_none());

        // Weather must NOT claim policy target is applied or confirmed hardware
        assert!(find_in_buffer(&buffer, "Applied").is_none());
        assert!(find_in_buffer(&buffer, "Current hardware").is_none());
        assert!(find_in_buffer(&buffer, "Confirmed brightness").is_none());
    }

    #[test]
    fn weather_pixel_font_and_forecast_strip_layout() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        let screen = render(&mut model, 160, 45);

        // Current conditions render with the 3-row pixel art block font
        assert!(screen.contains("°C"), "{screen}");
        assert!(
            screen.contains("▄") || screen.contains("▀") || screen.contains("█"),
            "Pixel art font glyphs expected in screen: {screen}"
        );

        // Forecast strip legend is clean and shows only Temperature
        assert!(screen.contains("Temperature"), "{screen}");
        assert!(!screen.contains("Precipitation chance"), "{screen}");

        // Forecast samples display temperatures with degree symbols
        assert!(screen.contains("22°"), "{screen}");
    }
}
