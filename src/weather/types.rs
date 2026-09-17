use crate::state::WeatherSnapshotMetadata;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Provider-neutral classification used by weather presentation. Provider
/// adapters own the conversion from their raw condition codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WeatherCondition {
    #[default]
    Unknown,
    Clear,
    PartlyCloudy,
    Cloudy,
    Drizzle,
    Rain,
    HeavyRain,
    Thunderstorm,
    Snow,
    Mist,
    Fog,
    Atmospheric,
}

impl WeatherCondition {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Condition unavailable",
            Self::Clear => "Clear",
            Self::PartlyCloudy => "Partly cloudy",
            Self::Cloudy => "Cloudy",
            Self::Drizzle => "Drizzle",
            Self::Rain => "Rain",
            Self::HeavyRain => "Heavy rain",
            Self::Thunderstorm => "Thunderstorm",
            Self::Snow => "Snow",
            Self::Mist => "Mist",
            Self::Fog => "Fog",
            Self::Atmospheric => "Atmospheric conditions",
        }
    }
}

/// The provider's day/night hint, if one was supplied with the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherDayPhase {
    Day,
    Night,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WeatherSnapshot {
    pub provider: String,
    /// When SunReactor received the provider response.
    pub fetched_at_epoch_s: u64,
    /// Timestamp for the forecast interval represented by this value.
    pub valid_at_epoch_s: u64,
    pub source_kind: WeatherSourceKind,
    pub cloud_cover_percent: u8,
    pub temperature: f32,
    pub condition: WeatherCondition,
    pub condition_description: Option<String>,
    pub day_phase: Option<WeatherDayPhase>,
    pub forecast: Vec<crate::state::ForecastPoint>,
    pub details: crate::state::WeatherDetails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WeatherSourceKind {
    #[default]
    Unknown,
    CurrentObservation,
    Forecast,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherResolution {
    pub modifier: Option<f64>,
    pub snapshot: Option<WeatherSnapshotMetadata>,
    pub next_refresh_at_epoch_s: Option<u64>,
    pub error: Option<WeatherError>,
    pub refresh_attempted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherSnapshotState {
    Missing,
    Stale,
    Incomplete,
    Ready,
}

/// The user-visible state of the weather service. It intentionally describes
/// the latest attempt without exposing credentials or transport internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WeatherState {
    #[default]
    Disabled,
    NoApiKey,
    Loading,
    Ready,
    Stale,
    Unauthorized,
    RateLimited,
    NetworkError,
    ProviderError,
    ParseError,
}

impl WeatherState {
    #[must_use]
    pub fn from_error(error: &WeatherError) -> Self {
        match error {
            WeatherError::MissingApiKey { .. } => Self::NoApiKey,
            WeatherError::HttpStatus {
                status: 401 | 403, ..
            } => Self::Unauthorized,
            WeatherError::HttpStatus { status: 429, .. } => Self::RateLimited,
            WeatherError::HttpStatus { .. }
            | WeatherError::MissingProvider
            | WeatherError::UnsupportedProvider { .. }
            | WeatherError::InvalidResponse { .. } => Self::ProviderError,
            WeatherError::Transport { .. } => Self::NetworkError,
            WeatherError::Parse { .. } => Self::ParseError,
        }
    }
}

/// Snapshot and diagnostics owned by the background weather service.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherServiceStatus {
    pub state: WeatherState,
    pub snapshot: Option<WeatherSnapshotMetadata>,
    pub last_attempted_at_epoch_s: Option<u64>,
    pub next_refresh_at_epoch_s: Option<u64>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

impl WeatherServiceStatus {
    #[must_use]
    pub fn disabled(snapshot: Option<WeatherSnapshotMetadata>) -> Self {
        Self {
            state: WeatherState::Disabled,
            snapshot,
            last_attempted_at_epoch_s: None,
            next_refresh_at_epoch_s: None,
            consecutive_failures: 0,
            last_error: None,
        }
    }

    #[must_use]
    pub fn loading(snapshot: Option<WeatherSnapshotMetadata>) -> Self {
        Self {
            state: WeatherState::Loading,
            snapshot,
            last_attempted_at_epoch_s: None,
            next_refresh_at_epoch_s: None,
            consecutive_failures: 0,
            last_error: None,
        }
    }
}

impl Default for WeatherServiceStatus {
    fn default() -> Self {
        Self::disabled(None)
    }
}

pub trait WeatherProvider {
    fn name(&self) -> &'static str;

    fn fetch_snapshot(&self, request: &WeatherRequest) -> Result<WeatherSnapshot, WeatherError>;
}
pub trait EnvironmentReader {
    fn get(&self, key: &str) -> Option<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessEnvironment;

impl EnvironmentReader for ProcessEnvironment {
    fn get(&self, key: &str) -> Option<String> {
        env::var(key).ok().filter(|value| !value.trim().is_empty())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub api_key: String,
    pub fetched_at_epoch_s: u64,
    pub timeout: Duration,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WeatherError {
    #[error("{}", format_missing_api_key(env_var))]
    MissingApiKey { env_var: Option<String> },
    #[error("weather is enabled but no provider is configured")]
    MissingProvider,
    #[error("unsupported weather provider `{provider}`")]
    UnsupportedProvider { provider: String },
    #[error("{provider} returned HTTP {status}")]
    HttpStatus { provider: &'static str, status: u16 },
    #[error("{provider} request failed: {message}")]
    Transport {
        provider: &'static str,
        message: String,
    },
    #[error("{provider} response parse failed: {message}")]
    Parse {
        provider: &'static str,
        message: String,
    },
    #[error("{provider} response was invalid: {message}")]
    InvalidResponse {
        provider: &'static str,
        message: String,
    },
}

fn format_missing_api_key(env_var: &Option<String>) -> String {
    match env_var {
        Some(var) => format!("weather is enabled but no API key is available; set {var} or configure weather.api_key explicitly"),
        None => "weather is enabled but no API key is available; configure weather.api_key_env or weather.api_key".to_owned(),
    }
}

impl WeatherError {
    pub(crate) fn is_transient(&self) -> bool {
        match self {
            Self::Transport { .. } => true,
            Self::HttpStatus { status, .. } => *status == 408 || *status == 429 || *status >= 500,
            Self::MissingApiKey { .. }
            | Self::MissingProvider
            | Self::UnsupportedProvider { .. }
            | Self::Parse { .. }
            | Self::InvalidResponse { .. } => false,
        }
    }
}
