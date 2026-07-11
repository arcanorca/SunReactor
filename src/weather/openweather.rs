use serde::Deserialize;

use super::{WeatherError, WeatherProvider, WeatherRequest, WeatherSnapshot};
use crate::state::ForecastPoint;

const OPENWEATHER_ENDPOINT: &str = "https://api.openweathermap.org/data/2.5/forecast";

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

        parse_snapshot(&body, request.fetched_at_epoch_s)
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
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMain {
    temp: f32,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherClouds {
    all: u16,
}

fn request_url(request: &WeatherRequest) -> String {
    format!(
        "{OPENWEATHER_ENDPOINT}?lat={:.6}&lon={:.6}&appid={}&cnt=9&units=metric",
        request.latitude, request.longitude, request.api_key
    )
}

fn map_request_error(error: ureq::Error) -> WeatherError {
    match error {
        ureq::Error::Status(status, _) => WeatherError::HttpStatus {
            provider: "openweather",
            status,
        },
        ureq::Error::Transport(transport) => WeatherError::Transport {
            provider: "openweather",
            message: transport.to_string(),
        },
    }
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

    // The first item in the forecast list is usually the current/nearest 3-hour window
    let current = &parsed.list[0];

    // The rest form the forecast
    let forecast = parsed
        .list
        .iter()
        .skip(1)
        .map(|p| ForecastPoint {
            dt_epoch_s: p.dt,
            cloud_cover_percent: p.clouds.all.min(100) as u8,
            temperature: p.main.temp,
        })
        .collect();

    Ok(WeatherSnapshot {
        provider: String::from("openweather"),
        observed_at_epoch_s: fetched_at_epoch_s,
        cloud_cover_percent: current.clouds.all.min(100) as u8,
        temperature: current.main.temp,
        forecast,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_snapshot;

    #[test]
    fn parses_valid_openweather_forecast_response() {
        let json = r#"{"list":[{"dt":1700000000,"clouds":{"all":83},"main":{"temp":12.5}},{"dt":1700010800,"clouds":{"all":50},"main":{"temp":14.2}}]}"#;
        let snapshot =
            parse_snapshot(json, 1_800_000_000).expect("valid OpenWeather response should parse");

        assert_eq!(snapshot.provider, "openweather");
        assert_eq!(snapshot.observed_at_epoch_s, 1_800_000_000);
        assert_eq!(snapshot.cloud_cover_percent, 83);
        assert_eq!(snapshot.temperature, 12.5);
        assert_eq!(snapshot.forecast.len(), 1);
        assert_eq!(snapshot.forecast[0].dt_epoch_s, 1700010800);
        assert_eq!(snapshot.forecast[0].cloud_cover_percent, 50);
        assert_eq!(snapshot.forecast[0].temperature, 14.2);
    }
}
