use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{MonitorConfig, MonitorSelector};

use super::{
    clamp_percent, command_failure, map_command_error, BackendError, BackendKind, BackendWrite,
    ProcessRunner,
};

const SYSFS_BACKLIGHT_ROOT: &str = "/sys/class/backlight";

/// Compares two backlight device paths and returns `true` when they are
/// provably different pins.
///
/// The kernel warns that multiple `/sys/class/backlight/<device>` entries can
/// represent the same physical panel, so SunReactor never manufactures
/// physical identity. Logical duplication safety must come from the config
/// validator: it can only prove two monitored backlight targets are distinct
/// when both pin an explicit `sysfs_path`, and both pins differ.
#[allow(dead_code)]
pub(crate) fn backlight_selectors_disjoint(
    left: &MonitorSelector,
    right: &MonitorSelector,
) -> Option<bool> {
    let left_path = normalized(&left.sysfs_path);
    let right_path = normalized(&right.sysfs_path);

    match (left_path, right_path) {
        (Some(left_path), Some(right_path)) => Some(left_path != right_path),
        _ => None,
    }
}

/// Selects the stable control interface for a backlight monitor.
///
/// Accepts:
/// - the canonical `path` emitted by discovery (default), or
/// - a relative device directory name (for hand-written configs), or
/// - a normalized absolute `/sys/class/backlight/<device>` path.
///
/// This is the singular place where a backlight target is pinned to a
/// device.  It never falls back to "first device in `/sys/class/backlight`":
/// that historical implicit selection can target the wrong panel when
/// multiple backlight devices exist and has no encoding in the generated
/// config, so callers cannot detect ambiguous pins.  Discovery always
/// writes an explicit `sysfs_path`, so real users never lose a panel.
#[allow(dead_code)]
pub(crate) fn resolve_backlight_device_name(
    selector: &MonitorSelector,
) -> Result<String, BackendError> {
    resolve_backlight_device_name_with_diagnostic(selector).map(|(device_name, _)| device_name)
}

/// Core resolver: returns the stable backlight device name and, for legacy
/// connector-only targeting, a deprecation diagnostic the caller should
/// surface.
///
/// Dispatch order (fail-closed contract):
/// 1. DDC fields on a backlight target are always invalid.
/// 2. An explicit `sysfs_path` pin wins and ignores other selector fields.
/// 3. A connector-only legacy selector resolves ONLY through authoritative
///    kernel/driver topology evidence (`/sys/class/drm/<connector>/ddcci_backlight`),
///    never through heuristics.
/// 4. Anything else (loose serial/model/edid, or nothing) fails closed.
#[allow(dead_code)]
#[allow(clippy::unnecessary_lazy_evaluations)]
fn resolve_backlight_device_name_with_diagnostic(
    selector: &MonitorSelector,
) -> Result<(String, Option<String>), BackendError> {
    resolve_backlight_device_name_with_roots(
        selector,
        Path::new("/sys/class/drm"),
        Path::new(SYSFS_BACKLIGHT_ROOT),
    )
}

fn resolve_backlight_device_name_with_roots(
    selector: &MonitorSelector,
    drm_root: &Path,
    backlight_root: &Path,
) -> Result<(String, Option<String>), BackendError> {
    if selector.ddc_bus.is_some() || selector.ddc_address.is_some() {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "ddc_bus/ddc_address",
            message: String::from("DDC selectors do not apply to backlight devices"),
        });
    }

    if let Some(path_value) = normalized(&selector.sysfs_path) {
        // The explicit pin wins; extra identity fields cannot improve a pin
        // and are deliberately ignored (they were accepted historically).
        let path = Path::new(&path_value);
        let device_name = if path.is_absolute() {
            resolve_explicit_sysfs_path(&path_value)?
        } else {
            resolve_relative_device_name(&path_value)?
        };
        return Ok((device_name, None));
    }

    let connector = normalized(&selector.connector);
    let has_loose_identity = normalized(&selector.serial).is_some()
        || normalized(&selector.model).is_some()
        || normalized(&selector.edid).is_some();

    match connector {
        Some(_) if !has_loose_identity => {
            let (device_name, diagnostic) =
                resolve_connector_backlight_legacy(selector, drm_root, backlight_root)?;
            Ok((device_name, Some(diagnostic)))
        }
        _ => Err(BackendError::MissingSelector {
            backend: BackendKind::Backlight,
            expected: "sysfs_path (an explicit /sys/class/backlight/<device> path)",
        }),
    }
}

