use serde::Deserialize;

use super::{
    WeatherCondition, WeatherDayPhase, WeatherError, WeatherProvider, WeatherRequest,
    WeatherSnapshot, WeatherSourceKind,
};
use crate::state::{AirQuality, ForecastPoint, WeatherDetails};

const OPENWEATHER_ENDPOINT: &str = "https://api.openweathermap.org/data/2.5/forecast";
const OPENWEATHER_AIR_POLLUTION_ENDPOINT: &str =
    "https://api.openweathermap.org/data/2.5/air_pollution";

#[derive(Debug, Clone, Default)]
pub struct OpenWeatherProvider;

impl WeatherProvider for OpenWeatherProvider {
    fn name(&self) -> &'static str {
        "openweather"
    }

    fn fetch_snapshot(&self, request: &WeatherRequest) -> Result<WeatherSnapshot, WeatherError> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(request.timeout)
            .timeout_read(request.timeout)
            .timeout_write(request.timeout)
            .build();
        let url = request_url(request);
        let response = agent.get(&url).call().map_err(map_request_error)?;
        let body = response
            .into_string()
            .map_err(|_| WeatherError::Transport {
                provider: self.name(),
                message: String::from("failed to read HTTPS response body"),
            })?;

        let mut snapshot = parse_snapshot(&body, request.fetched_at_epoch_s)?;
        // Air quality is supplementary: any failure leaves it empty and never
        // fails the weather refresh that drives brightness.
        snapshot.details.air_quality = agent
            .get(&air_pollution_url(request))
            .call()
            .ok()
            .and_then(|response| response.into_string().ok())
            .and_then(|body| parse_air_quality(&body));
        Ok(snapshot)
    }
}

