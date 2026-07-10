use ratatui::style::Color;

use crate::ipc::{StatusResponse, WeatherStatus};
use crate::solar::LunarPhase;

use crate::config::TemperatureUnit;
use crate::tui::theme::Palette;

const AWAITING_DAEMON_MESSAGE: &str = "Awaiting daemon sync...";
const WEATHER_INACTIVE_MESSAGE: &str = "Weather module inactive.\nNo sky data available.";

pub(super) enum WeatherPanelState {
    Message(String),
    Ready(Box<WeatherPanelData>),
}

pub(super) struct WeatherPanelData {
    pub header: WeatherHeader,
    pub forecast_rows: Vec<ForecastRow>,
    pub temperature_chart: TemperatureChart,
}

impl WeatherPanelData {
    pub(super) fn has_forecast(&self) -> bool {
        !self.forecast_rows.is_empty()
    }
}

pub(super) struct WeatherHeader {
    pub art: &'static str,
    pub art_color: Color,
    pub temperature_label: String,
    pub cloud_label: String,
    pub sunrise_label: String,
    pub sunset_label: String,
}

pub(super) struct ForecastRow {
    pub time_label: String,
    pub icon: &'static str,
    pub icon_color: Color,
    pub cloud_label: String,
    pub temperature_label: String,
}

pub(super) struct TemperatureChart {
    pub points: Vec<(f64, f64)>,
    pub min_label: String,
    pub mid_label: String,
    pub max_label: String,
    pub min_temp: f64,
    pub max_temp: f64,
}

pub(super) fn weather_panel_state(
    status: Option<&StatusResponse>,
    use_12h_time: bool,
    timezone: &str,
    unit: TemperatureUnit,
    palette: &Palette,
) -> WeatherPanelState {
    let Some(status) = status else {
        return WeatherPanelState::Message(String::from(AWAITING_DAEMON_MESSAGE));
    };
    let is_weather_active = status.weather.as_ref().is_some_and(|w| w.active);
    
    WeatherPanelState::Ready(Box::new(WeatherPanelData {
        header: build_header(status, use_12h_time, timezone, unit, palette),
        forecast_rows: if is_weather_active {
            build_forecast_rows(status.weather.as_ref().unwrap(), use_12h_time, timezone, unit, palette)
        } else {
            vec![]
        },
        temperature_chart: if is_weather_active {
            build_temperature_chart(status.weather.as_ref().unwrap(), unit)
        } else {
            TemperatureChart {
                points: vec![],
                min_label: String::new(),
                mid_label: String::new(),
                max_label: String::new(),
                min_temp: 0.0,
                max_temp: 0.0,
            }
        },
    }))
}

fn convert_temperature(celsius: f64, unit: crate::config::TemperatureUnit) -> f64 {
    match unit {
        crate::config::TemperatureUnit::Celsius => celsius,
        crate::config::TemperatureUnit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
    }
}

fn format_temperature(celsius: f64, unit: crate::config::TemperatureUnit) -> String {
    let converted = convert_temperature(celsius, unit);
    let symbol = match unit {
        crate::config::TemperatureUnit::Celsius => "C",
        crate::config::TemperatureUnit::Fahrenheit => "F",
    };
    format!("{converted:.1}°{symbol}")
}

fn inactive_weather_message(weather: &WeatherStatus, use_12h_time: bool, timezone: &str) -> String {
    if let Some(error) = weather.last_error.as_deref() {
        let headline = if weather.stale {
            "Weather data stale."
        } else {
            "Weather refresh failed."
        };
        let retry_line = retry_line(weather.next_refresh_at_epoch_s, use_12h_time, timezone);
        return format!("{headline}\n{}\n{retry_line}", truncate_message(error, 54));
    }

    if weather.stale {
        return format!(
            "Weather data stale.\n{}",
            retry_line(weather.next_refresh_at_epoch_s, use_12h_time, timezone)
        );
    }

    if weather.observed_at_epoch_s.is_some() {
        return String::from("Weather data incomplete.\nNo usable cloud sample available.");
    }

    String::from(WEATHER_INACTIVE_MESSAGE)
}

fn retry_line(next_refresh_at_epoch_s: Option<u64>, use_12h_time: bool, timezone: &str) -> String {
    match next_refresh_at_epoch_s {
        Some(next_refresh_at_epoch_s) => format!(
            "Retry scheduled at {}.",
            forecast_time_label(next_refresh_at_epoch_s, use_12h_time, timezone)
        ),
        None => String::from("Retry scheduled on next daemon tick."),
    }
}