#[allow(dead_code)]
fn resolve_device_name(selector: &MonitorSelector) -> Result<String, BackendError> {
    resolve_backlight_device_name(selector)
}

/// Resolves a legacy connector-only backlight selector using authoritative
/// kernel/driver topology evidence.
///
/// # Contract (owner decision, Phase 8 Option 1)
///
/// A connector-only selector is resolved ONLY when SunReactor can produce
/// direct, non-heuristic evidence linking the connector to exactly one
/// `/sys/class/backlight` device.
///
/// The only accepted evidence today is the kernel/driver-provided
/// ``ddcci_backlight`` symlink created by ddcci-driver-linux on the DRM
/// connector directory (`/sys/class/drm/<connector>/ddcci_backlight`), which
/// points to the same hardware device that owns the backlight class entry
/// (`/sys/class/backlight/<device>/device`).  When both links converge on one
/// hardware device the topology is proven and unambiguous.
///
/// Deliberately rejected as insufficient: filename ordering, "first device",
/// device-name similarity, GPU vendor, backlight type, and the mere
/// existence of a single backlight entry. All such heuristics can target the
/// wrong physical panel when multiple backlight devices exist.
///
/// Returns `(device_name, diagnostics)`. Zero or multiple proven mappings
/// fail closed with `MissingSelector`.
///
/// The diagnostic string is a legacy/deprecation notice: this syntactic form
/// was accepted historically but never provided safe physical targeting
/// (the connector was effectively ignored by the old first-device fallback).
pub(crate) fn resolve_connector_backlight_legacy(
    selector: &MonitorSelector,
    drm_root: &Path,
    backlight_root: &Path,
) -> Result<(String, String), BackendError> {
    if selector.ddc_bus.is_some()
        || selector.ddc_address.is_some()
        || normalized(&selector.serial).is_some()
        || normalized(&selector.model).is_some()
        || normalized(&selector.edid).is_some()
    {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "connector",
            message: String::from(
                "connector-only backlight resolution requires exactly a connector field",
            ),
        });
    }

    let connector_name = normalized(&selector.connector).ok_or(BackendError::MissingSelector {
        backend: BackendKind::Backlight,
        expected: "connector (for legacy connector-only backlight configs)",
    })?;

    // Connector path: /sys/class/drm/<connector>/ddcci_backlight ->
    //   the hardware device that owns the backlight class entry.
    let connector_link = drm_root.join(&connector_name).join("ddcci_backlight");
    let hw_device = canonicalize(&connector_link).ok_or_else(|| BackendError::InvalidSelector {
        backend: BackendKind::Backlight,
        field: "connector",
        message: format!(
            "no authoritative kernel/driver connector→backlight link at {} (legacy \
             connector-only targeting cannot be resolved without direct topology evidence)",
            connector_link.display()
        ),
    })?;

    // Enumerate backlight class entries and keep only those whose `device`
    // link converges on the same hardware device.
    let mut candidates: Vec<String> = Vec::new();
    let entries = fs::read_dir(backlight_root);
    if let Ok(entries) = entries {
        for entry in entries.flatten() {
            let device_path = entry.path();
            let name = device_path.file_name().and_then(|v| v.to_str());
            let Some(name) = name else { continue };
            let device_link = device_path.join("device");
            if canonicalize(&device_link).as_deref() == Some(hw_device.as_path()) {
                candidates.push(String::from(name));
            }
        }
    }

    match candidates.len() {
        1 => {
            let device_name = candidates.into_iter().next().expect("len==1");
            let diagnostic = format!(
                "connector-only backlight targeting via `connector=\"{connector_name}\"` is \
                 deprecated: it resolved through the kernel/driver `ddcci_backlight` link; \
                 pin an explicit `sysfs_path` in config.toml to make targeting stable"
            );
            Ok((device_name, diagnostic))
        }
        0 => Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "connector",
            message: format!(
                "no proven backlight device for connector `{connector_name}`: zero \
                 /sys/class/backlight entries link to the connector's driver device \
                 (no heuristic connector-to-panel mapping is available for ordinary internal panels)"
            ),
        }),
        n => Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "connector",
            message: format!(
                "connector `{connector_name}` maps to {n} proven backlight devices; \
                 refusing ambiguous targeting — pin an explicit sysfs_path"
            ),
        }),
    }
}

