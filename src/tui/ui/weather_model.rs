use super::weather_art::MoonView;
use crate::config::TemperatureUnit;
use crate::ipc::WeatherStatus;
use crate::tui::Model;
use crate::weather::{WeatherCondition, WeatherDayPhase, WeatherState};

/// The weather screen's user-facing state. It deliberately describes what a
/// person can act on rather than exposing daemon or provider implementation
/// details in the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WeatherViewState {
    DaemonUnavailable,
    Disabled,
    Loading,
    NoApiKey,
    Unauthorized,
    RateLimited,
    NetworkError,
    ProviderError,
    ParseError,
    Ready,
    Stale,
}

impl WeatherViewState {
    #[must_use]
    pub(crate) const fn is_error(self) -> bool {
        matches!(
            self,
            Self::Unauthorized
                | Self::RateLimited
                | Self::NetworkError
                | Self::ProviderError
                | Self::ParseError
        )
    }

    #[must_use]
    pub(crate) const fn is_warning(self) -> bool {
        matches!(self, Self::Stale | Self::NoApiKey | Self::RateLimited)
    }
}

/// Presentation data for Weather. The model contains no Ratatui types and
/// receives already-normalized weather condition data from IPC.
#[derive(Debug, Clone)]
pub(crate) struct WeatherViewModel {
    pub state: WeatherViewState,
    pub has_current_conditions: bool,
    pub condition: WeatherCondition,
    pub cloud_cover_percent: Option<u8>,
    pub day_phase: WeatherDayPhase,
    /// The moon as it looks now at the configured latitude.
    pub moon: Option<MoonView>,
    pub condition_label: String,
    pub condition_description: Option<String>,
    pub temperature_label: String,
    pub location_label: String,
    pub forecast: Vec<WeatherForecastPoint>,
    pub source_label: String,
    pub status_label: String,
    pub status_detail: String,
    /// Local time the used forecast sample is valid for.
    pub valid_at_label: Option<String>,
    /// The cache is still usable but its scheduled refresh time has passed.
    pub refresh_due: bool,
    pub temperature_celsius: Option<f32>,
    /// When the forecast sample in use is valid.
    pub valid_at_epoch_s: Option<u64>,
    pub details: crate::state::WeatherDetails,
    /// The multiplier weather currently applies to daylight, when it is used.
    pub multiplier: Option<f64>,
}

