use super::*;
#[cfg(target_os = "linux")]
use crate::backends::FailureKind;
use crate::backends::{testutil::FakeRunner, BackendKind};
use crate::config::{
    Config, ConfigError, ConfigReport, ConfigSource, DaemonConfig, LocationConfig, LogLevel,
    MonitorConfig, MonitorSelector, SolarPolicyConfig, WeatherConfig,
};
use crate::ipc::{Request, Response};
use crate::process::{CommandError, CommandOutput, ProcessRunner};
#[cfg(target_os = "linux")]
use crate::state::FailureBackoffState;
use crate::state::{ManualOverrideState, RuntimeState, WeatherSnapshotMetadata};
use chrono::{TimeZone, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod bootstrap;
mod fade_budget;
mod ipc_characterization;
mod ipc_mutations;
mod ipc_queries;
mod lifecycle;
mod persistence;
mod reload;
mod run_once;

struct RecordingRunner {
    calls: std::sync::Mutex<Vec<String>>,
}

#[derive(Clone)]
struct SlowObservationRunner {
    delay: Duration,
    started: Arc<AtomicBool>,
    discoveries: Arc<std::sync::atomic::AtomicUsize>,
}

impl ProcessRunner for SlowObservationRunner {
    fn run(
        &self,
        program: &str,
        args: &[String],
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        // Each discovery starts by probing ddcutil's supported options.
        // Count refreshes here instead of counting every child process.
        if program == "ddcutil" && args == [String::from("--help")] {
            self.discoveries.fetch_add(1, Ordering::Relaxed);
        }
        self.started.store(true, Ordering::Release);
        std::thread::sleep(self.delay);
        Err(CommandError::Missing {
            program: program.to_owned(),
        })
    }
}

struct PanicObservationRunner;

struct SlowApplyRunner;

impl ProcessRunner for SlowApplyRunner {
    fn run(
        &self,
        _program: &str,
        _args: &[String],
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        std::thread::sleep(Duration::from_secs(2));
        Ok(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}

impl ProcessRunner for PanicObservationRunner {
    fn run(
        &self,
        _program: &str,
        _args: &[String],
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        panic!("synthetic observation panic");
    }
}

impl RecordingRunner {
    fn new() -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    #[cfg(target_os = "linux")]
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl ProcessRunner for RecordingRunner {
    fn run(
        &self,
        program: &str,
        args: &[String],
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        let mut call = String::from(program);
        for arg in args {
            call.push('|');
            call.push_str(arg);
        }
        self.calls.lock().unwrap().push(call);
        Ok(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}

fn test_config_report() -> ConfigReport {
    ConfigReport {
        path: PathBuf::from("/tmp/sunreactor-config.toml"),
        source: ConfigSource::Defaults,
        config: Config {
            daemon: DaemonConfig {
                tick_seconds: 60,
                dry_run: false,
                smooth_transition: false,
                desktop_idle_sync: false,
                desktop_idle_timeout_minutes: 0,
                log_level: LogLevel::Info,
                apply_reassert_minutes: 2,
                external_change_hold_minutes: 30,
                probe_seconds: 15,
                ddc_timeout_seconds: 4,
                backlight_timeout_seconds: 2,
            },
            location: LocationConfig {
                city: String::new(),
                latitude: 41.0082,
                longitude: 28.9784,
                timezone: String::from("Europe/Istanbul"),
            },
            solar_policy: SolarPolicyConfig {
                use_adaptive_zenith: true,
                twilight_elevation_start: -6.0,
                day_elevation_full: 3.0,
                max_step_pct_per_tick: 6,
                min_write_delta_pct: 2,
            },
            monitors: vec![MonitorConfig {
                logical_id: String::from("internal"),
                backend: crate::backends::BackendKind::Backlight,
                enabled: true,
                allow_topology_retargeting: false,
                min_pct: 10,
                max_pct: 100,
                gain: 1.0,
                transition_gamma: 1.4,
                milestone_adjustments: Vec::new(),
                selector: MonitorSelector {
                    connector: None,
                    serial: None,
                    model: None,
                    edid: None,
                    sysfs_path: Some(String::from("/sys/class/backlight/intel_backlight")),
                    ddc_bus: None,
                    ddc_address: None,
                },
            }],
            weather: WeatherConfig::default(),
            tui: crate::config::TuiConfig::default(),
        },
        warnings: Vec::new(),
    }
}

fn test_weather_config_report() -> ConfigReport {
    let mut report = test_config_report();
    report.config.weather.enabled = true;
    report.config.weather.refresh_minutes = 30;
    report.config.weather.min_multiplier = 0.75;
    report
}

#[cfg(target_os = "linux")]
fn test_capability_snapshot() -> CapabilitySnapshot {
    CapabilitySnapshot {
        ddc_present: Vec::new(),
        backlights_present: vec![crate::discovery::BacklightDeviceDiscovery {
            stable_id: String::from("backlight:intel_backlight"),
            device_name: String::from("intel_backlight"),
            class: String::from("backlight"),
            max_brightness: Some(100),
            backlight_type: Some(String::from("raw")),
            probe_source: String::from("test"),
            sysfs_path: String::from("/sys/class/backlight/intel_backlight"),
            ddcci_connector: None,
            backend_viable: true,
            note: None,
        }],
    }
}

fn test_dry_run_config_report() -> ConfigReport {
    let mut report = test_config_report();
    report.config.daemon.dry_run = true;
    report
}

fn count_temp_state_files(state_path: &Path) -> usize {
    let Some(state_dir) = state_path.parent() else {
        return 0;
    };
    let prefix = format!(
        "{}.tmp-",
        state_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("runtime-state.json")
    );
    fs::read_dir(state_dir)
        .expect("state dir should be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .count()
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should work")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sunreactor-runtime-test-{unique}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}