/// Canonicalizes `path` (following symlinks) and returns the absolute path,
/// or `None` on any I/O failure.
fn canonicalize(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn resolve_relative_device_name(device_name: &str) -> Result<String, BackendError> {
    let trimmed = device_name.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains("..") {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Backlight,
            field: "sysfs_path",
            message: format!("invalid backlight device name `{device_name}`"),
        });
    }
    Ok(String::from("/sys/class/backlight/") + trimmed)
}

/// Validates an explicit absolute `sysfs_path` and extracts the device name component.
pub(crate) fn resolve_explicit_sysfs_path(raw_path: &str) -> Result<String, BackendError> {
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

pub(crate) fn apply_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
) -> Result<BackendWrite, BackendError> {
    apply_with_runner_roots(
        runner,
        monitor,
        percent,
        timeout,
        Path::new("/sys/class/drm"),
        Path::new(SYSFS_BACKLIGHT_ROOT),
    )
}

#[cfg(test)]
pub(crate) fn apply_with_runner_roots_for_test<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
    drm_root: &Path,
    backlight_root: &Path,
) -> Result<BackendWrite, BackendError> {
    apply_with_runner_roots(runner, monitor, percent, timeout, drm_root, backlight_root)
}

fn apply_with_runner_roots<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
    drm_root: &Path,
    backlight_root: &Path,
) -> Result<BackendWrite, BackendError> {
    let percent = clamp_percent(percent);
    let (device_name, diagnostic) =
        resolve_backlight_device_name_with_roots(&monitor.selector, drm_root, backlight_root)?;
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
            detail: format!(
                "applied via brightnessctl device `{device_name}`{suffix}",
                suffix = diagnostic.map(|d| format!("; {d}")).unwrap_or_default(),
            ),
        }),
        Ok(output) => Err(command_failure(
            BackendKind::Backlight,
            "brightnessctl",
            &output,
        )),
        Err(error) => Err(map_command_error(BackendKind::Backlight, error)),
    }
}

/// Applies brightness via direct sysfs write, bypassing `brightnessctl`.
///
/// Reads `max_brightness` from the device directory and writes the
/// computed raw value to the `brightness` file. Intended as a fallback
/// when `brightnessctl` is not installed.
#[allow(dead_code)]
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
#[allow(dead_code)]
fn sysfs_device_dir(device_name: &str) -> PathBuf {
    Path::new(SYSFS_BACKLIGHT_ROOT).join(device_name)
}