#[derive(Debug, Clone)]
pub(crate) struct WeatherForecastPoint {
    pub temperature_celsius: f32,
    pub cloud_cover_percent: u8,
    pub time_label: String,
    pub epoch_s: u64,
    pub condition: WeatherCondition,
    pub day_phase: Option<WeatherDayPhase>,
    #[allow(dead_code)]
    pub precipitation_percent: Option<u8>,
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub(crate) fn weather_view_model(app: &Model) -> WeatherViewModel {
    let status = app.status.as_ref();
    let weather = status.and_then(|status| status.weather.as_ref());
    let state = weather_view_state(app.config.weather.enabled, status.is_some(), weather);
    let has_current_conditions = has_displayable_conditions(state, weather);

    let condition = weather.map_or(WeatherCondition::Unknown, |weather| weather.condition);
    let cloud_cover_percent = weather.and_then(|weather| weather.cloud_cover_percent);
    let day_phase = weather
        .and_then(|weather| weather.day_phase)
        .or_else(|| {
            status.and_then(|status| {
                status.solar_elevation.map(|elevation| {
                    if elevation < 0.0 {
                        WeatherDayPhase::Night
                    } else {
                        WeatherDayPhase::Day
                    }
                })
            })
        })
        .unwrap_or(WeatherDayPhase::Day);
    let condition_label = condition_label(condition, cloud_cover_percent);
    let temperature_label = weather.and_then(|weather| weather.temperature).map_or_else(
        || String::from("--"),
        |temperature| format_temp(temperature, app.config.tui.temperature_unit),
    );
    let location_label = location_label(app);
    let forecast = weather.map_or_else(Vec::new, |weather| {
        weather
            .forecast
            .iter()
            .take(8)
            .map(|point| WeatherForecastPoint {
                temperature_celsius: point.temperature,
                cloud_cover_percent: point.cloud_cover_percent,
                time_label: app.local_time_at_epoch(point.dt_epoch_s).map_or_else(
                    || String::from("--:--"),
                    |time| format_weather_time(time, app.config.tui.use_12h_time),
                ),
                epoch_s: point.dt_epoch_s,
                condition: point.condition,
                day_phase: point.day_phase,
                precipitation_percent: point.precipitation_percent,
            })
            .collect()
    });
    let now_epoch_s = status.map_or(0, |status| status.now_epoch_s);
    let observation_age = weather
        .and_then(|weather| weather.fetched_at_epoch_s)
        .map(|fetched_at_epoch_s| format_observation_age(now_epoch_s, fetched_at_epoch_s));
    let (mut status_label, mut status_detail, _) =
        weather_state_copy(state, has_current_conditions);
    // Core schedules refreshes; a passed deadline with usable data is only
    // "refresh due", never a claim that a request is in flight.
    let refresh_due = state == WeatherViewState::Ready
        && weather
            .and_then(|weather| weather.next_refresh_at_epoch_s)
            .is_some_and(|next| now_epoch_s > 0 && next <= now_epoch_s);
    if refresh_due {
        status_label = String::from("Refresh due");
    }
    if let Some(age) = observation_age {
        if status_detail.is_empty() {
            status_detail = age;
        } else {
            status_detail = format!("{status_detail} · {age}");
        }
    }

    WeatherViewModel {
        state,
        has_current_conditions,
        condition,
        cloud_cover_percent,
        day_phase,
        moon: Some(MoonView {
            age: crate::solar::lunar_age_fraction(
                status
                    .map(|status| status.now_epoch_s)
                    .filter(|epoch| *epoch > 0)
                    .and_then(|epoch| chrono::DateTime::from_timestamp(epoch as i64, 0))
                    .unwrap_or_else(chrono::Utc::now),
            ),
            southern: app.config.location.latitude < 0.0,
        }),
        condition_label,
        condition_description: weather
            .and_then(|weather| weather.condition_description.clone())
            .filter(|description| !description.trim().is_empty()),
        temperature_label,
        location_label,
        forecast,
        source_label: weather
            .and_then(|weather| weather.provider.as_deref())
            .map(str::trim)
            .filter(|provider| !provider.is_empty())
            .map_or_else(
                || String::from("OpenWeather"),
                |provider| {
                    if provider.eq_ignore_ascii_case("openweather") {
                        String::from("OpenWeather")
                    } else {
                        provider.to_owned()
                    }
                },
            ),
        status_label,
        status_detail,
        valid_at_label: weather
            .and_then(|weather| weather.valid_at_epoch_s)
            .and_then(|epoch_s| app.local_time_at_epoch(epoch_s))
            .map(|time| format_weather_time(time, app.config.tui.use_12h_time)),
        refresh_due,
        temperature_celsius: weather.and_then(|weather| weather.temperature),
        valid_at_epoch_s: weather.and_then(|weather| weather.valid_at_epoch_s),
        details: weather.map(|weather| weather.details).unwrap_or_default(),
        multiplier: weather
            .filter(|weather| weather.active && !weather.stale)
            .and_then(|weather| weather.multiplier),
    }
}

/// Returns the currently actionable Weather command label without requiring a
/// renderer. Footer and in-view hints use this shared state mapping so a
/// nominal refresh is never advertised as a retry.
#[must_use]
pub(crate) fn weather_action_label(app: &Model) -> Option<&'static str> {
    let status = app.status.as_ref();
    let weather = status.and_then(|status| status.weather.as_ref());
    let state = weather_view_state(app.config.weather.enabled, status.is_some(), weather);
    let has_current_conditions = has_displayable_conditions(state, weather);
    weather_state_copy(state, has_current_conditions).2
}

fn weather_view_state(
    weather_enabled: bool,
    daemon_available: bool,
    weather: Option<&WeatherStatus>,
) -> WeatherViewState {
    if !weather_enabled {
        return WeatherViewState::Disabled;
    }
    if !daemon_available {
        return WeatherViewState::DaemonUnavailable;
    }
    let Some(weather) = weather else {
        return WeatherViewState::Loading;
    };

    match normalized_weather_state(weather) {
        WeatherState::Disabled => WeatherViewState::Disabled,
        WeatherState::NoApiKey => WeatherViewState::NoApiKey,
        WeatherState::Loading => WeatherViewState::Loading,
        WeatherState::Ready => WeatherViewState::Ready,
        WeatherState::Stale => WeatherViewState::Stale,
        WeatherState::Unauthorized => WeatherViewState::Unauthorized,
        WeatherState::RateLimited => WeatherViewState::RateLimited,
        WeatherState::NetworkError => WeatherViewState::NetworkError,
        WeatherState::ProviderError => WeatherViewState::ProviderError,
        WeatherState::ParseError => WeatherViewState::ParseError,
    }
}

