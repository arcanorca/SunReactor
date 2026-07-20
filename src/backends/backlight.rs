use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{MonitorConfig, MonitorSelector};

use super::{
    clamp_percent, command_failure, map_command_error, BackendError, BackendKind, BackendWrite,
    ProcessRunner, RealProcessRunner,
};

const SYSFS_BACKLIGHT_ROOT: &str = "/sys/class/backlight";

pub fn apply(monitor: &MonitorConfig, percent: u8) -> Result<BackendWrite, BackendError> {
    match apply_with_runner(&RealProcessRunner, monitor, percent, Duration::from_secs(2)) {
        Err(BackendError::MissingProgram { .. }) => apply_sysfs_fallback(monitor, percent),
        other => other,
    }
}

pub(crate) fn apply_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
) -> Result<BackendWrite, BackendError> {
    let percent = clamp_percent(percent);
    let device_name = resolve_device_name(&monitor.selector)?;
    let args = vec![
        String::from("--quiet"),
        String::from("--class"),
        String::from("backlight"),
        String::from("--device"),
        device_name.clone(),
        String::from("set"),
        format!("{percent}%"),
    ];

    match runner.run("brightnessctl", &args, timeout) {
        Ok(output) if output.success() => Ok(BackendWrite {
            backend: BackendKind::Backlight,
            applied_percent: percent,
            attempts: 1,
            detail: format!("applied via brightnessctl device `{device_name}`"),
        }),
        Ok(output) => Err(command_failure(
            BackendKind::Backlight,
            "brightnessctl",
            &output,
        )),
        Err(error) => Err(map_command_error(BackendKind::Backlight, error)),
    }
}

/// Resolves the backlight device name from the monitor selector.
///
/// # Distro-agnostic resolution
///
/// The selector's `sysfs_path` is the preferred and explicit mechanism.
/// When it is absent, this function **dynamically scans** `/sys/class/backlight/`
/// and picks the first available device.  This makes sunreactor work on:
///
/// - Arch / Manjaro  (`amdgpu_bl0`, `amdgpu_bl1`, `nvidia_0`)
/// - Debian / Ubuntu (`intel_backlight`, `acpi_video0`)
/// - Alpine Linux    (`acpi_video0`, or vendor-specific entries)
/// - ARM SBCs        (`backlight`, `pwm-backlight`)
///
/// The function **never assumes** a specific device name.
fn resolve_device_name(selector: &MonitorSelector) -> Result<String, BackendError> {
    if selector.ddc_bus.is_some() || selector.ddc_address.is_some() {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "ddc_bus/ddc_address",
            message: String::from("DDC selectors do not apply to backlight devices"),
        });
    }

    if normalized(&selector.connector).is_some()
        || normalized(&selector.serial).is_some()
        || normalized(&selector.model).is_some()
        || normalized(&selector.edid).is_some()
    {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "connector/serial/model/edid",
            message: String::from("use sysfs_path to target a specific backlight device"),
        });
    }

    if let Some(raw_path) = normalized(&selector.sysfs_path) {
        // Explicit sysfs_path configured — validate and extract device name.
        return resolve_explicit_sysfs_path(&raw_path);
    }

    // No sysfs_path configured: scan the backlight class directory and pick
    // the first device found.  Sorted for determinism across distros.
    scan_first_backlight_device(SYSFS_BACKLIGHT_ROOT)
}

/// Validates an explicit `sysfs_path` and extracts the device name component.
fn resolve_explicit_sysfs_path(raw_path: &str) -> Result<String, BackendError> {
    let path = PathBuf::from(raw_path);
    let root = Path::new(SYSFS_BACKLIGHT_ROOT);

    if !path.is_absolute() {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "sysfs_path",
            message: String::from("expected an absolute path"),
        });
    }

    let relative = path
        .strip_prefix(root)
        .map_err(|_| BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "sysfs_path",
            message: format!("expected a path under {}", root.display()),
        })?;

    if relative.components().count() != 1 {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "sysfs_path",
            message: String::from("expected the backlight device directory, not a nested file"),
        });
    }

    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "sysfs_path",
            message: String::from("could not derive a device name from the path"),
        })
}