fn truncate_message(message: &str, max_chars: usize) -> String {
    let normalized = message.replace('\n', " ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    let truncated: String = normalized
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect();
    format!("{truncated}…")
}

fn build_header(
    status: &StatusResponse,
    use_12h_time: bool,
    timezone: &str,
    unit: crate::config::TemperatureUnit,
    palette: &Palette,
) -> WeatherHeader {
    let is_night = status
        .solar_elevation
        .is_some_and(|elevation| elevation < 0.0);

    let (cloud_pct, temp_str, cloud_str) = match status.weather.as_ref() {
        Some(weather) if weather.active => {
            let pct = weather.cloud_cover_percent.unwrap_or(0);
            let t_str = format_temperature(f64::from(weather.temperature.unwrap_or(0.0)), unit);
            (pct, t_str, format!("{pct}%"))
        }
        _ => (0, String::from("--"), String::from("--")),
    };

    let (art, art_color) = weather_art(cloud_pct, is_night, status.lunar_phase, palette);

    let sunrise_label = status.sunrise_epoch_s.map_or_else(
        || String::from("Sunrise: --:--"),
        |s| {
            format!(
                "Sunrise: {}",
                forecast_time_label(s, use_12h_time, timezone)
            )
        },
    );
    let sunset_label = status.sunset_epoch_s.map_or_else(
        || String::from("Sunset: --:--"),
        |s| format!("Sunset: {}", forecast_time_label(s, use_12h_time, timezone)),
    );

    WeatherHeader {
        art,
        art_color,
        temperature_label: format!("Current Temp: {temp_str}"),
        cloud_label: format!("Cloudiness: {cloud_str}"),
        sunrise_label,
        sunset_label,
    }
}

fn build_forecast_rows(
    weather: &WeatherStatus,
    use_12h_time: bool,
    timezone: &str,
    unit: crate::config::TemperatureUnit,
    palette: &Palette,
) -> Vec<ForecastRow> {
    weather
        .forecast
        .iter()
        .take(8)
        .map(|point| {
            let (icon, icon_color) = forecast_icon(point.cloud_cover_percent, palette);
            ForecastRow {
                time_label: forecast_time_label(point.dt_epoch_s, use_12h_time, timezone),
                icon,
                icon_color,
                cloud_label: format!("{:>3}%", point.cloud_cover_percent),
                temperature_label: format_temperature(f64::from(point.temperature), unit),
            }
        })
        .collect()
}

fn build_temperature_chart(
    weather: &WeatherStatus,
    unit: crate::config::TemperatureUnit,
) -> TemperatureChart {
    let mut points = Vec::with_capacity(weather.forecast.len().min(8) + 1);
    let base_temp = f64::from(weather.temperature.unwrap_or(0.0));
    points.push((0.0, convert_temperature(base_temp, unit)));
    for (index, point) in weather.forecast.iter().take(8).enumerate() {
        points.push((
            (index + 1) as f64,
            convert_temperature(f64::from(point.temperature), unit),
        ));
    }

    let min_temp = points
        .iter()
        .map(|(_, value)| *value)
        .fold(f64::INFINITY, f64::min)
        .floor()
        - 1.0;
    let max_temp = points
        .iter()
        .map(|(_, value)| *value)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        + 1.0;
    let mid_temp = f64::midpoint(min_temp, max_temp);

    TemperatureChart {
        points,
        min_label: format!("{min_temp:.0}"),
        mid_label: format!("{mid_temp:.0}"),
        max_label: format!("{max_temp:.0}"),
        min_temp,
        max_temp,
    }
}

fn forecast_time_label(epoch_s: u64, use_12h_time: bool, timezone: &str) -> String {
    let dt_utc = chrono::DateTime::from_timestamp(epoch_s as i64, 0).unwrap_or_default();

    let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone);
    let offset = std::fs::read(&path)
        .ok()
        .and_then(|data| tz::TimeZone::from_tz_data(&data).ok())
        .or_else(|| tz::TimeZone::from_posix_tz(timezone).ok())
        .and_then(|tz| {
            tz.find_local_time_type(epoch_s as i64)
                .map(tz::LocalTimeType::ut_offset)
                .ok()
        })
        .map_or_else(
            || chrono::FixedOffset::east_opt(0).unwrap(),
            |offset| chrono::FixedOffset::east_opt(offset).unwrap(),
        );

    let dt = dt_utc.with_timezone(&offset);
    if use_12h_time {
        dt.format("%I:%M %p").to_string()
    } else {
        dt.format("%H:%M").to_string()
    }
}

