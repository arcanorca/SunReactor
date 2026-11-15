use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::weather_art::{weather_art, ART_ROWS, ART_WIDTH};
use super::super::weather_model::WeatherViewModel;
use super::super::{fonts, kit};
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;

pub(super) fn freshness_spans(
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) -> Vec<Span<'static>> {
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

pub(super) fn detail_row(label: &str, value: String, styles: &SemanticStyles) -> Line<'static> {
    labelled_row(label, DETAIL_LABEL_WIDTH, value, styles)
}

pub(super) fn labelled_row(
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

pub(super) fn temperature_in_unit(
    celsius: f32,
    unit: crate::config::TemperatureUnit,
) -> (f32, &'static str) {
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
pub(super) fn render_conditions(
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
            super::super::weather_model::format_temp(feels_like, unit),
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
            for (index, row) in fonts::weather_pixel_font_rows(&format!("{value:.0}"))
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

/// A colour for a temperature: cool blue through a mild gold to warm coral.
pub(super) fn temperature_color(celsius: f32) -> ratatui::style::Color {
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
        ratatui::style::Color::Rgb(from.0, from.1, from.2),
        ratatui::style::Color::Rgb(to.0, to.1, to.2),
        amount,
    )
}