/// Reads and parses the `max_brightness` value from sysfs.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    fn rejects_unpinned_backlight_before_invoking_brightnessctl() {
        let monitor = backlight_monitor(None);
        let runner = FakeRunner::new();

        let error = apply_with_runner(&runner, &monitor, 50, std::time::Duration::from_secs(2))
            .expect_err("an unpinned backlight must fail closed");

        assert!(
            matches!(error, super::BackendError::MissingSelector { .. }),
            "expected MissingSelector, got: {error:?}",
        );
        assert!(runner.calls().is_empty(), "must not invoke brightnessctl");
    }

    #[test]
    fn legacy_two_device_fixture_fails_closed_instead_of_sorting_names() {
        // A pair of viable class entries is intentionally ambiguous: neither
        // filename proves which physical panel should be controlled.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        for name in ["acpi_video0", "intel_backlight"] {
            let device_dir = tmpdir.path().join(name);
            fs::create_dir_all(&device_dir).expect("mkdir");
            fs::write(device_dir.join("max_brightness"), "100").expect("write max");
        }

        let monitor = backlight_monitor(None);
        let error = apply_with_runner(
            &FakeRunner::new(),
            &monitor,
            50,
            std::time::Duration::from_secs(2),
        )
        .expect_err("ambiguous legacy selection must fail closed");

        assert!(matches!(error, super::BackendError::MissingSelector { .. }));
    }

    #[test]
    fn explicit_pins_are_not_selected_by_device_name_order() {
        let acpi = backlight_monitor(Some(String::from("/sys/class/backlight/acpi_video0")));
        let intel = backlight_monitor(Some(String::from("/sys/class/backlight/intel_backlight")));

        // Backend resolves the PIN, never a device picked by filename order.
        assert_eq!(
            super::resolve_backlight_device_name(&acpi.selector).unwrap(),
            "acpi_video0"
        );
        assert_eq!(
            super::resolve_backlight_device_name(&intel.selector).unwrap(),
            "intel_backlight"
        );
        // Two pins are provably different control interfaces.
        assert_eq!(
            super::backlight_selectors_disjoint(&acpi.selector, &intel.selector),
            Some(true)
        );
    }

    #[test]
    fn rejects_ddc_selectors_on_backlight() {
        // Backlight targets may not be pinned via DDC-only selectors.
        let mut monitor =
            backlight_monitor(Some(String::from("/sys/class/backlight/intel_backlight")));
        monitor.selector.ddc_bus = Some(1);
        let error = apply_with_runner(
            &FakeRunner::new(),
            &monitor,
            50,
            std::time::Duration::from_secs(2),
        )
        .expect_err("DDC selectors are invalid on a backlight target");

        assert!(
            matches!(error, super::BackendError::InvalidSelector { .. }),
            "expected InvalidSelector, got: {error:?}",
        );
    }

    #[test]
    fn legacy_connector_only_resolves_via_ddcci_symlink_and_emits_diagnostic() {
        // Option 1: a legacy connector-only config resolves ONLY when an
        // authoritative kernel/driver symlink on the DRM connector directory
        // (e.g. ddcci-driver-linux's `ddcci_backlight`) points to the device
        // that also owns a /sys/class/backlight entry. No heuristics.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let drm_root = tmpdir.path().join("drm");
        let conn = drm_root.join("card1-DP-1");
        fs::create_dir_all(&conn).expect("mkdir connector dir");
        let backlight_root = tmpdir.path().join("backlight");
        let hw_device = tmpdir.path().join("hwdd");
        fs::create_dir_all(&hw_device).expect("mkdir hw device");
        let device = backlight_root.join("ddcci_backlight_0");
        fs::create_dir_all(&device).expect("mkdir device");
        fs::write(device.join("max_brightness"), "100").expect("write max");
        fs::write(device.join("type"), "raw\n").expect("write type");
        // Kernel/driver topology: connector ->
        //   ddcci device; backlight class -> same ddcci device.
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_device, conn.join("ddcci_backlight")).expect("symlink");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_device, device.join("device")).expect("device symlink");

        let mut monitor = backlight_monitor(None);
        monitor.selector.connector = Some(String::from("card1-DP-1"));

        let (resolved, diag) = super::resolve_connector_backlight_legacy(
            &monitor.selector,
            &drm_root,
            &backlight_root,
        )
        .expect("exactly one proven mapping must resolve");

        assert_eq!(resolved, "ddcci_backlight_0");
        assert!(
            diag.to_lowercase().contains("deprecat"),
            "diagnostic must be a legacy/deprecation diagnostic, got {diag}"
        );
    }

    #[test]
    fn legacy_connector_without_proven_mapping_fails_closed() {
        // Same connector, but no authoritative symlink: zero proven mappings.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let drm_root = tmpdir.path().join("drm");
        let conn = drm_root.join("card1-eDP-1");
        fs::create_dir_all(&conn).expect("create connector dir");
        let backlight_root = tmpdir.path().join("backlight");
        let device = backlight_root.join("intel_backlight");
        fs::create_dir_all(&device).expect("mkdir");
        fs::write(device.join("max_brightness"), "100").expect("max");
        fs::write(device.join("type"), "raw\n").expect("type");

        let mut monitor = backlight_monitor(None);
        monitor.selector.connector = Some(String::from("card1-eDP-1"));

        super::resolve_connector_backlight_legacy(&monitor.selector, &drm_root, &backlight_root)
            .expect_err("zero proven mappings must fail closed");
    }

    #[test]
    fn never_selects_backlight_by_alphabetical_first_name_without_evidence() {
        // Regression for the historical first-device / lexicographic fallback:
        // a connector-only selector must NOT pick `aa_backlight` merely
        // because it sorts first when no authoritative link exists.
        //
        // Both class entries exist and are identical in every observable
        // attribute except their names. Neither has a `device` symlink that
        // converges on the connector's hardware device. Zero proven mappings
        // must fail closed — never fall back to name ordering.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let drm_root = tmpdir.path().join("drm");
        let conn = drm_root.join("card1-eDP-1");
        fs::create_dir_all(&conn).expect("mkdir connector dir");
        let backlight_root = tmpdir.path().join("backlight");

        // Names deliberately arranged so the lexicographically FIRST name
        // (`aa_backlight`) is the one a historical fallback would pick,
        // while the connector belongs to a different panel.
        for name in ["aa_backlight", "zz_backlight"] {
            let device = backlight_root.join(name);
            fs::create_dir_all(&device).expect("mkdir device");
            fs::write(device.join("max_brightness"), "100").expect("max");
            fs::write(device.join("type"), "raw\n").expect("type");
            // No connector/driver symlink on the connector dir and no
            // `device` link in the class entry: unproven candidates.
        }

        let mut monitor = backlight_monitor(None);
        monitor.selector.connector = Some(String::from("card1-eDP-1"));

        let result = super::resolve_connector_backlight_legacy(
            &monitor.selector,
            &drm_root,
            &backlight_root,
        );
        assert!(
            result.is_err(),
            "unproven connector must fail closed; never select by name order (got {result:?})"
        );
    }

    #[test]
    fn picks_proven_device_despite_alphabetically_first_unproven_candidate() {
        // Strong ordering guarantee: even when the lexicographically FIRST
        // backlight entry has no proof and a LATER entry has authoritative
        // convergence, only the later (proven) device may be selected.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let drm_root = tmpdir.path().join("drm");
        let conn = drm_root.join("card1-DP-1");
        fs::create_dir_all(&conn).expect("mkdir connector dir");
        let backlight_root = tmpdir.path().join("backlight");
        let hw_device = tmpdir
            .path()
            .join("devices")
            .join("pci0000:00")
            .join("0000:00:02.0")
            .join("card0");
        fs::create_dir_all(&hw_device).expect("mkdir hw device");

        // Unproven candidate that sorts first (has a device dir but NO `device` symlink)
        let unproven = backlight_root.join("aa_backlight");
        fs::create_dir_all(&unproven).expect("mkdir");
        fs::write(unproven.join("max_brightness"), "100").expect("max");

        // Proven candidate that sorts later (its `device` link converges with
        // the connector's ddcci symlink on the same hw device)
        let proven = backlight_root.join("zz_backlight");
        fs::create_dir_all(&proven).expect("mkdir");
        fs::write(proven.join("max_brightness"), "100").expect("max");
        fs::write(proven.join("type"), "raw\n").expect("type");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_device, proven.join("device")).expect("device symlink");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_device, conn.join("ddcci_backlight"))
            .expect("connector symlink");

        let mut monitor = backlight_monitor(None);
        monitor.selector.connector = Some(String::from("card1-DP-1"));

        let (resolved, _diag) = super::resolve_connector_backlight_legacy(
            &monitor.selector,
            &drm_root,
            &backlight_root,
        )
        .expect("exactly one proven mapping");

        assert_eq!(
            resolved, "zz_backlight",
            "resolver must select the proven device, never an unproven alphabetically-first one"
        );
    }

    #[test]
    fn realistic_sysfs_devices_symlink_chain_is_followed_not_plain_dirs() {
        // Exercises the exact `/sys/class/backlight/<dev>/device ->
        // /sys/devices/...` topology that a real machine exposes: the
        // symlink target is itself a multi-component directory tree that
        // canonicalizes to the same path the connector's `ddcci_backlight`
        // link resolves to. Canonicalisation (not name comparison) is what
        // proves identity.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let drm_root = tmpdir.path().join("class").join("drm");
        let conn = drm_root.join("card1-DP-1");
        fs::create_dir_all(&conn).expect("mkdir connector dir");
        let backlight_root = tmpdir.path().join("class").join("backlight");

        // A realistic hardware-device tree under /sys/devices/pci.../drm/...
        let hw_dir = tmpdir
            .path()
            .join("devices")
            .join("pci0000:00")
            .join("0000:00:02.0")
            .join("drm")
            .join("card0")
            .join("card0-DP-1")
            .join("backlight")
            .join("ddcci_backlight");
        fs::create_dir_all(&hw_dir).expect("mkdir hw tree");

        let device = backlight_root.join("ddcci_backlight_0");
        fs::create_dir_all(&device).expect("mkdir device");
        fs::write(device.join("max_brightness"), "100").expect("max");
        // The backlight's `device` symlink points at the same hw tree that
        // the connector driver symlink targets.
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_dir, device.join("device")).expect("device symlink");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_dir, conn.join("ddcci_backlight"))
            .expect("connector symlink");

        let mut monitor = backlight_monitor(None);
        monitor.selector.connector = Some(String::from("card1-DP-1"));

        let (resolved, diag) = super::resolve_connector_backlight_legacy(
            &monitor.selector,
            &drm_root,
            &backlight_root,
        )
        .expect("realistic devices-tree convergence must resolve");

        assert_eq!(resolved, "ddcci_backlight_0");
        assert!(diag.to_lowercase().contains("deprecat"));
    }

    #[test]
    fn legacy_connector_multiple_proven_candidates_fails_closed() {
        // Two backlight class entries whose `device` links both resolve to the
        // same connector-linked ddcci device: ambiguous, fail closed.
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let drm_root = tmpdir.path().join("drm");
        let conn = drm_root.join("card1-HDMI-A-1");
        fs::create_dir_all(&conn).expect("mkdir connector dir");
        let backlight_root = tmpdir.path().join("backlight");
        let hw_device = tmpdir.path().join("hwdd2");
        fs::create_dir_all(&hw_device).expect("mkdir hw device");
        for name in ["ddcci_backlight_0", "ddcci_backlight_1"] {
            let device = backlight_root.join(name);
            fs::create_dir_all(&device).expect("mkdir device");
            fs::write(device.join("max_brightness"), "100").expect("max");
            fs::write(device.join("type"), "raw\n").expect("type");
            #[cfg(unix)]
            std::os::unix::fs::symlink(&hw_device, device.join("device")).expect("device symlink");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&hw_device, conn.join("ddcci_backlight")).expect("symlink");

        let mut monitor = backlight_monitor(None);
        monitor.selector.connector = Some(String::from("card1-HDMI-A-1"));

        super::resolve_connector_backlight_legacy(&monitor.selector, &drm_root, &backlight_root)
            .expect_err("multiple proven mappings must fail closed");
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
                assert!(message.contains("invalid max_brightness"), "got: {message}");
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
        let raw_value = (u64::from(percent) * max_brightness) / 100;
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
            allow_topology_retargeting: false,
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