#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    list: Vec<OpenWeatherListElement>,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherListElement {
    dt: u64,
    clouds: OpenWeatherClouds,
    main: OpenWeatherMain,
    #[serde(default)]
    weather: Vec<OpenWeatherCondition>,
    #[serde(default)]
    wind: Option<OpenWeatherWind>,
    #[serde(default)]
    visibility: Option<u32>,
    /// Probability of precipitation, 0–1.
    #[serde(default)]
    pop: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMain {
    temp: f32,
    #[serde(default)]
    feels_like: Option<f32>,
    #[serde(default)]
    humidity: Option<u8>,
    #[serde(default)]
    pressure: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherWind {
    #[serde(default)]
    speed: Option<f32>,
    #[serde(default)]
    deg: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct AirPollutionResponse {
    list: Vec<AirPollutionElement>,
}

#[derive(Debug, Deserialize)]
struct AirPollutionElement {
    components: AirPollutionComponents,
}

#[derive(Debug, Deserialize)]
struct AirPollutionComponents {
    pm2_5: f32,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherClouds {
    all: u16,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherCondition {
    id: u16,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: String,
}

fn request_url(request: &WeatherRequest) -> String {
    format!(
        "{OPENWEATHER_ENDPOINT}?lat={:.6}&lon={:.6}&appid={}&cnt=9&units=metric",
        request.latitude, request.longitude, request.api_key
    )
}

fn air_pollution_url(request: &WeatherRequest) -> String {
    format!(
        "{OPENWEATHER_AIR_POLLUTION_ENDPOINT}?lat={:.6}&lon={:.6}&appid={}",
        request.latitude, request.longitude, request.api_key
    )
}

/// Air quality from an Air Pollution API response, using the US EPA index
/// for PM2.5 (the pollutant OpenWeather always reports).
pub(crate) fn parse_air_quality(body: &str) -> Option<AirQuality> {
    let parsed: AirPollutionResponse = serde_json::from_str(body).ok()?;
    let pm2_5 = parsed.list.first()?.components.pm2_5;
    Some(AirQuality {
        us_aqi: us_aqi_from_pm2_5(pm2_5)?,
        pm2_5,
    })
}

/// US EPA AQI for a PM2.5 concentration in µg/m³ (2024 breakpoints),
/// truncated to one decimal as the EPA specifies.
#[must_use]
pub(crate) fn us_aqi_from_pm2_5(concentration: f32) -> Option<u16> {
    const BREAKPOINTS: [(f32, f32, u16, u16); 6] = [
        (0.0, 9.0, 0, 50),
        (9.1, 35.4, 51, 100),
        (35.5, 55.4, 101, 150),
        (55.5, 125.4, 151, 200),
        (125.5, 225.4, 201, 300),
        (225.5, 325.4, 301, 500),
    ];
    if !concentration.is_finite() || concentration < 0.0 {
        return None;
    }
    let truncated = (concentration * 10.0).floor() / 10.0;
    let (low, high, index_low, index_high) = BREAKPOINTS
        .iter()
        .copied()
        .find(|(_, high, _, _)| truncated <= *high)
        .unwrap_or(BREAKPOINTS[BREAKPOINTS.len() - 1]);
    let fraction = ((truncated.min(high) - low) / (high - low)).clamp(0.0, 1.0);
    Some(index_low + (f32::from(index_high - index_low) * fraction).round() as u16)
}

fn map_request_error(error: ureq::Error) -> WeatherError {
    match error {
        ureq::Error::Status(status, _) => WeatherError::HttpStatus {
            provider: "openweather",
            status,
        },
        ureq::Error::Transport(transport) => WeatherError::Transport {
            provider: "openweather",
            // `ureq::Transport::Display` may include the request URL. The URL
            // contains OpenWeather's API key, so retain only its safe error
            // class for logs and user-facing diagnostics.
            message: transport_error_message(transport.kind()),
        },
    }
}

fn transport_error_message(kind: ureq::ErrorKind) -> String {
    format!("network request failed ({kind})")
}

pub(crate) fn parse_snapshot(
    body: &str,
    fetched_at_epoch_s: u64,
) -> Result<WeatherSnapshot, WeatherError> {
    let parsed: OpenWeatherResponse =
        serde_json::from_str(body).map_err(|source| WeatherError::Parse {
            provider: "openweather",
            message: source.to_string(),
        })?;

    if parsed.list.is_empty() {
        return Err(WeatherError::InvalidResponse {
            provider: "openweather",
            message: String::from("forecast list is empty"),
        });
    }

    // The first item is a forecast interval, not a current observation.
    let current = &parsed.list[0];
    let current_condition = current.weather.first();
    let (condition, condition_description, day_phase) = current_condition.map_or_else(
        || (condition_from_cloud_cover(current.clouds.all), None, None),
        |weather| {
            (
                normalize_openweather_condition(weather.id),
                normalize_description(&weather.description),
                day_phase_from_icon(&weather.icon),
            )
        },
    );

    // The rest form the forecast
    let forecast = parsed
        .list
        .iter()
        .skip(1)
        .map(|p| ForecastPoint {
            dt_epoch_s: p.dt,
            cloud_cover_percent: p.clouds.all.min(100) as u8,
            temperature: p.main.temp,
            condition: p.weather.first().map_or_else(
                || condition_from_cloud_cover(p.clouds.all),
                |weather| normalize_openweather_condition(weather.id),
            ),
            day_phase: p
                .weather
                .first()
                .and_then(|weather| day_phase_from_icon(&weather.icon)),
            precipitation_percent: precipitation_percent(p.pop),
        })
        .collect();

    Ok(WeatherSnapshot {
        provider: String::from("openweather"),
        fetched_at_epoch_s,
        valid_at_epoch_s: current.dt,
        source_kind: WeatherSourceKind::Forecast,
        cloud_cover_percent: current.clouds.all.min(100) as u8,
        temperature: current.main.temp,
        condition,
        condition_description,
        day_phase,
        forecast,
        details: WeatherDetails {
            feels_like: current.main.feels_like.filter(|value| value.is_finite()),
            humidity_percent: current.main.humidity.map(|value| value.min(100)),
            pressure_hpa: current.main.pressure,
            wind_speed_mps: current
                .wind
                .as_ref()
                .and_then(|wind| wind.speed)
                .filter(|speed| speed.is_finite() && *speed >= 0.0),
            wind_direction_deg: current
                .wind
                .as_ref()
                .and_then(|wind| wind.deg)
                .map(|deg| deg % 360),
            visibility_m: current.visibility,
            precipitation_percent: precipitation_percent(current.pop),
            air_quality: None,
        },
    })
}

fn precipitation_percent(pop: Option<f32>) -> Option<u8> {
    pop.filter(|value| value.is_finite())
        .map(|value| (value.clamp(0.0, 1.0) * 100.0).round() as u8)
}

/// Converts OpenWeather's documented condition identifiers into the small,
/// provider-neutral vocabulary used by state and presentation.
#[must_use]
pub(crate) const fn normalize_openweather_condition(code: u16) -> WeatherCondition {
    match code {
        200..=232 => WeatherCondition::Thunderstorm,
        300..=321 => WeatherCondition::Drizzle,
        500 | 501 | 511 | 520 | 521 | 531 => WeatherCondition::Rain,
        502..=504 | 522 => WeatherCondition::HeavyRain,
        600..=622 => WeatherCondition::Snow,
        701 | 711 | 721 => WeatherCondition::Mist,
        741 => WeatherCondition::Fog,
        731 | 751 | 761 | 762 | 771 | 781 => WeatherCondition::Atmospheric,
        800 => WeatherCondition::Clear,
        801 => WeatherCondition::PartlyCloudy,
        802..=804 => WeatherCondition::Cloudy,
        _ => WeatherCondition::Unknown,
    }
}

const fn condition_from_cloud_cover(cloud_cover_percent: u16) -> WeatherCondition {
    match cloud_cover_percent {
        0..=19 => WeatherCondition::Clear,
        20..=59 => WeatherCondition::PartlyCloudy,
        _ => WeatherCondition::Cloudy,
    }
}

fn day_phase_from_icon(icon: &str) -> Option<WeatherDayPhase> {
    match icon.trim().as_bytes().last().copied() {
        Some(b'd') => Some(WeatherDayPhase::Day),
        Some(b'n') => Some(WeatherDayPhase::Night),
        _ => None,
    }
}

fn normalize_description(description: &str) -> Option<String> {
    let normalized: String = description
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect();
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_openweather_condition, parse_snapshot, transport_error_message, WeatherCondition,
        WeatherDayPhase, WeatherSourceKind,
    };

    #[test]
    fn parses_valid_openweather_forecast_response() {
        let json = r#"{"list":[{"dt":1700000000,"clouds":{"all":83},"main":{"temp":12.5}},{"dt":1700010800,"clouds":{"all":50},"main":{"temp":14.2}}]}"#;
        let snapshot =
            parse_snapshot(json, 1_800_000_000).expect("valid OpenWeather response should parse");

        assert_eq!(snapshot.provider, "openweather");
        assert_eq!(snapshot.fetched_at_epoch_s, 1_800_000_000);
        assert_eq!(snapshot.valid_at_epoch_s, 1_700_000_000);
        assert_eq!(snapshot.source_kind, WeatherSourceKind::Forecast);
        assert_eq!(snapshot.cloud_cover_percent, 83);
        assert!((snapshot.temperature - 12.5).abs() < f32::EPSILON);
        assert_eq!(snapshot.condition, WeatherCondition::Cloudy);
        assert_eq!(snapshot.condition_description, None);
        assert_eq!(snapshot.day_phase, None);
        assert_eq!(snapshot.forecast.len(), 1);
        assert_eq!(snapshot.forecast[0].dt_epoch_s, 1_700_010_800);
        assert_eq!(snapshot.forecast[0].cloud_cover_percent, 50);
        assert!((snapshot.forecast[0].temperature - 14.2).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_secondary_readings_and_per_interval_conditions() {
        let json = r#"{"list":[
            {"dt":1700000000,"clouds":{"all":90},"main":{"temp":18.8,"feels_like":18.2,"humidity":72,"pressure":1018},
             "wind":{"speed":3.4,"deg":292},"visibility":10000,"pop":0.09,
             "weather":[{"id":804,"description":"overcast clouds","icon":"04n"}]},
            {"dt":1700010800,"clouds":{"all":20},"main":{"temp":16.0},"pop":0.25,
             "weather":[{"id":500,"description":"light rain","icon":"10d"}]}
        ]}"#;
        let snapshot = parse_snapshot(json, 1_700_000_100).expect("valid forecast");
        let details = snapshot.details;
        assert_eq!(details.feels_like, Some(18.2));
        assert_eq!(details.humidity_percent, Some(72));
        assert_eq!(details.pressure_hpa, Some(1018));
        assert_eq!(details.wind_speed_mps, Some(3.4));
        assert_eq!(details.wind_direction_deg, Some(292));
        assert_eq!(details.visibility_m, Some(10_000));
        assert_eq!(details.precipitation_percent, Some(9));
        let next = &snapshot.forecast[0];
        assert_eq!(next.condition, WeatherCondition::Rain);
        assert_eq!(next.day_phase, Some(WeatherDayPhase::Day));
        assert_eq!(next.precipitation_percent, Some(25));

        // Older payloads without the extra fields still parse.
        let minimal = r#"{"list":[{"dt":1,"clouds":{"all":10},"main":{"temp":1.0}},{"dt":2,"clouds":{"all":70},"main":{"temp":2.0}}]}"#;
        let snapshot = parse_snapshot(minimal, 3).expect("minimal forecast");
        assert_eq!(snapshot.details, crate::state::WeatherDetails::default());
        assert_eq!(snapshot.forecast[0].condition, WeatherCondition::Cloudy);
        assert_eq!(snapshot.forecast[0].precipitation_percent, None);
    }

    #[test]
    fn air_quality_uses_the_us_epa_pm2_5_index() {
        assert_eq!(super::us_aqi_from_pm2_5(0.0), Some(0));
        assert_eq!(super::us_aqi_from_pm2_5(9.0), Some(50));
        assert_eq!(super::us_aqi_from_pm2_5(5.0), Some(28));
        assert_eq!(super::us_aqi_from_pm2_5(35.4), Some(100));
        assert_eq!(super::us_aqi_from_pm2_5(55.4), Some(150));
        assert_eq!(super::us_aqi_from_pm2_5(-1.0), None);
        let body = r#"{"coord":{"lon":28.9,"lat":41.0},"list":[{"main":{"aqi":1},"components":{"co":201.9,"pm2_5":5.04,"pm10":7.1},"dt":1700000000}]}"#;
        let air = super::parse_air_quality(body).expect("air quality");
        assert_eq!(air.us_aqi, 28);
        assert!(super::parse_air_quality(r#"{"list":[]}"#).is_none());
        assert!(super::parse_air_quality("not json").is_none());
    }

    #[test]
    fn parses_provider_condition_description_and_day_night_hint() {
        let json = r#"{
          "list": [
            {
              "dt": 1700000000,
              "clouds": {"all": 95},
              "main": {"temp": 12.5},
              "weather": [{"id": 502, "description": "heavy\n rain", "icon": "10n"}]
            }
          ]
        }"#;

        let snapshot =
            parse_snapshot(json, 1_800_000_000).expect("valid OpenWeather response should parse");

        assert_eq!(snapshot.condition, WeatherCondition::HeavyRain);
        assert_eq!(
            snapshot.condition_description.as_deref(),
            Some("heavy rain")
        );
        assert_eq!(snapshot.day_phase, Some(WeatherDayPhase::Night));
    }

    #[test]
    fn normalizes_openweather_condition_codes_once_at_the_provider_boundary() {
        let cases = [
            (200, WeatherCondition::Thunderstorm),
            (301, WeatherCondition::Drizzle),
            (500, WeatherCondition::Rain),
            (502, WeatherCondition::HeavyRain),
            (601, WeatherCondition::Snow),
            (701, WeatherCondition::Mist),
            (741, WeatherCondition::Fog),
            (761, WeatherCondition::Atmospheric),
            (800, WeatherCondition::Clear),
            (801, WeatherCondition::PartlyCloudy),
            (803, WeatherCondition::Cloudy),
            (999, WeatherCondition::Unknown),
        ];

        for (code, expected) in cases {
            assert_eq!(normalize_openweather_condition(code), expected);
        }
    }

    #[test]
    fn rejects_missing_forecast_timestamp_without_inventing_weather_time() {
        let json = r#"{"list":[{"clouds":{"all":83},"main":{"temp":12.5}}]}"#;
        assert!(parse_snapshot(json, 1_800_000_000).is_err());
    }

    #[test]
    fn transport_diagnostics_never_include_request_credentials() {
        let message = transport_error_message(ureq::ErrorKind::ConnectionFailed);
        assert_eq!(message, "network request failed (Connection Failed)");
        assert!(!message.contains("appid"));
    }
}