fn forecast_icon(cloud_pct: u8, palette: &Palette) -> (&'static str, Color) {
    match cloud_pct {
        0..=19 => ("☀️ ", palette.accent),
        20..=59 => ("⛅ ", palette.secondary_accent),
        _ => ("☁️ ", palette.text_muted),
    }
}

/// Returns the ASCII art and color for the current sky condition.
fn weather_art(
    cloud_pct: u8,
    is_night: bool,
    lunar_phase: Option<LunarPhase>,
    palette: &Palette,
) -> (&'static str, Color) {
    match (cloud_pct, is_night) {
        // ── Clear night: show the moon in its current phase ──────────────
        (0..=19, true) => {
            // If weather is disabled/missing but it's night, we default to None.
            // Oh wait, if lunar_phase is None, let's just use NewMoon as a safe blank,
            // or maybe we shouldn't default to FullMoon which is confusing.
            let phase = lunar_phase.unwrap_or(LunarPhase::NewMoon);
            lunar_phase_art(phase, palette)
        }

        // ── Clear day: sun with rays ──────────────────────────────────────
        (0..=19, false) => (
            "\n   \\  ──  / \n  ──▄████▄──\n  ──██████──\n  ──▀████▀──\n   /  ──  \\ ",
            palette.accent,
        ),

        // ── Partly cloudy night: show phase peeking behind cloud ──────────
        (20..=59, true) => {
            let phase = lunar_phase.unwrap_or(LunarPhase::NewMoon);
            let (_, color) = lunar_phase_art(phase, palette);
            (partly_cloudy_night_art(phase), color)
        }

        // ── Partly cloudy day ─────────────────────────────────────────────
        (20..=59, false) => (
            "\n     \\|/    \n   ─▄████▄▄ \n  ▄█████████\n  ▀█████████\n    ▀▀▀▀▀▀  ",
            palette.accent,
        ),

        // ── Overcast / heavy cloud ────────────────────────────────────────
        _ => (
            "\n            \n     ▄▄▄▄   \n   ▄██████▄ \n  ▄█████████\n  ▀█████████",
            palette.text_muted,
        ),
    }
}

/// Returns the half-block art and colour for a specific lunar phase.
///
/// Art design notes:
///  • 5-line height, 14-char width — fits the 16-char art column in the header.
///  • Upper/lower half-blocks (▀ ▄) double the effective vertical resolution,
///    making the terminator line crisp despite monospace cell aspect ratio ~1:2.
///  • █ = fully lit surface, ▓ = bright limb glow, ▒ = mid shadow, ░ = deep shadow.
///  • Stars (· ✦ ✧) provide depth cues on the night side.
fn lunar_phase_art(phase: LunarPhase, palette: &Palette) -> (&'static str, Color) {
    match phase {
        LunarPhase::NewMoon => (
            "\n ✦  ░░░░░░  · \n  ░░░░░░░░░░  \n ░░░░░░░░░░░░✧\n  ░░░░░░░░░░  \n ·  ░░░░░░  ✦ ",
            palette.text_muted,
        ),
        LunarPhase::WaxingCrescent => (
            "\n ✦  ░░░░██  · \n  ░░░░░░░███  \n ░░░░░░░░░███✧\n  ░░░░░░░███  \n ·  ░░░░██  ✦ ",
            palette.fg,
        ),
        LunarPhase::FirstQuarter => (
            "\n ✦  ░░░███  · \n  ░░░░░█████  \n ░░░░░░██████✧\n  ░░░░░█████  \n ·  ░░░███  ✦ ",
            palette.fg,
        ),
        LunarPhase::WaxingGibbous => (
            "\n ✦  ░█████  · \n  ░░████████  \n ░░██████████✧\n  ░░████████  \n ·  ░█████  ✦ ",
            palette.fg,
        ),
        LunarPhase::FullMoon => (
            "\n ✦  ██████  · \n  ██████████  \n ████████████✧\n  ██████████  \n ·  ██████  ✦ ",
            palette.accent,
        ),
        LunarPhase::WaningGibbous => (
            "\n ✦  █████░  · \n  ████████░░  \n ██████████░░✧\n  ████████░░  \n ·  █████░  ✦ ",
            palette.fg,
        ),
        LunarPhase::LastQuarter => (
            "\n ✦  ███░░░  · \n  █████░░░░░  \n ██████░░░░░░✧\n  █████░░░░░  \n ·  ███░░░  ✦ ",
            palette.fg,
        ),
        LunarPhase::WaningCrescent => (
            "\n ✦  ██░░░░  · \n  ███░░░░░░░  \n ███░░░░░░░░░✧\n  ███░░░░░░░  \n ·  ██░░░░  ✦ ",
            palette.fg,
        ),
    }
}

