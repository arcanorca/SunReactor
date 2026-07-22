use serde::{Deserialize, Serialize};

use crate::backends::BackendKind;
use crate::policy::AutomationMilestone;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub daemon: DaemonConfig,
    pub location: LocationConfig,
    #[serde(rename = "solar_policy")]
    pub solar_policy: SolarPolicyConfig,
    pub monitors: Vec<MonitorConfig>,
    pub weather: WeatherConfig,
    pub tui: TuiConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DaemonConfig {
    pub tick_seconds: u64,
    pub dry_run: bool,
    /// Enable multi-step brightness fades. Disabled by default because rapidly
    /// issuing DDC/CI writes can destabilize some external displays.
    pub smooth_transition: bool,
    pub desktop_idle_sync: bool,
    pub desktop_idle_timeout_minutes: u64,
    pub log_level: LogLevel,
    pub apply_reassert_minutes: u64,
    pub ddc_timeout_seconds: u64,
    pub backlight_timeout_seconds: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            tick_seconds: 60,
            dry_run: false,
            smooth_transition: false,
            desktop_idle_sync: true,
            desktop_idle_timeout_minutes: 0,
            log_level: LogLevel::Info,
            apply_reassert_minutes: 2,
            ddc_timeout_seconds: 4,
            backlight_timeout_seconds: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LocationConfig {
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

impl Default for LocationConfig {
    fn default() -> Self {
        Self {
            city: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            timezone: String::from("UTC"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SolarPolicyConfig {
    pub twilight_elevation_start: f64,
    pub day_elevation_full: f64,
    pub use_adaptive_zenith: bool,

    pub max_step_pct_per_tick: u8,
    pub min_write_delta_pct: u8,
}

impl Default for SolarPolicyConfig {
    fn default() -> Self {
        Self {
            twilight_elevation_start: -6.0,
            day_elevation_full: 20.0,
            use_adaptive_zenith: true,

            max_step_pct_per_tick: 6,
            min_write_delta_pct: 1,
        }
    }
}

fn default_transition_gamma() -> f64 {
    0.5
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MonitorConfig {
    pub logical_id: String,
    pub backend: BackendKind,
    pub enabled: bool,
    pub min_pct: u8,
    pub max_pct: u8,
    pub gain: f64,
    #[serde(default = "default_transition_gamma")]
    pub transition_gamma: f64,
    pub milestone_adjustments: Vec<MonitorMilestoneAdjustment>,
    #[serde(flatten)]
    pub selector: MonitorSelector,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            logical_id: String::new(),
            backend: BackendKind::Backlight,
            enabled: true,
            min_pct: 15,
            max_pct: 60,
            gain: 1.0,
            transition_gamma: 0.5,
            milestone_adjustments: Vec::new(),
            selector: MonitorSelector::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MonitorMilestoneAdjustment {
    pub milestone: AutomationMilestone,
    pub minutes_offset: i16,
}

impl Default for MonitorMilestoneAdjustment {
    fn default() -> Self {
        Self {
            milestone: AutomationMilestone::RiseStart,
            minutes_offset: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MonitorSelector {
    pub connector: Option<String>,
    pub serial: Option<String>,
    pub model: Option<String>,
    pub edid: Option<String>,
    pub sysfs_path: Option<String>,
    pub ddc_bus: Option<u8>,
    pub ddc_address: Option<u16>,
}

impl MonitorSelector {
    pub(super) fn has_any(&self) -> bool {
        selector_text_present(&self.connector)
            || selector_text_present(&self.serial)
            || selector_text_present(&self.model)
            || selector_text_present(&self.edid)
            || selector_text_present(&self.sysfs_path)
            || self.ddc_bus.is_some()
            || self.ddc_address.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WeatherConfig {
    pub enabled: bool,
    pub provider: Option<WeatherProvider>,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub refresh_minutes: u32,
    pub min_multiplier: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TuiConfig {
    pub fps: u32,
    pub use_12h_time: bool,
    pub temperature_unit: TemperatureUnit,
    pub theme: crate::tui::theme::Theme,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            fps: 1,
            use_12h_time: false,
            temperature_unit: TemperatureUnit::Celsius,
            theme: crate::tui::theme::Theme::default(),
        }
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: Some(WeatherProvider::OpenWeather),
            api_key_env: Some(String::from("OPENWEATHER_API_KEY")),
            api_key: None,
            refresh_minutes: 30,
            min_multiplier: 0.75,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherProvider {
    OpenWeather,
}

fn selector_text_present(value: &Option<String>) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .is_some_and(|entry| !entry.is_empty())
}
