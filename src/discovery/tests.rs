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
fn duplicate_metadata_gets_unique_report_occurrences() {
    let mut first = super::model::RawDdcMonitor::new(1);
    first.manufacturer = Some(String::from("DEL"));
    first.model = Some(String::from("U2720Q"));
    first.serial = Some(String::from("ABC123"));
    first.bus_number = Some(7);
    first.connector = Some(String::from("card1-DP-1"));
    let mut second = super::model::RawDdcMonitor::new(2);
    second.manufacturer = first.manufacturer.clone();
    second.model = first.model.clone();
    second.serial = first.serial.clone();
    second.bus_number = Some(8);
    second.connector = Some(String::from("card1-DP-2"));
    let mut monitors = vec![first.into_discovery(), second.into_discovery()];
    super::assign_ddc_occurrence_ids(&mut monitors);
    assert_ne!(monitors[0].occurrence_id, monitors[1].occurrence_id);
    assert_eq!(monitors[0].stable_id, monitors[1].stable_id);
}

#[test]
fn discovers_ddc_monitors_and_marks_vcp_brightness_support() {
    let runner = FakeRunner::new()
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
fn bus_only_generated_candidate_is_disabled_by_default() {
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            "Display 1\n  I2C bus:          /dev/i2c-7\n  DRM connector:   card1-DP-1\n  Monitor:          XMI::\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "capabilities"],
            "Feature: 10 (Brightness)\\n",
        )
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );

    let sysfs_root = TempSysfs::new(true);
    let report = discover_with_runner(&runner, sysfs_root.path());

    assert!(report.config_snippet.contains("ddc_bus = 7"));
    assert!(report.config_snippet.contains("enabled = false"));
    assert!(report
        .config_snippet
        .contains("allow_topology_retargeting = false"));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("topology slot")));
}

#[test]
fn mixed_internal_and_external_monitors_in_single_discovery_run() {
    // One discovery run sees both an external DDC monitor and an internal
    // backlight device; both must appear in the snapshot and the generated
    // config must contain both backend blocks.
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            "Display 1\n  I2C bus:          /dev/i2c-7\n  DRM connector:   card1-DP-1\n  Monitor:          XMI:Mi Monitor:\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "capabilities"],
            "Feature: 10 (Brightness)\n",
        )
        .with_success(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
            "intel_backlight,backlight,4712,50%,9375\n",
        );

    let sysfs_root = TempSysfs::new(true);
    write_backlight_device(sysfs_root.path(), "intel_backlight", Some(9375), true);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.summary.ddc_monitors, 1);
    assert_eq!(report.summary.backlight_devices, 1);
    assert_eq!(report.summary.viable_targets, 2);
    assert_eq!(report.backends.ddcutil.status, BackendStatusKind::Ok);
    assert_eq!(report.backends.brightnessctl.status, BackendStatusKind::Ok);
    assert_eq!(report.backends.sysfs.status, BackendStatusKind::Ok);
    assert_eq!(report.ddc_monitors[0].manufacturer.as_deref(), Some("XMI"));
    assert_eq!(report.backlight_devices[0].device_name, "intel_backlight");
    assert_eq!(report.backlight_devices[0].probe_source, "brightnessctl");
    assert!(report.config_snippet.contains("backend = \"ddc\""));
    assert!(report.config_snippet.contains("backend = \"backlight\""));
    assert!(report.config_snippet.contains("ddc_bus = 7"));
    assert!(report
        .config_snippet
        .contains("allow_topology_retargeting = false"));
    assert!(report.config_snippet.contains("enabled = true"));
    assert!(report.config_snippet.contains("model = \"Mi Monitor\""));
    assert!(report.config_snippet.contains("sysfs_path = \""));
}