/// Partly-cloudy night art with the lunar phase peeking above a cloud band.
fn partly_cloudy_night_art(phase: LunarPhase) -> &'static str {
    match phase {
        LunarPhase::NewMoon => {
            "\n ✦  ░░░░░░  · \n  ░░░░░░░░░░  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::WaxingCrescent => {
            "\n ✦  ░░░░██  · \n  ░░░░░░░███  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::FirstQuarter => {
            "\n ✦  ░░░███  · \n  ░░░░░█████  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::WaxingGibbous => {
            "\n ✦  ░█████  · \n  ░░████████  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::FullMoon => {
            "\n ✦  ██████  · \n  ██████████  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::WaningGibbous => {
            "\n ✦  █████░  · \n  ████████░░  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::LastQuarter => {
            "\n ✦  ███░░░  · \n  █████░░░░░  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
        LunarPhase::WaningCrescent => {
            "\n ✦  ██░░░░  · \n  ███░░░░░░░  \n    ▄▄████▄▄  \n   ▄█████████ \n   ▀█████████ "
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{weather_panel_state, WeatherPanelState};
    use crate::config::TemperatureUnit;
    use crate::ipc::{MonitorStatus, StatusResponse, WeatherStatus};
    use crate::state::ForecastPoint;

    #[test]
    fn returns_message_when_status_is_missing() {
        assert!(matches!(
            weather_panel_state(
                None,
                false,
                "UTC",
                TemperatureUnit::Celsius,
                &crate::tui::theme::Theme::Terminal.palette()
            ),
            WeatherPanelState::Message(ref message) if message == "Awaiting daemon sync..."
        ));
    }

    #[test]
    fn builds_ready_panel_for_active_weather() {
        let status = StatusResponse {
            now_epoch_s: 0,
            sunrise_epoch_s: Some(0),
            sunset_epoch_s: Some(0),
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
                multiplier: Some(1.0),
                enabled: true,
                active: true,
                stale: false,
                provider: Some(String::from("openweather")),
                observed_at_epoch_s: Some(1_700_000_000),
                last_refresh_attempt_epoch_s: Some(1_700_000_000),
                next_refresh_at_epoch_s: Some(1_700_000_600),
                consecutive_failures: 0,
                last_error: None,
                cloud_cover_percent: Some(42),
                temperature: Some(21.5),
                forecast: vec![ForecastPoint {
                    dt_epoch_s: 1_700_000_000,
                    cloud_cover_percent: 55,
                    temperature: 19.0,
                }],
            }),
            monitors: Vec::<MonitorStatus>::new(),
            solar_elevation: Some(12.0),
            lunar_phase: None,
        };

        let WeatherPanelState::Ready(panel) = weather_panel_state(
            Some(&status),
            false,
            "UTC",
            TemperatureUnit::Celsius,
            &crate::tui::theme::Theme::Terminal.palette(),
        ) else {
            panic!("expected ready weather panel");
        };

        assert_eq!(panel.header.temperature_label, "Current Temp: 21.5°C");
        assert_eq!(panel.header.cloud_label, "Cloudiness: 42%");
        assert_eq!(panel.forecast_rows.len(), 1);
        assert_eq!(panel.temperature_chart.points.len(), 2);
    }

    #[test]
    fn explains_stale_weather_with_retry_and_error() {
        let status = StatusResponse {
            now_epoch_s: 0,
            sunrise_epoch_s: Some(0),
            sunset_epoch_s: Some(0),
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
                multiplier: Some(1.0),
                enabled: true,
                active: false,
                stale: true,
                provider: Some(String::from("openweather")),
                observed_at_epoch_s: Some(1_700_000_000),
                last_refresh_attempt_epoch_s: Some(1_700_000_030),
                next_refresh_at_epoch_s: Some(1_700_000_060),
                consecutive_failures: 2,
                last_error: Some(String::from("openweather request failed: network timeout")),
                cloud_cover_percent: Some(42),
                temperature: Some(21.5),
                forecast: vec![],
            }),
            monitors: Vec::<MonitorStatus>::new(),
            solar_elevation: Some(12.0),
            lunar_phase: None,
        };

        let WeatherPanelState::Message(message) = weather_panel_state(
            Some(&status),
            false,
            "UTC",
            TemperatureUnit::Celsius,
            &crate::tui::theme::Theme::Terminal.palette(),
        ) else {
            panic!("expected stale weather message");
        };

        assert!(message.contains("Weather data stale."));
        assert!(message.contains("network timeout"));
        assert!(message.contains("Retry scheduled at"));
    }
}