/// Scans `root` (normally `/sys/class/backlight`) and returns the name of the
/// first device directory found, sorted lexicographically for determinism.
///
/// Returns `BackendError::Io` if the directory cannot be read (e.g. the kernel
/// has no backlight subsystem entries — common on headless servers or distros
/// without the driver loaded).
pub(crate) fn scan_first_backlight_device(root: &str) -> Result<String, BackendError> {
    let root_path = Path::new(root);
    let mut entries = fs::read_dir(root_path)
        .map_err(|err| BackendError::Io {
            backend: BackendKind::Backlight,
            program: String::from("sysfs"),
            message: format!(
                "could not scan {}: {} — no backlight devices found",
                root,
                classify_io_error(&err),
            ),
            attempts: 0,
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            // Only include directories (each backlight device is a dir).
            if entry.file_type().ok()?.is_dir() {
                entry.file_name().into_string().ok()
            } else {
                None
            }
        })
        .collect::<Vec<String>>();

    entries.sort();

    entries.into_iter().next().ok_or_else(|| BackendError::Io {
        backend: BackendKind::Backlight,
        program: String::from("sysfs"),
        message: format!("{root} exists but contains no backlight device entries"),
        attempts: 0,
    })
}

/// Applies brightness via direct sysfs write, bypassing `brightnessctl`.
///
/// Reads `max_brightness` from the device directory and writes the
/// computed raw value to the `brightness` file. Intended as a fallback
/// when `brightnessctl` is not installed.
pub(crate) fn apply_sysfs_fallback(
    monitor: &MonitorConfig,
    percent: u8,
) -> Result<BackendWrite, BackendError> {
    let percent = clamp_percent(percent);
    let device_name = resolve_device_name(&monitor.selector)?;
    let device_dir = sysfs_device_dir(&device_name);

    let max_brightness = read_max_brightness(&device_dir, &device_name)?;
    let raw_value = (u64::from(percent) * max_brightness) / 100;
    write_brightness(&device_dir, &device_name, raw_value)?;

    Ok(BackendWrite {
        backend: BackendKind::Backlight,
        applied_percent: percent,
        attempts: 1,
        detail: format!("applied via sysfs direct write to {device_name}"),
    })
}

/// Returns the sysfs device directory path for a given backlight device name.
fn sysfs_device_dir(device_name: &str) -> PathBuf {
    Path::new(SYSFS_BACKLIGHT_ROOT).join(device_name)
}

/// Reads and parses the `max_brightness` value from sysfs.
fn read_max_brightness(device_dir: &Path, _device_name: &str) -> Result<u64, BackendError> {
    let path = device_dir.join("max_brightness");
    let content = fs::read_to_string(&path).map_err(|err| BackendError::Io {
        backend: BackendKind::Backlight,
        program: String::from("sysfs"),
        message: format!(
            "failed to read {}: {}",
            path.display(),
            classify_io_error(&err),
        ),
        attempts: 1,
    })?;

    content.trim().parse::<u64>().map_err(|_| BackendError::Io {
        backend: BackendKind::Backlight,
        program: String::from("sysfs"),
        message: format!(
            "invalid max_brightness value '{}' in {}",
            content.trim(),
            path.display(),
        ),
        attempts: 1,
    })
}

/// Writes a raw brightness value to the sysfs `brightness` file.
fn write_brightness(
    device_dir: &Path,
    _device_name: &str,
    raw_value: u64,
) -> Result<(), BackendError> {
    let path = device_dir.join("brightness");
    fs::write(&path, raw_value.to_string()).map_err(|err| BackendError::Io {
        backend: BackendKind::Backlight,
        program: String::from("sysfs"),
        message: format!(
            "failed to write brightness {} to {}: {}",
            raw_value,
            path.display(),
            classify_io_error(&err),
        ),
        attempts: 1,
    })
}