/// Artwork communicates a confirmed observation, not the most recent request
/// outcome. A stale snapshot remains useful when it is explicitly labelled as
/// stale, and a loading refresh can retain the last confirmed observation.
/// Failures instead use the actionable status-only presentation.
fn has_displayable_conditions(state: WeatherViewState, weather: Option<&WeatherStatus>) -> bool {
    matches!(
        state,
        WeatherViewState::Ready | WeatherViewState::Stale | WeatherViewState::Loading
    ) && weather.is_some_and(|weather| {
        weather.condition != WeatherCondition::Unknown || weather.cloud_cover_percent.is_some()
    })
}

fn normalized_weather_state(weather: &WeatherStatus) -> WeatherState {
    if weather.state != WeatherState::Disabled || !weather.enabled {
        return weather.state;
    }

    // Older daemons did not publish an explicit weather state. Preserve their
    // active/stale fields while new daemons use the explicit state machine.
    if weather.stale {
        WeatherState::Stale
    } else if weather.active {
        WeatherState::Ready
    } else if weather.last_error.is_some() {
        WeatherState::NetworkError
    } else {
        WeatherState::Loading
    }
}

fn condition_label(condition: WeatherCondition, cloud_cover_percent: Option<u8>) -> String {
    if condition != WeatherCondition::Unknown {
        return String::from(condition.label());
    }

    cloud_cover_percent.map_or_else(
        || String::from(condition.label()),
        |cloud_cover_percent| format!("{cloud_cover_percent}% cloud cover"),
    )
}

fn location_label(app: &Model) -> String {
    let city = app.config.location.city.trim();
    let timezone = app.config.location.timezone.trim();
    match (city.is_empty(), timezone.is_empty()) {
        (false, false) => format!("{city} · {timezone}"),
        (false, true) => String::from(city),
        (true, false) => String::from(timezone),
        (true, true) => String::from("Location unavailable"),
    }
}

fn weather_state_copy(
    state: WeatherViewState,
    has_current_conditions: bool,
) -> (String, String, Option<&'static str>) {
    match state {
        WeatherViewState::DaemonUnavailable => (
            String::from("Daemon unavailable"),
            String::from("SunReactor is not responding."),
            None,
        ),
        WeatherViewState::Disabled => (
            String::from("Weather is off"),
            String::from("Enable weather in Settings to use cloud-aware brightness."),
            None,
        ),
        WeatherViewState::Loading if has_current_conditions => (
            String::from("Refreshing weather"),
            String::from("Showing last confirmed conditions."),
            None,
        ),
        WeatherViewState::Loading => (
            String::from("Loading weather"),
            String::from("Contacting OpenWeather."),
            None,
        ),
        WeatherViewState::NoApiKey => (
            String::from("API key needed"),
            String::from("Set an OpenWeather API key in Settings."),
            None,
        ),
        WeatherViewState::Unauthorized => (
            String::from("API key rejected"),
            String::from("Update the OpenWeather API key in Settings."),
            Some("Retry"),
        ),
        WeatherViewState::RateLimited => (
            String::from("Request rate limited"),
            String::from("OpenWeather is limiting requests."),
            Some("Retry"),
        ),
        WeatherViewState::NetworkError if has_current_conditions => (
            String::from("Refresh failed"),
            String::from("Showing last confirmed conditions."),
            Some("Retry"),
        ),
        WeatherViewState::NetworkError => (
            String::from("Weather unavailable"),
            String::from("Network request failed."),
            Some("Retry"),
        ),
        WeatherViewState::ProviderError if has_current_conditions => (
            String::from("Refresh failed"),
            String::from("Showing last confirmed conditions."),
            Some("Retry"),
        ),
        WeatherViewState::ProviderError => (
            String::from("Weather unavailable"),
            String::from("OpenWeather returned an unexpected response."),
            Some("Retry"),
        ),
        WeatherViewState::ParseError if has_current_conditions => (
            String::from("Refresh failed"),
            String::from("Showing last confirmed conditions."),
            Some("Retry"),
        ),
        WeatherViewState::ParseError => (
            String::from("Weather unavailable"),
            String::from("OpenWeather sent unreadable data."),
            Some("Retry"),
        ),
        WeatherViewState::Ready => (String::from("Up to date"), String::new(), Some("Refresh")),
        WeatherViewState::Stale => (
            String::from("Stale"),
            String::from("Last data is too old for automation."),
            Some("Retry"),
        ),
    }
}

