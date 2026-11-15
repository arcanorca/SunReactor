use std::time::Instant;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::weather_model::WeatherViewModel;
use super::super::{automation, kit, light_cycle, weather_art};
use super::conditions::{temperature_color, temperature_in_unit};
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;
use crate::weather::{WeatherCondition, WeatherDayPhase};

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
pub(super) fn render_forecast(
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
                    cell.set_symbol(light_cycle::EIGHTHS[partial])
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
            .map(|time| automation::format_time(&time, app.config.tui.use_12h_time))
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
            let icon = weather_art::mini_icon(
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
