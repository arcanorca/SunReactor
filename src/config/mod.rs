mod compat;
mod error;
mod io;
mod model;
mod template;
mod validate;

pub use error::{ConfigError, ConfigReport, ConfigSource, ValidationError};
pub use io::{
    load, parse_text, render, save, save_raw, save_raw_to_path, save_to_path, write_default,
};
pub use model::{
    Config, DaemonConfig, LocationConfig, LogLevel, MonitorConfig, MonitorMilestoneAdjustment,
    MonitorSelector, MotionLevel, SolarPolicyConfig, TemperatureUnit, Theme, TuiConfig,
    WeatherConfig, WeatherProvider,
};
pub use template::DEFAULT_CONFIG_TEMPLATE;
pub use validate::MAX_TRANSITION_GAMMA;

pub fn validate(config: &Config) -> Result<(), ConfigError> {
    config.validate()
}

#[cfg(any(test, feature = "tui"))]
pub(crate) use io::load_from_path;
#[cfg(test)]
pub(crate) use io::{parse_str, write_default_to};

#[cfg(test)]
mod tests;
