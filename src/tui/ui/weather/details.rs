use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use super::super::weather_model::WeatherViewModel;
use super::super::{kit, weather_art};
use super::conditions::labelled_row;
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;
use crate::weather::{WeatherCondition, WeatherDayPhase};

/// What the weather means for SunReactor, in one headline and one sentence.
pub(super) fn render_impact(
    f: &mut Frame,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
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
    let icon = weather_art::mini_icon(
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

pub(super) fn render_additional(
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
            rows.push((
                "Dew point",
                super::super::weather_model::format_temp(dew, unit),
            ));
        }
    }
    if let Some(chance) = details.precipitation_percent {
        rows.push(("Precipitation", format!("{chance}% chance")));
    }
    if let (Some(celsius), Some(speed)) = (view.temperature_celsius, details.wind_speed_mps) {
        if let Some(chill) = wind_chill(celsius, speed * 3.6) {
            rows.push((
                "Wind chill",
                super::super::weather_model::format_temp(chill, unit),
            ));
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
