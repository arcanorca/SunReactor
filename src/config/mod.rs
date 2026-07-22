mod compat;
mod discovered;
mod error;
mod io;
mod model;
mod template;
mod validate;

pub use discovered::{apply_discovered_transaction, DiscoveredApplyError, DiscoveredApplyResult};
pub use error::{ConfigError, ConfigReport, ConfigSource, ValidationError};
pub use io::{
    load, parse_text, render, save, save_raw, save_raw_to_path, save_to_path, write_default,
};
pub use model::{
    Config, DaemonConfig, LocationConfig, LogLevel, MonitorConfig, MonitorMilestoneAdjustment,
    MonitorSelector, SolarPolicyConfig, TemperatureUnit, TuiConfig, WeatherConfig, WeatherProvider,
};
pub use template::DEFAULT_CONFIG_TEMPLATE;

pub fn validate(config: &Config) -> Result<(), ConfigError> {
    config.validate()
}

#[cfg(test)]
pub(crate) use io::{load_from_path, parse_str, write_default_to};

#[cfg(test)]
mod tests;
