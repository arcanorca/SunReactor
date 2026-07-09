use super::smoothing::refresh_interval;
use super::types::{EnvironmentReader, WeatherError, WeatherRequest};
use crate::config::{WeatherConfig, WeatherProvider as ConfigWeatherProvider};
use crate::solar::Location;
use std::time::Duration;

const WEATHER_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const WEATHER_FAILURE_RETRY_BASE_SECONDS: u64 = 60;
const WEATHER_FAILURE_RETRY_MAX_SECONDS: u64 = 300;

pub(crate) fn next_refresh_delay(
    config: &WeatherConfig,
    error: Option<&WeatherError>,
    consecutive_refresh_failures: u32,
) -> Duration {
    let refresh = refresh_interval(config);
    let Some(error) = error else {
        return refresh;
    };

    if !error.is_transient() {
        return refresh;
    }

    let shift = consecutive_refresh_failures.saturating_sub(1).min(16);
    let multiplier = 1u64 << shift;
    Duration::from_secs(
        WEATHER_FAILURE_RETRY_BASE_SECONDS
            .saturating_mul(multiplier)
            .min(WEATHER_FAILURE_RETRY_MAX_SECONDS),
    )
    .min(refresh)
}
pub(crate) fn provider_request<E>(
    config: &WeatherConfig,
    location: &Location,
    now_epoch_s: u64,
    environment: &E,
) -> Result<WeatherRequest, WeatherError>
where
    E: EnvironmentReader,
{
    let provider = configured_provider(config)?;
    if provider != ConfigWeatherProvider::OpenWeather {
        return Err(WeatherError::UnsupportedProvider {
            provider: format!("{provider:?}").to_lowercase(),
        });
    }

    let api_key = resolve_api_key(config, environment)?;

    Ok(WeatherRequest {
        latitude: location.latitude,
        longitude: location.longitude,
        api_key,
        fetched_at_epoch_s: now_epoch_s,
        timeout: WEATHER_REQUEST_TIMEOUT,
    })
}
pub(crate) fn configured_provider(
    config: &WeatherConfig,
) -> Result<ConfigWeatherProvider, WeatherError> {
    config.provider.ok_or(WeatherError::MissingProvider)
}
pub(crate) fn resolve_api_key<E>(
    config: &WeatherConfig,
    environment: &E,
) -> Result<String, WeatherError>
where
    E: EnvironmentReader,
{
    if let Some(env_var) = config
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|env_var| !env_var.is_empty())
    {
        if let Some(value) = environment.get(env_var) {
            return Ok(value);
        }
    }

    if let Some(api_key) = config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|api_key| !api_key.is_empty())
    {
        return Ok(api_key.to_owned());
    }

    Err(WeatherError::MissingApiKey {
        env_var: config
            .api_key_env
            .as_deref()
            .map(str::trim)
            .filter(|env_var| !env_var.is_empty())
            .map(str::to_owned),
    })
}
