use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::{
    discover_with_runner, BackendStatusKind, CommandError, CommandOutput, DiscoveryReport,
    ProcessRunner,
};

#[test]
fn discovers_ddc_monitors_and_marks_vcp_brightness_support() {
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --terse")
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            "Display 1\n   I2C bus:          /dev/i2c-7\n   DRM connector:    card1-DP-1\n   Monitor:          XMI:Mi Monitor:\n\nDisplay 2\n   I2C bus:          /dev/i2c-9\n   DRM connector:    card1-DP-3\n   Monitor:          LEN:LEN P24h-20:V305PTDA\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "capabilities"],
            "Feature: 10 (Brightness)\nFeature: 12 (Contrast)\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "2", "capabilities"],
            "Feature: 12 (Contrast)\n",
        )
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );

    let sysfs_root = TempSysfs::new(true);
    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.backends.ddcutil.status, BackendStatusKind::Ok);
    assert_eq!(report.summary.ddc_monitors, 2);
    assert_eq!(report.summary.viable_targets, 1);
    assert_eq!(report.ddc_monitors[0].manufacturer.as_deref(), Some("XMI"));
    assert_eq!(report.ddc_monitors[0].model.as_deref(), Some("Mi Monitor"));
    assert_eq!(report.ddc_monitors[0].serial, None);
    assert_eq!(report.ddc_monitors[0].bus_number, Some(7));
    assert_eq!(report.ddc_monitors[0].brightness_vcp_supported, Some(true));
    assert_eq!(report.ddc_monitors[1].brightness_vcp_supported, Some(false));
    assert!(report.config_snippet.contains("backend = \"ddc\""));
    assert!(report.config_snippet.contains("ddc_bus = 7"));
    assert!(!report.config_snippet.contains("V305PTDA"));
}

#[test]
fn valid_msi_survives_invalid_boe_and_getvcp_fallback() {
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig")
        .with_success(
            "ddcutil",
            &["--noconfig", "detect"],
            include_str!("../../tests/fixtures/ddcutil/msi_then_invalid_boe.txt"),
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "capabilities"],
            "Feature: 12 (Contrast)",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "getvcp", "10"],
            "VCP code 0x10 (Brightness): current value = 35, max value = 100",
        )
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );
    let sysfs_root = TempSysfs::new(false);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.ddc_monitors.len(), 1);
    assert_eq!(report.ddc_monitors[0].manufacturer.as_deref(), Some("MSI"));
    assert_eq!(report.ddc_monitors[0].bus_number, Some(4));
    assert!(report.ddc_monitors[0].backend_viable);
    assert!(report.ddc_monitors[0]
        .note
        .as_deref()
        .is_some_and(|note| note.contains("recovered via getvcp")));
}

#[test]
fn successful_detect_with_no_displays_is_not_a_permission_error() {
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "")
        .with_success("ddcutil", &["detect"], "No displays found")
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );
    let sysfs_root = TempSysfs::new(false);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.backends.ddcutil.status, BackendStatusKind::Ok);
    assert!(report.ddc_monitors.is_empty());
    assert!(report
        .backends
        .ddcutil
        .message
        .contains("No external monitors"));
}

#[test]
fn falls_back_to_sysfs_when_brightnessctl_is_missing() {
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --terse")
        .with_missing("ddcutil", &["--noconfig", "--terse", "detect"])
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );

    let sysfs_root = TempSysfs::new(true);
    write_backlight_device(sysfs_root.path(), "intel_backlight", Some(9375), true);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(
        report.backends.brightnessctl.status,
        BackendStatusKind::Missing
    );
    assert_eq!(report.backends.sysfs.status, BackendStatusKind::Ok);
    assert_eq!(report.summary.backlight_devices, 1);
    assert_eq!(report.summary.viable_targets, 1);
    assert_eq!(report.backlight_devices[0].device_name, "intel_backlight");
    assert_eq!(report.backlight_devices[0].max_brightness, Some(9375));
    assert_eq!(report.backlight_devices[0].probe_source, "sysfs");
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("sysfs fallback")));
    assert!(report.config_snippet.contains("backend = \"backlight\""));
    assert!(report.config_snippet.contains("intel_backlight"));
}