fn format_observation_age(now_epoch_s: u64, fetched_at_epoch_s: u64) -> String {
    let seconds = now_epoch_s.saturating_sub(fetched_at_epoch_s);
    match seconds {
        0..=59 => String::from("updated just now"),
        60..=3599 => format!("updated {} min ago", seconds / 60),
        _ => format!("updated {} h ago", seconds / 3600),
    }
}

#[must_use]
pub(crate) fn format_temp(celsius: f32, unit: TemperatureUnit) -> String {
    let (value, symbol) = match unit {
        TemperatureUnit::Celsius => (celsius, "°C"),
        TemperatureUnit::Fahrenheit => (celsius * 9.0 / 5.0 + 32.0, "°F"),
    };
    format!("{value:.1}{symbol}")
}

fn format_weather_time(time: chrono::DateTime<chrono::FixedOffset>, use_12h_time: bool) -> String {
    if use_12h_time {
        time.format("%I:%M %p").to_string()
    } else {
        time.format("%H:%M").to_string()
    }
}

// The broad TUI regression suite still characterizes the former telemetry
// screen's time/freshness rules. Keep its small pure adapter test-only while
// production rendering uses `WeatherViewModel` above.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FreshnessState {
    Disabled,
    Fresh,
    RefreshDue,
    Stale,
    RefreshFailedWithCache,
    Unavailable,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct LegacyAtmosphericPolicy {
    pub multiplier: Option<f64>,
    pub solar_target_percent: Option<u8>,
    pub effective_target_percent: Option<u8>,
    pub delta_percent: Option<i16>,
    pub explanation: String,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct LegacyAtmosphericTelemetry {
    pub freshness: FreshnessState,
    pub freshness_label: String,
    pub policy: LegacyAtmosphericPolicy,
}

#[cfg(test)]
pub(crate) fn compute_atmospheric_policy(app: &Model) -> LegacyAtmosphericPolicy {
    let weather = app
        .status
        .as_ref()
        .and_then(|status| status.weather.as_ref());
    let normalized_state = weather.map(normalized_weather_state);
    let stale = matches!(normalized_state, Some(WeatherState::Stale));
    let multiplier = if app.config.weather.enabled && !stale {
        weather.and_then(|weather| weather.multiplier)
    } else {
        None
    };
    let solar_target_percent = Some(0);
    let effective_target_percent = if multiplier.is_some() {
        Some(0)
    } else {
        solar_target_percent
    };
    let explanation = if !app.config.weather.enabled {
        String::from("Weather disabled · solar policy drives display directly")
    } else if stale {
        String::from(
            "Stale weather data excluded from automation · solar curve drives display directly",
        )
    } else if multiplier.is_some() {
        String::from("Cloud cover attenuates solar daylight factor")
    } else {
        String::from("Weather modifier inactive · nominal solar curve active")
    };

    LegacyAtmosphericPolicy {
        multiplier,
        solar_target_percent,
        effective_target_percent,
        delta_percent: Some(0),
        explanation,
    }
}

#[cfg(test)]
pub(crate) fn atmospheric_telemetry(app: &Model) -> LegacyAtmosphericTelemetry {
    let weather = app
        .status
        .as_ref()
        .and_then(|status| status.weather.as_ref());
    let now_epoch_s = app.status.as_ref().map_or(0, |status| status.now_epoch_s);
    let freshness = if !app.config.weather.enabled {
        FreshnessState::Disabled
    } else if let Some(weather) = weather {
        if weather.last_error.is_some() && weather.cloud_cover_percent.is_some() {
            FreshnessState::RefreshFailedWithCache
        } else {
            match normalized_weather_state(weather) {
                WeatherState::Stale => FreshnessState::Stale,
                WeatherState::Ready => {
                    let age_s = weather.fetched_at_epoch_s.map_or(0, |fetched_at_epoch_s| {
                        now_epoch_s.saturating_sub(fetched_at_epoch_s)
                    });
                    if age_s > u64::from(app.config.weather.refresh_minutes) * 60 {
                        FreshnessState::RefreshDue
                    } else {
                        FreshnessState::Fresh
                    }
                }
                _ => FreshnessState::Unavailable,
            }
        }
    } else {
        FreshnessState::Unavailable
    };
    let freshness_label = match freshness {
        FreshnessState::Disabled => String::from("○ Disabled"),
        FreshnessState::Fresh => String::from("● Fresh"),
        FreshnessState::RefreshDue => String::from("○ Refresh Due"),
        FreshnessState::Stale => String::from("▲ Stale"),
        FreshnessState::RefreshFailedWithCache => String::from("! Refresh Failed"),
        FreshnessState::Unavailable => String::from("○ Unavailable"),
    };

    LegacyAtmosphericTelemetry {
        freshness,
        freshness_label,
        policy: compute_atmospheric_policy(app),
    }
}

#[cfg(test)]
pub(crate) fn forecast_time_label(epoch_s: u64, use_12h_time: bool, timezone: &str) -> String {
    let datetime = chrono::DateTime::from_timestamp(epoch_s as i64, 0).unwrap_or_default();
    let timezone_path = std::path::Path::new("/usr/share/zoneinfo").join(timezone);
    let offset = std::fs::read(&timezone_path)
        .ok()
        .and_then(|data| tz::TimeZone::from_tz_data(&data).ok())
        .or_else(|| tz::TimeZone::from_posix_tz(timezone).ok())
        .and_then(|timezone| {
            timezone
                .find_local_time_type(epoch_s as i64)
                .map(tz::LocalTimeType::ut_offset)
                .ok()
        })
        .and_then(chrono::FixedOffset::east_opt)
        .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).expect("UTC offset is valid"));
    format_weather_time(datetime.with_timezone(&offset), use_12h_time)
}