#[cfg(unix)]
#[test]
fn ddc_primary_suppresses_authoritative_ddcci_alias_from_generated_config() {
    // The same physical external panel is visible through ddcutil and the
    // ddcci-driver-linux backlight. Only the viable DDC target may be
    // auto-generated; the proven ddcci backlight is an alias, not a second
    // enabled target.
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            "Display 1\n  I2C bus:          /dev/i2c-7\n  DRM connector:   card1-DP-1\n  Monitor:          DEL:U2720Q:ABC123\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "capabilities"],
            "Feature: 10 (Brightness)\n",
        )
        .with_success(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
            "ddcci_backlight_0,backlight,4712,50%,100\n",
        );

    let roots = tempfile::tempdir().expect("tempdir");
    let sysfs_root = roots.path().join("backlight");
    let drm_root = roots.path().join("drm");
    let connector = drm_root.join("card1-DP-1");
    fs::create_dir_all(&connector).expect("connector dir should exist");
    let hardware = roots.path().join("devices").join("panel0");
    fs::create_dir_all(&hardware).expect("hardware device should exist");
    write_backlight_device(&sysfs_root, "ddcci_backlight_0", Some(100), true);
    std::os::unix::fs::symlink(&hardware, connector.join("ddcci_backlight"))
        .expect("connector topology symlink should exist");
    std::os::unix::fs::symlink(
        &hardware,
        sysfs_root.join("ddcci_backlight_0").join("device"),
    )
    .expect("backlight topology symlink should exist");

    let report = super::discover_with_roots(&runner, &sysfs_root, &drm_root);

    assert_eq!(report.summary.ddc_monitors, 1);
    assert_eq!(report.summary.backlight_devices, 1);
    assert_eq!(report.summary.viable_targets, 1);
    assert_eq!(
        report.backlight_devices[0].ddcci_connector.as_deref(),
        Some("card1-DP-1")
    );
    let targets = report.viable_targets();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].backend, crate::backends::BackendKind::Ddc);

    let generated: toml::Value =
        toml::from_str(&report.config_snippet).expect("generated config should be valid TOML");
    let monitors = generated["monitors"]
        .as_array()
        .expect("generated config should contain monitor blocks");
    assert_eq!(
        monitors.len(),
        1,
        "one physical monitor must yield one target"
    );
    assert_eq!(monitors[0]["backend"].as_str(), Some("ddc"));
    assert_eq!(monitors[0]["enabled"].as_bool(), Some(true));
    assert!(
        monitors[0].get("sysfs_path").is_none(),
        "the authoritative ddcci alias must not become a second backlight block"
    );
}

#[test]
fn duplicate_effective_ddc_selectors_are_withheld_from_generated_config() {
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            "Display 1\n I2C bus: /dev/i2c-7\n DRM connector: card1-DP-1\n Monitor: DEL:U2720Q:ABC123\n\nDisplay 2\n I2C bus: /dev/i2c-8\n DRM connector: card1-DP-2\n Monitor: DEL:U2720Q:ABC123\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "1", "capabilities"],
            "Feature: 10 (Brightness)\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "2", "capabilities"],
            "Feature: 10 (Brightness)\n",
        )
        .with_success("brightnessctl", &["--list", "--machine-readable"], "");
    let report = discover_with_runner(&runner, Path::new("/nonexistent"));
    assert_eq!(report.ddc_monitors.len(), 2);
    assert!(
        report.config_snippet.starts_with("# No viable")
            || !report.config_snippet.contains("enabled = true")
    );
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("Withheld enabled DDC config")));
}

#[test]
fn backend_failure_isolation_keeps_sysfs_probe_when_brightnessctl_errors() {
    // Regression for backend isolation: when brightnessctl exists but exits
    // nonzero (malformed output, broken per-user access), the DDC/sysfs
    // probes must still run and produce usable viable counts — a failing
    // brightnessctl must not make every other discovery backend fail.
    let runner = FakeRunner::new()
        .with_missing("ddcutil", &["--noconfig", "--terse", "detect"])
        .with_error(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
            "brightnessctl: cannot determine backlight device\n",
        );

    let sysfs_root = TempSysfs::new(true);
    write_backlight_device(sysfs_root.path(), "intel_backlight", Some(9375), true);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(
        report.backends.brightnessctl.status,
        BackendStatusKind::Error
    );
    assert_eq!(report.backends.sysfs.status, BackendStatusKind::Ok);
    assert_eq!(report.summary.backlight_devices, 1);
    assert_eq!(report.summary.viable_targets, 1);
    assert_eq!(report.backlight_devices[0].device_name, "intel_backlight");
    assert_eq!(report.backlight_devices[0].probe_source, "sysfs");
    assert!(report.config_snippet.contains("backend = \"backlight\""));
    assert!(report.config_snippet.contains("sysfs_path = \""));
}