/// Maps std::io::ErrorKind to a human-readable label for sysfs errors.
fn classify_io_error(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "file not found",
        std::io::ErrorKind::PermissionDenied => "permission denied",
        _ => "I/O error",
    }
}

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use crate::backends::testutil::FakeRunner;
    use crate::backends::BackendError;
    use crate::config::{MonitorConfig, MonitorSelector};
    use crate::process::CommandError;

    use super::apply_with_runner;

    #[test]
    fn applies_absolute_percentage_to_specific_device() {
        let monitor = backlight_monitor(Some(String::from("/sys/class/backlight/intel_backlight")));
        let runner = FakeRunner::new().with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "37%",
            ],
            "",
        );

        let result = apply_with_runner(&runner, &monitor, 37, std::time::Duration::from_secs(2))
            .expect("backlight write should succeed");

        assert_eq!(result.applied_percent, 37);
        assert_eq!(result.attempts, 1);
        assert!(result.detail.contains("intel_backlight"));
    }

    #[test]
    fn falls_back_to_scan_when_sysfs_path_absent() {
        // With no sysfs_path configured, the backend now dynamically scans
        // /sys/class/backlight/ instead of immediately returning MissingSelector.
        // In the test environment /sys/class/backlight is either absent or has
        // no entries, so the scan itself fails with an Io error.
        let monitor = backlight_monitor(None);
        let runner = FakeRunner::new();

        let error = apply_with_runner(&runner, &monitor, 50, std::time::Duration::from_secs(2))
            .expect_err("scan of missing sysfs root must fail");

        // The error must be an Io variant (scan failure) — not a selector error.
        assert!(
            matches!(error, super::BackendError::Io { .. }),
            "expected Io scan error, got: {error:?}",
        );
        // The message must mention backlight discovery.
        let msg = error.to_string();
        assert!(
            msg.contains("backlight") || msg.contains("/sys/class/backlight"),
            "error message should mention backlight: {msg}",
        );
    }

    #[test]
    fn rejects_nested_sysfs_paths() {
        let monitor = backlight_monitor(Some(String::from(
            "/sys/class/backlight/intel_backlight/brightness",
        )));
        let runner = FakeRunner::new();

        let error = apply_with_runner(&runner, &monitor, 50, std::time::Duration::from_secs(2))
            .expect_err("nested sysfs path must fail");

        assert!(error
            .to_string()
            .contains("expected the backlight device directory"));
    }

    // --- sysfs fallback path construction ---

    #[test]
    fn sysfs_device_dir_constructs_correct_path() {
        let dir = super::sysfs_device_dir("intel_backlight");
        assert_eq!(dir, Path::new("/sys/class/backlight/intel_backlight"));
    }

    // --- percentage-to-raw conversion ---

    #[test]
    fn sysfs_fallback_writes_correct_raw_value() {
        let tmpdir = tempdir_with_backlight("test_bl", 1000);
        let sysfs_path = tmpdir.path().join("test_bl");
        let monitor = backlight_monitor_with_root(
            sysfs_path.to_str().unwrap(),
            tmpdir.path().to_str().unwrap(),
        );

        let result = apply_sysfs_fallback_with_root(&monitor, 50, tmpdir.path().to_str().unwrap())
            .expect("sysfs fallback should succeed");

        assert_eq!(result.applied_percent, 50);
        assert_eq!(result.detail, "applied via sysfs direct write to test_bl");

        let written = fs::read_to_string(sysfs_path.join("brightness"))
            .expect("brightness file should exist");
        assert_eq!(written, "500");
    }

    #[test]
    fn sysfs_fallback_zero_percent_writes_zero() {
        let tmpdir = tempdir_with_backlight("panel0", 255);
        let sysfs_path = tmpdir.path().join("panel0");
        let monitor = backlight_monitor_with_root(
            sysfs_path.to_str().unwrap(),
            tmpdir.path().to_str().unwrap(),
        );

        apply_sysfs_fallback_with_root(&monitor, 0, tmpdir.path().to_str().unwrap())
            .expect("sysfs fallback at 0% should succeed");

        let written = fs::read_to_string(sysfs_path.join("brightness"))
            .expect("brightness file should exist");
        assert_eq!(written, "0");
    }

    #[test]
    fn sysfs_fallback_full_percent_writes_max() {
        let tmpdir = tempdir_with_backlight("panel0", 255);
        let sysfs_path = tmpdir.path().join("panel0");
        let monitor = backlight_monitor_with_root(
            sysfs_path.to_str().unwrap(),
            tmpdir.path().to_str().unwrap(),
        );

        apply_sysfs_fallback_with_root(&monitor, 100, tmpdir.path().to_str().unwrap())
            .expect("sysfs fallback at 100% should succeed");

        let written = fs::read_to_string(sysfs_path.join("brightness"))
            .expect("brightness file should exist");
        assert_eq!(written, "255");
    }

    #[test]
    fn sysfs_fallback_rounding_truncates() {
        // 33% of 255 = 84.15, should truncate to 84 via integer division
        let tmpdir = tempdir_with_backlight("panel0", 255);
        let sysfs_path = tmpdir.path().join("panel0");
        let monitor = backlight_monitor_with_root(
            sysfs_path.to_str().unwrap(),
            tmpdir.path().to_str().unwrap(),
        );

        apply_sysfs_fallback_with_root(&monitor, 33, tmpdir.path().to_str().unwrap())
            .expect("sysfs fallback at 33% should succeed");

        let written = fs::read_to_string(sysfs_path.join("brightness"))
            .expect("brightness file should exist");
        assert_eq!(written, "84"); // (33 * 255) / 100 = 8415 / 100 = 84
    }

    // --- sysfs error handling ---

    #[test]
    fn sysfs_fallback_reports_missing_max_brightness() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let device_dir = tmpdir.path().join("ghost_bl");
        fs::create_dir_all(&device_dir).expect("mkdir");
        // No max_brightness file created

        let monitor = backlight_monitor_with_root(
            device_dir.to_str().unwrap(),
            tmpdir.path().to_str().unwrap(),
        );

        let err = apply_sysfs_fallback_with_root(&monitor, 50, tmpdir.path().to_str().unwrap())
            .expect_err("should fail without max_brightness");

        match &err {
            BackendError::Io {
                program, message, ..
            } => {
                assert_eq!(program, "sysfs");
                assert!(message.contains("file not found"), "got: {message}");
            }
            other => panic!("expected Io error, got: {other:?}"),
        }
    }

    #[test]
    fn sysfs_fallback_reports_invalid_max_brightness() {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let device_dir = tmpdir.path().join("bad_bl");
        fs::create_dir_all(&device_dir).expect("mkdir");
        fs::write(device_dir.join("max_brightness"), "not_a_number").expect("write max_brightness");

        let monitor = backlight_monitor_with_root(
            device_dir.to_str().unwrap(),
            tmpdir.path().to_str().unwrap(),
        );

        let err = apply_sysfs_fallback_with_root(&monitor, 50, tmpdir.path().to_str().unwrap())
            .expect_err("should fail with invalid max_brightness");

        match &err {
            BackendError::Io {
                program, message, ..
            } => {
                assert_eq!(program, "sysfs");
                assert!(message.contains("invalid max_brightness"), "got: {message}",);
            }
            other => panic!("expected Io error, got: {other:?}"),
        }
    }

    // --- apply() fallback behavior ---

    #[test]
    fn apply_with_runner_does_not_fallback_on_success() {
        let monitor = backlight_monitor(Some(String::from("/sys/class/backlight/intel_backlight")));
        let runner = FakeRunner::new().with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "70%",
            ],
            "",
        );

        let result = apply_with_runner(&runner, &monitor, 70, std::time::Duration::from_secs(2))
            .expect("should succeed via brightnessctl");

        assert!(result.detail.contains("brightnessctl"));
    }

    #[test]
    fn apply_with_runner_returns_missing_program_when_brightnessctl_absent() {
        // Verify that apply_with_runner itself propagates MissingProgram
        // (the actual fallback happens in apply(), not apply_with_runner())
        let monitor = backlight_monitor(Some(String::from("/sys/class/backlight/intel_backlight")));
        let runner = FakeRunner::new();
        runner.push_response(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "intel_backlight",
                "set",
                "50%",
            ],
            Err(CommandError::Missing {
                program: String::from("brightnessctl"),
            }),
        );

        let err = apply_with_runner(&runner, &monitor, 50, std::time::Duration::from_secs(2))
            .expect_err("should return MissingProgram");

        assert!(
            matches!(err, BackendError::MissingProgram { .. }),
            "expected MissingProgram, got: {err:?}",
        );
    }

    // --- test helpers ---

    /// Creates a temp directory simulating `/sys/class/backlight/{name}/`
    /// with a valid `max_brightness` file.
    fn tempdir_with_backlight(name: &str, max: u64) -> tempfile::TempDir {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let device_dir = tmpdir.path().join(name);
        fs::create_dir_all(&device_dir).expect("mkdir");
        fs::write(device_dir.join("max_brightness"), max.to_string())
            .expect("write max_brightness");
        tmpdir
    }

    /// Variant of `apply_sysfs_fallback` that overrides `SYSFS_BACKLIGHT_ROOT`
    /// by using a custom root directory. This avoids touching real `/sys/`
    /// during tests.
    fn apply_sysfs_fallback_with_root(
        monitor: &MonitorConfig,
        percent: u8,
        root: &str,
    ) -> Result<super::BackendWrite, BackendError> {
        use super::{clamp_percent, BackendKind};

        let percent = clamp_percent(percent);
        let device_name = super::resolve_device_name(&monitor.selector)?;
        let device_dir = std::path::Path::new(root).join(&device_name);

        let max_brightness = super::read_max_brightness(&device_dir, &device_name)?;
        let raw_value = (percent as u64 * max_brightness) / 100;
        super::write_brightness(&device_dir, &device_name, raw_value)?;

        Ok(super::BackendWrite {
            backend: BackendKind::Backlight,
            applied_percent: percent,
            attempts: 1,
            detail: format!("applied via sysfs direct write to {device_name}"),
        })
    }

    /// Helper to create a monitor config targeting a given sysfs path.
    /// When `root` differs from SYSFS_BACKLIGHT_ROOT, use
    /// `backlight_monitor_with_root()` instead.
    fn backlight_monitor(sysfs_path: Option<String>) -> MonitorConfig {
        MonitorConfig {
            logical_id: String::from("internal"),
            backend: crate::backends::BackendKind::Backlight,
            enabled: true,
            min_pct: 0,
            max_pct: 100,
            gain: 1.0,
            transition_gamma: 1.4,
            milestone_adjustments: Vec::new(),
            selector: MonitorSelector {
                connector: None,
                serial: None,
                model: None,
                edid: None,
                sysfs_path,
                ddc_bus: None,
                ddc_address: None,
            },
        }
    }

    /// Creates a monitor config pointing at a temp dir sysfs path.
    /// `resolve_device_name()` requires paths under SYSFS_BACKLIGHT_ROOT,
    /// so this builds the path within the given `root` and sets sysfs_path
    /// accordingly. Used by sysfs fallback tests.
    fn backlight_monitor_with_root(device_path: &str, _root: &str) -> MonitorConfig {
        // Extract just the device name from the full device_path
        let device_name = Path::new(device_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        // Construct the canonical /sys/class/backlight/{name} path so that
        // resolve_device_name() validation passes.
        let canonical_path = format!("/sys/class/backlight/{device_name}");
        backlight_monitor(Some(canonical_path))
    }
}