#[cfg(test)]
mod tests {
    use super::{
        format_observation_age, has_displayable_conditions, normalized_weather_state,
        weather_view_state, WeatherViewState,
    };
    use crate::ipc::WeatherStatus;
    use crate::weather::{WeatherCondition, WeatherState};

    #[test]
    fn keeps_legacy_weather_status_truthful() {
        let status = WeatherStatus {
            enabled: true,
            active: true,
            ..WeatherStatus::default()
        };
        assert_eq!(normalized_weather_state(&status), WeatherState::Ready);
    }

    #[test]
    fn distinguishes_daemon_disabled_loading_and_failure_states() {
        assert_eq!(
            weather_view_state(true, false, None),
            WeatherViewState::DaemonUnavailable
        );
        assert_eq!(
            weather_view_state(false, true, None),
            WeatherViewState::Disabled
        );
        assert_eq!(
            weather_view_state(true, true, None),
            WeatherViewState::Loading
        );

        let status = WeatherStatus {
            enabled: true,
            state: WeatherState::NetworkError,
            ..WeatherStatus::default()
        };
        assert_eq!(
            weather_view_state(true, true, Some(&status)),
            WeatherViewState::NetworkError
        );
    }

    #[test]
    fn formats_observation_age_without_future_time_claims() {
        assert_eq!(format_observation_age(10, 20), "updated just now");
        assert_eq!(format_observation_age(3_610, 10), "updated 1 h ago");
    }

    #[test]
    fn only_confirmed_or_explicitly_stale_observations_can_render_art() {
        let observation = WeatherStatus {
            enabled: true,
            condition: WeatherCondition::Clear,
            cloud_cover_percent: Some(0),
            ..WeatherStatus::default()
        };

        for state in [
            WeatherViewState::Ready,
            WeatherViewState::Stale,
            WeatherViewState::Loading,
        ] {
            assert!(has_displayable_conditions(state, Some(&observation)));
        }

        for state in [
            WeatherViewState::DaemonUnavailable,
            WeatherViewState::Disabled,
            WeatherViewState::NoApiKey,
            WeatherViewState::Unauthorized,
            WeatherViewState::RateLimited,
            WeatherViewState::NetworkError,
            WeatherViewState::ProviderError,
            WeatherViewState::ParseError,
        ] {
            assert!(!has_displayable_conditions(state, Some(&observation)));
        }
    }
}
