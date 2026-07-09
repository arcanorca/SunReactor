use crate::state::WeatherSnapshotMetadata;
use std::env;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherSnapshot {
    pub provider: String,
    pub observed_at_epoch_s: u64,
    pub cloud_cover_percent: u8,
    pub temperature: f32,
    pub forecast: Vec<crate::state::ForecastPoint>,
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