#[test]
fn parses_brightnessctl_machine_readable_output() {
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --terse")
        .with_missing("ddcutil", &["--noconfig", "--terse", "detect"])
        .with_success(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
            "intel_backlight,backlight,4712,50%,9375\namdgpu_bl1,backlight,42,10%,255\n",
        );

    let sysfs_root = TempSysfs::new(true);
    write_backlight_device(sysfs_root.path(), "intel_backlight", Some(9375), true);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.backends.brightnessctl.status, BackendStatusKind::Ok);
    assert_eq!(report.summary.backlight_devices, 2);
    assert_eq!(report.summary.viable_targets, 2);
    assert_eq!(report.backlight_devices[0].device_name, "amdgpu_bl1");
    assert_eq!(report.backlight_devices[0].probe_source, "brightnessctl");
    assert_eq!(report.backlight_devices[0].max_brightness, Some(255));
    assert_eq!(report.backlight_devices[1].device_name, "intel_backlight");
    assert_eq!(report.backlight_devices[1].probe_source, "brightnessctl");
    assert!(report
        .config_snippet
        .contains("logical_id = \"amdgpu-bl1\""));
    assert!(report.config_snippet.contains("sysfs_path = \""));
}

#[test]
fn reports_clear_guidance_when_no_backends_are_available() {
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --terse")
        .with_missing("ddcutil", &["--noconfig", "--terse", "detect"])
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );

    let sysfs_root = TempSysfs::new(false);
    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.summary.viable_targets, 0);
    assert_eq!(report.backends.sysfs.status, BackendStatusKind::Unavailable);
    assert!(report
        .render_human()
        .contains("No brightness-capable devices were discovered."));

    let json = parse_json(&report);
    assert_eq!(json["backends"]["ddcutil"]["status"], "missing");
    assert_eq!(json["backends"]["brightnessctl"]["status"], "missing");
    assert_eq!(json["backends"]["sysfs"]["status"], "unavailable");
    assert_eq!(
        json["config_snippet"],
        "# No viable brightness-capable devices were discovered."
    );
}

fn parse_json(report: &DiscoveryReport) -> Value {
    serde_json::from_str(&report.render_json()).expect("report JSON should parse")
}

#[derive(Default)]
struct FakeRunner {
    responses: BTreeMap<String, Result<CommandOutput, CommandError>>,
}

impl FakeRunner {
    fn new() -> Self {
        Self::default()
    }

    fn with_success(mut self, program: &str, args: &[&str], stdout: &str) -> Self {
        self.responses.insert(
            command_key(program, args),
            Ok(CommandOutput {
                stdout: stdout.to_owned(),
                stderr: String::new(),
                exit_code: Some(0),
            }),
        );
        self
    }

    fn with_missing(mut self, program: &str, args: &[&str]) -> Self {
        self.responses.insert(
            command_key(program, args),
            Err(CommandError::Missing {
                program: program.to_owned(),
            }),
        );
        self
    }
}

impl ProcessRunner for FakeRunner {
    fn run(
        &self,
        program: &str,
        args: &[String],
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        self.responses
            .get(&command_key_owned(program, args))
            .cloned()
            .unwrap_or_else(|| {
                Err(CommandError::Io {
                    program: program.to_owned(),
                    message: format!("unexpected command: {}", command_key_owned(program, args)),
                })
            })
    }
}

fn command_key(program: &str, args: &[&str]) -> String {
    let mut key = String::from(program);
    for arg in args {
        key.push('|');
        key.push_str(arg);
    }
    key
}

fn command_key_owned(program: &str, args: &[String]) -> String {
    let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
    command_key(program, &borrowed)
}

struct TempSysfs {
    path: PathBuf,
}

impl TempSysfs {
    fn new(create_dir: bool) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should work")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sunreactor-discovery-test-{unique}"));
        if create_dir {
            fs::create_dir_all(&path).expect("temp sysfs dir should be created");
        }
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSysfs {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}

fn write_backlight_device(root: &Path, name: &str, max_brightness: Option<u32>, brightness: bool) {
    let device_dir = root.join(name);
    fs::create_dir_all(&device_dir).expect("device dir should exist");
    if let Some(value) = max_brightness {
        fs::write(device_dir.join("max_brightness"), format!("{value}\n"))
            .expect("max_brightness should be writable");
    }
    if brightness {
        fs::write(device_dir.join("brightness"), "1\n").expect("brightness should be writable");
    }
}