#[test]
fn ddc_detect_timeout_keeps_backlight_and_marks_ddc_unavailable() {
    // A wedged ddcutil detect must not drop internal-backlight discovery.
    let runner = FakeRunner::new()
        .with_timeout("ddcutil", &["--noconfig", "--terse", "detect"])
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );

    let sysfs_root = TempSysfs::new(true);
    write_backlight_device(sysfs_root.path(), "intel_backlight", Some(9375), true);

    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.backends.ddcutil.status, BackendStatusKind::Timeout);
    assert_eq!(report.summary.ddc_monitors, 0);
    assert_eq!(report.summary.backlight_devices, 1);
    assert_eq!(report.summary.viable_targets, 1);
    assert_eq!(report.backlight_devices[0].device_name, "intel_backlight");
}

#[test]
fn ddc_capabilities_timeout_keeps_other_ddc_monitor_viable() {
    // One display's capabilities probe times out; the other display with a
    // successful brightness VCP probe must remain independently viable and
    // present in the report. A single failed capability probe must not clear
    // the whole DDC observation.
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            "Display 1\n   I2C bus:          /dev/i2c-7\n   DRM connector:    card1-DP-1\n   Monitor:          XMI:Mi Monitor:\n\nDisplay 2\n   I2C bus:          /dev/i2c-9\n   DRM connector:    card1-DP-2\n   Monitor:          LEN:LEN P24h-20:V305PTDA\n",
        )
        .with_timeout("ddcutil", &["--noconfig", "--display", "1", "capabilities"])
        .with_success(
            "ddcutil",
            &["--noconfig", "--display", "2", "capabilities"],
            "Feature: 10 (Brightness)\n",
        )
        .with_missing(
            "brightnessctl",
            &["--list", "--machine-readable", "--class", "backlight"],
        );

    let sysfs_root = TempSysfs::new(true);
    let report = discover_with_runner(&runner, sysfs_root.path());

    assert_eq!(report.summary.ddc_monitors, 2);
    let viable = report
        .ddc_monitors
        .iter()
        .filter(|monitor| monitor.backend_viable)
        .collect::<Vec<_>>();
    assert_eq!(viable.len(), 1);
    assert_eq!(viable[0].display_number, 2);
    assert!(!report.ddc_monitors[0].backend_viable);
    assert!(report.ddc_monitors[0]
        .note
        .as_deref()
        .unwrap_or("")
        .contains("timed out"));
    assert_eq!(report.summary.viable_targets, 1);
}

#[test]
fn falls_back_to_sysfs_when_brightnessctl_is_missing() {
    let runner = FakeRunner::new()
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
    assert_eq!(
        report.backlight_devices[0].backlight_type.as_deref(),
        Some("raw")
    );
    assert_eq!(report.backends.sysfs.status, BackendStatusKind::Ok);
    assert_eq!(report.summary.backlight_devices, 1);
    assert_eq!(report.summary.viable_targets, 1);
    assert_eq!(report.backlight_devices[0].device_name, "intel_backlight");
    assert_eq!(report.backlight_devices[0].max_brightness, Some(9375));
    assert_eq!(
        report.backlight_devices[0].backlight_type.as_deref(),
        Some("raw")
    );
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

    fn with_error(mut self, program: &str, args: &[&str], stderr: &str) -> Self {
        self.responses.insert(
            command_key(program, args),
            Ok(CommandOutput {
                stdout: String::new(),
                stderr: stderr.to_owned(),
                exit_code: Some(1),
            }),
        );
        self
    }

    fn with_timeout(mut self, program: &str, args: &[&str]) -> Self {
        self.responses.insert(
            command_key(program, args),
            Err(CommandError::Timeout {
                program: program.to_owned(),
                after: Duration::from_secs(4),
                stdout: String::new(),
                stderr: String::new(),
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
    fs::write(device_dir.join("type"), "raw\n").expect("type should be writable");
    if brightness {
        fs::write(device_dir.join("brightness"), "1\n").expect("brightness should be writable");
    }
}
