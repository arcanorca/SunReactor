use std::path::PathBuf;

use crate::apply::ApplySettings;
use crate::config::{ConfigReport, ConfigSource, MonitorConfig, WeatherConfig};
use crate::runtime::orchestrator::RuntimeError;
use crate::solar::Location;

#[derive(Debug, Clone)]
pub(super) struct RuntimeConfig {
    pub path: PathBuf,
    pub source: ConfigSource,
    pub daemon: RuntimeDaemonConfig,
    pub solar: crate::config::SolarPolicyConfig,
    pub apply: ApplySettings,
    pub location: Location,
    pub monitors: Vec<MonitorConfig>,
    pub weather: WeatherConfig,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RuntimeDaemonConfig {
    pub tick_seconds: u64,
    pub dry_run: bool,
    pub desktop_idle_sync: bool,
    pub desktop_idle_timeout_minutes: u64,
    pub apply_reassert_minutes: u64,
    pub ddc_timeout_seconds: u64,
    pub backlight_timeout_seconds: u64,
}

impl RuntimeConfig {
    pub(super) fn from_report(report: ConfigReport) -> Result<Self, RuntimeError> {
        let location = Location::from_timezone_name(
            report.config.location.latitude,
            report.config.location.longitude,
            &report.config.location.timezone,
        )?;
        let apply = ApplySettings::from_config(&report.config);
        Ok(Self {
            path: report.path,
            source: report.source,
            daemon: RuntimeDaemonConfig {
                tick_seconds: report.config.daemon.tick_seconds,
                dry_run: report.config.daemon.dry_run,
                desktop_idle_sync: report.config.daemon.desktop_idle_sync,
                desktop_idle_timeout_minutes: report.config.daemon.desktop_idle_timeout_minutes,
                apply_reassert_minutes: report.config.daemon.apply_reassert_minutes,
                ddc_timeout_seconds: report.config.daemon.ddc_timeout_seconds,
                backlight_timeout_seconds: report.config.daemon.backlight_timeout_seconds,
            },
            solar: report.config.solar_policy,
            apply,
            monitors: report.config.monitors,
            weather: report.config.weather,
            location,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LocationConfig, MonitorSelector, SolarPolicyConfig};

    #[test]
    fn converts_raw_config_into_resolved_runtime_config() {
        let config = Config {
            location: LocationConfig {
                city: String::new(),
                latitude: 41.0082,
                longitude: 28.9784,
                timezone: String::from("Europe/Istanbul"),
            },
            solar_policy: SolarPolicyConfig {
                use_adaptive_zenith: true,
                ..SolarPolicyConfig::default()
            },
            monitors: vec![
                MonitorConfig {
                    logical_id: String::from("desk"),
                    min_pct: 12,
                    max_pct: 38,
                    selector: MonitorSelector {
                        sysfs_path: Some(String::from("/sys/class/backlight/mock")),
                        ..MonitorSelector::default()
                    },
                    ..MonitorConfig::default()
                },
                MonitorConfig {
                    logical_id: String::from("disabled"),
                    enabled: false,
                    ..MonitorConfig::default()
                },
            ],
            ..Config::default()
        };
        config.validate().expect("test config should validate");

        let runtime = RuntimeConfig::from_report(ConfigReport {
            path: PathBuf::from("/tmp/sunreactor-config.toml"),
            source: ConfigSource::FilePresent,
            config,
            warnings: Vec::new(),
        })
        .expect("runtime config should resolve");

        assert_eq!(runtime.location.timezone_name, "Europe/Istanbul");
        assert_eq!(runtime.monitors.len(), 2);
    }
}
