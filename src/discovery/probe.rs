use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::model::{
    build_backlight_stable_id, BackendStatus, BackendStatusKind, BacklightDeviceDiscovery,
    DdcMonitorDiscovery, DiscoveryBackends, DiscoverySnapshot, DiscoverySummary, RawDdcMonitor,
};
use super::runner::{command_failure_detail, CommandError, ProcessRunner};
use crate::backends::ddc::{filter_unsupported_noconfig, is_unsupported_noconfig_error};

// `ddcutil detect` probes every I2C bus and routinely takes five seconds or
// more on multi-output GPUs, so the limit leaves generous headroom.
const DDCUTIL_DETECT_TIMEOUT: Duration = Duration::from_secs(15);
const DDCUTIL_CAPABILITIES_TIMEOUT: Duration = Duration::from_secs(6);
const BRIGHTNESSCTL_LIST_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn discover_with_runner<R: ProcessRunner>(
    runner: &R,
    sysfs_root: &Path,
) -> DiscoverySnapshot {
    discover_with_roots(runner, sysfs_root, Path::new("/sys/class/drm"))
}

pub(crate) fn discover_with_roots<R: ProcessRunner>(
    runner: &R,
    sysfs_root: &Path,
    drm_root: &Path,
) -> DiscoverySnapshot {
    let (ddc_status, ddc_monitors, ddc_observation_complete) = discover_ddc_monitors(runner);
    let (brightnessctl_status, brightnessctl_devices) =
        discover_brightnessctl_backlights(runner, sysfs_root);
    let (sysfs_status, sysfs_devices) = discover_sysfs_backlights(sysfs_root);
    let mut backlight_devices = merge_backlight_devices(brightnessctl_devices, sysfs_devices);
    annotate_ddcci_connector_evidence(&mut backlight_devices, drm_root);

    backlight_devices.sort_by(|left, right| left.device_name.cmp(&right.device_name));

    let summary = DiscoverySummary {
        ddc_monitors: ddc_monitors.len(),
        backlight_devices: backlight_devices.len(),
        viable_targets: ddc_monitors
            .iter()
            .filter(|monitor| monitor.backend_viable)
            .count()
            + backlight_devices
                .iter()
                .filter(|device| {
                    device.backend_viable
                        && !super::is_ddcci_alias_of_viable_ddc(device, &ddc_monitors)
                })
                .count(),
    };

    DiscoverySnapshot {
        summary,
        backends: DiscoveryBackends {
            ddcutil: ddc_status,
            brightnessctl: brightnessctl_status,
            sysfs: sysfs_status,
        },
        ddc_observation_complete,
        ddc_monitors,
        backlight_devices,
    }
}

#[allow(clippy::too_many_lines)]
fn discover_ddc_monitors<R: ProcessRunner>(
    runner: &R,
) -> (BackendStatus, Vec<DdcMonitorDiscovery>, bool) {
    let args = vec![
        String::from("--noconfig"),
        String::from("--terse"),
        String::from("detect"),
    ];

    let run_detect = runner.run("ddcutil", &args, DDCUTIL_DETECT_TIMEOUT);
    let detect_result = match run_detect {
        Ok(output) if !output.success() && is_unsupported_noconfig_error(&output) => {
            let fallback = filter_unsupported_noconfig(&args);
            runner.run("ddcutil", &fallback, DDCUTIL_DETECT_TIMEOUT)
        }
        other => other,
    };

    match detect_result {
        Ok(output) if output.success() => {
            let mut monitors = parse_ddc_detect(&output.stdout);
            let observation_complete = !has_invalid_display_record(&output.stdout);
            super::assign_ddc_occurrence_ids(&mut monitors);
            let mut capability_failures = 0usize;

            for monitor in &mut monitors {
                // `--display N` makes ddcutil repeat the full bus scan; the bus
                // reported by `detect` addresses the same display directly.
                let (selector_flag, selector_value) = monitor.bus_number.map_or_else(
                    || ("--display", monitor.display_number.to_string()),
                    |bus| ("--bus", bus.to_string()),
                );
                let capability_args = vec![
                    String::from("--noconfig"),
                    String::from(selector_flag),
                    selector_value,
                    String::from("capabilities"),
                ];

                let cap_run = runner.run("ddcutil", &capability_args, DDCUTIL_CAPABILITIES_TIMEOUT);
                let cap_result = match cap_run {
                    Ok(output) if !output.success() && is_unsupported_noconfig_error(&output) => {
                        let fallback = filter_unsupported_noconfig(&capability_args);
                        runner.run("ddcutil", &fallback, DDCUTIL_CAPABILITIES_TIMEOUT)
                    }
                    other => other,
                };

                match cap_result {
                    Ok(capabilities) if capabilities.success() => {
                        let supported = parse_brightness_vcp_support(&capabilities.stdout);
                        monitor.brightness_vcp_supported = Some(supported);
                        monitor.backend_viable = supported;
                    }
                    Ok(capabilities) => {
                        capability_failures += 1;
                        monitor.note = Some(format!(
                            "capabilities probe failed: {}",
                            command_failure_detail(&capabilities)
                        ));
                    }
                    Err(CommandError::Timeout { after, .. }) => {
                        capability_failures += 1;
                        monitor.note = Some(format!(
                            "capabilities probe timed out after {}s",
                            after.as_secs()
                        ));
                    }
                    Err(error) => {
                        capability_failures += 1;
                        monitor.note = Some(error.to_string());
                    }
                }
            }

            let message = if monitors.is_empty() {
                String::from("No external monitors were reported by ddcutil.")
            } else if capability_failures == 0 {
                format!("Detected {} external monitor(s).", monitors.len())
            } else {
                format!(
                    "Detected {} external monitor(s); {} capability probe(s) failed.",
                    monitors.len(),
                    capability_failures
                )
            };

            (
                BackendStatus {
                    backend: String::from("ddcutil"),
                    status: BackendStatusKind::Ok,
                    available: true,
                    message,
                    guidance: None,
                },
                monitors,
                observation_complete,
            )
        }
        Ok(output) => {
            let detail = command_failure_detail(&output);
            let guidance = if detail.contains("i2c-dev")
                || output.stderr.contains("i2c-dev")
                || output.stdout.contains("i2c-dev")
                || detail.contains("No /dev/i2c devices exist")
            {
                String::from("Kernel module `i2c-dev` is not loaded. Load it with `sudo modprobe i2c-dev` or add it to `/etc/modules-load.d/i2c.conf`.")
            } else if detail.contains("Permission denied")
                || output.stderr.contains("Permission denied")
            {
                String::from("Permission denied accessing /dev/i2c-* devices. Add your user to the `i2c` group (`sudo usermod -aG i2c $USER`) or configure uaccess rules, then relogin.")
            } else {
                String::from("Ensure the user can access the relevant /dev/i2c-* devices and rerun discovery.")
            };
            (
                BackendStatus {
                    backend: String::from("ddcutil"),
                    status: BackendStatusKind::Error,
                    available: true,
                    message: format!("ddcutil detect failed: {detail}"),
                    guidance: Some(guidance),
                },
                Vec::new(),
                false,
            )
        }
        Err(CommandError::Missing { .. }) => (
            BackendStatus {
                backend: String::from("ddcutil"),
                status: BackendStatusKind::Missing,
                available: false,
                message: String::from("ddcutil is not installed."),
                guidance: Some(String::from(
                    "Install `ddcutil` to discover external monitors and verify VCP 0x10 brightness support.",
                )),
            },
            Vec::new(),
            false,
        ),
        Err(CommandError::Timeout { after, .. }) => (
            BackendStatus {
                backend: String::from("ddcutil"),
                status: BackendStatusKind::Timeout,
                available: true,
                message: format!("ddcutil detect timed out after {}s.", after.as_secs()),
                guidance: Some(String::from(
                    "Retry when the I2C bus is idle; busy or wedged DDC busses can stall discovery.",
                )),
            },
            Vec::new(),
            false,
        ),
        Err(error) => (
            BackendStatus {
                backend: String::from("ddcutil"),
                status: BackendStatusKind::Error,
                available: true,
                message: error.to_string(),
                guidance: Some(String::from(
                    "Check ddcutil access and monitor cabling, then rerun discovery.",
                )),
            },
            Vec::new(),
            false,
        ),
    }
}

fn has_invalid_display_record(output: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("Invalid display"))
}

fn discover_brightnessctl_backlights<R: ProcessRunner>(
    runner: &R,
    sysfs_root: &Path,
) -> (BackendStatus, Vec<BacklightDeviceDiscovery>) {
    let args = vec![
        String::from("--list"),
        String::from("--machine-readable"),
        String::from("--class"),
        String::from("backlight"),
    ];

    match runner.run("brightnessctl", &args, BRIGHTNESSCTL_LIST_TIMEOUT) {
        Ok(output) if output.success() => {
            let devices = parse_brightnessctl_backlights(&output.stdout, sysfs_root);
            let message = if devices.is_empty() {
                String::from("brightnessctl reported no backlight devices.")
            } else {
                format!("brightnessctl reported {} backlight device(s).", devices.len())
            };

            (
                BackendStatus {
                    backend: String::from("brightnessctl"),
                    status: BackendStatusKind::Ok,
                    available: true,
                    message,
                    guidance: None,
                },
                devices,
            )
        }
        Ok(output) => (
            BackendStatus {
                backend: String::from("brightnessctl"),
                status: BackendStatusKind::Error,
                available: true,
                message: format!(
                    "brightnessctl list failed: {}",
                    command_failure_detail(&output)
                ),
                guidance: Some(String::from(
                    "Check that `brightnessctl` can enumerate backlight devices for the current user.",
                )),
            },
            Vec::new(),
        ),
        Err(CommandError::Missing { .. }) => (
            BackendStatus {
                backend: String::from("brightnessctl"),
                status: BackendStatusKind::Missing,
                available: false,
                message: String::from("brightnessctl is not installed."),
                guidance: Some(String::from(
                    "Install `brightnessctl` to enumerate internal panels, or rely on sysfs fallback if available.",
                )),
            },
            Vec::new(),
        ),
        Err(CommandError::Timeout { after, .. }) => (
            BackendStatus {
                backend: String::from("brightnessctl"),
                status: BackendStatusKind::Timeout,
                available: true,
                message: format!("brightnessctl timed out after {}s.", after.as_secs()),
                guidance: Some(String::from(
                    "Retry discovery and inspect the local backlight stack if the command keeps hanging.",
                )),
            },
            Vec::new(),
        ),
        Err(error) => (
            BackendStatus {
                backend: String::from("brightnessctl"),
                status: BackendStatusKind::Error,
                available: true,
                message: error.to_string(),
                guidance: Some(String::from(
                    "Check that brightnessctl is functional for the current session, or use sysfs fallback.",
                )),
            },
            Vec::new(),
        ),
    }
}

fn discover_sysfs_backlights(sysfs_root: &Path) -> (BackendStatus, Vec<BacklightDeviceDiscovery>) {
    if !sysfs_root.exists() {
        return (
            BackendStatus {
                backend: String::from("sysfs"),
                status: BackendStatusKind::Unavailable,
                available: false,
                message: format!("{} does not exist.", sysfs_root.display()),
                guidance: Some(String::from(
                    "Expose a backlight device under `/sys/class/backlight` for sysfs fallback discovery.",
                )),
            },
            Vec::new(),
        );
    }

    let entries = match fs::read_dir(sysfs_root) {
        Ok(entries) => entries,
        Err(error) => {
            return (
                BackendStatus {
                    backend: String::from("sysfs"),
                    status: BackendStatusKind::Error,
                    available: true,
                    message: format!("failed to read {}: {error}", sysfs_root.display()),
                    guidance: Some(String::from(
                        "Check permissions on the sysfs backlight directory and rerun discovery.",
                    )),
                },
                Vec::new(),
            );
        }
    };

    let mut devices = Vec::new();

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(device_name) = entry.file_name().into_string().ok() else {
            continue;
        };

        let max_brightness = read_optional_u32(&path.join("max_brightness"));
        let backlight_type = read_optional_text(&path.join("type"));
        let brightness_exists = path.join("brightness").exists();
        let backend_viable = brightness_exists && max_brightness.unwrap_or(0) > 0;
        let note = if brightness_exists {
            None
        } else {
            Some(String::from("brightness file is missing"))
        };

        devices.push(BacklightDeviceDiscovery {
            stable_id: build_backlight_stable_id(&device_name),
            device_name,
            class: String::from("backlight"),
            max_brightness,
            backlight_type,
            probe_source: String::from("sysfs"),
            sysfs_path: path.display().to_string(),
            ddcci_connector: None,
            backend_viable,
            note,
        });
    }

    devices.sort_by(|left, right| left.device_name.cmp(&right.device_name));

    let message = if devices.is_empty() {
        format!("No backlight devices found under {}.", sysfs_root.display())
    } else {
        format!(
            "Found {} backlight device(s) under {}.",
            devices.len(),
            sysfs_root.display()
        )
    };

    (
        BackendStatus {
            backend: String::from("sysfs"),
            status: BackendStatusKind::Ok,
            available: true,
            message,
            guidance: None,
        },
        devices,
    )
}

fn annotate_ddcci_connector_evidence(
    backlight_devices: &mut [BacklightDeviceDiscovery],
    drm_root: &Path,
) {
    // Build a map: canonical hardware device path -> connector name.
    // Evidence = the kernel/driver-provided `ddcci_backlight` symlink inside
    // `/sys/class/drm/<connector>` that converges (via realpath) on the same
    // hardware device that owns a `/sys/class/backlight` class entry.
    let mut connectors_by_hw: BTreeMap<_, Vec<String>> = BTreeMap::new();
    if let Ok(entries) = fs::read_dir(drm_root) {
        for entry in entries.flatten() {
            let connector_name = entry.file_name().to_string_lossy().into_owned();
            let ddcci_link = entry.path().join("ddcci_backlight");
            if let Ok(hw) = fs::canonicalize(&ddcci_link) {
                connectors_by_hw.entry(hw).or_default().push(connector_name);
            }
        }
    }

    for device in backlight_devices.iter_mut() {
        // The `device` symlink on a backlight class entry points to the
        // hardware device; canonicalize so we compare resolved paths, never
        // names or ordering.
        let device_path = Path::new(&device.sysfs_path);
        let Ok(device_hw) = fs::canonicalize(device_path.join("device")) else {
            continue;
        };
        if let Some(connectors) = connectors_by_hw.get(&device_hw) {
            if connectors.len() == 1 {
                device.ddcci_connector = connectors.first().cloned();
            }
        }
    }
}

fn parse_ddc_detect(output: &str) -> Vec<DdcMonitorDiscovery> {
    let mut monitors = Vec::new();
    let mut current: Option<RawDdcMonitor> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(display_number) = parse_display_header(trimmed) {
            if let Some(monitor) = current.take() {
                monitors.push(monitor.into_discovery());
            }
            current = Some(RawDdcMonitor::new(display_number));
            continue;
        }

        let Some(monitor) = current.as_mut() else {
            continue;
        };

        if let Some(value) = trimmed.strip_prefix("I2C bus:") {
            monitor.bus_number = parse_bus_number(value.trim());
        } else if let Some(value) = trimmed.strip_prefix("DRM connector:") {
            monitor.connector = normalize_optional(value.trim());
        } else if let Some(value) = trimmed.strip_prefix("Monitor:") {
            let (manufacturer, model, serial) = parse_monitor_identity(value.trim());
            monitor.manufacturer = manufacturer;
            monitor.model = model;
            monitor.serial = serial;
        }
    }

    if let Some(monitor) = current {
        monitors.push(monitor.into_discovery());
    }

    monitors.sort_by_key(|monitor| monitor.display_number);
    monitors
}

fn parse_brightness_vcp_support(output: &str) -> bool {
    for line in output.lines() {
        let normalized = line.trim().to_ascii_lowercase();
        let Some(rest) = normalized.strip_prefix("feature:") else {
            continue;
        };
        let Some(code) = rest.split_whitespace().next() else {
            continue;
        };
        if code == "10" {
            return true;
        }
    }

    false
}

fn parse_brightnessctl_backlights(
    output: &str,
    sysfs_root: &Path,
) -> Vec<BacklightDeviceDiscovery> {
    let mut devices = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let fields = trimmed.splitn(5, ',').collect::<Vec<_>>();
        if fields.len() != 5 {
            continue;
        }

        let device_name = fields[0].trim();
        let class = fields[1].trim();
        if device_name.is_empty() || class != "backlight" {
            continue;
        }

        let max_brightness = fields[4].trim().parse::<u32>().ok();
        let sysfs_path = sysfs_root.join(device_name);

        devices.push(BacklightDeviceDiscovery {
            stable_id: build_backlight_stable_id(device_name),
            device_name: device_name.to_owned(),
            class: class.to_owned(),
            max_brightness,
            backlight_type: None,
            probe_source: String::from("brightnessctl"),
            sysfs_path: sysfs_path.display().to_string(),
            ddcci_connector: None,
            backend_viable: max_brightness.unwrap_or(0) > 0,
            note: None,
        });
    }

    devices.sort_by(|left, right| left.device_name.cmp(&right.device_name));
    devices
}

fn merge_backlight_devices(
    brightnessctl_devices: Vec<BacklightDeviceDiscovery>,
    sysfs_devices: Vec<BacklightDeviceDiscovery>,
) -> Vec<BacklightDeviceDiscovery> {
    let mut merged = BTreeMap::new();

    for device in sysfs_devices {
        merged.insert(device.device_name.clone(), device);
    }

    for mut device in brightnessctl_devices {
        if let Some(existing) = merged.remove(&device.device_name) {
            if device.max_brightness.is_none() {
                device.max_brightness = existing.max_brightness;
            }
            if device.backlight_type.is_none() {
                device.backlight_type = existing.backlight_type;
            }
            if device.sysfs_path.is_empty() {
                device.sysfs_path = existing.sysfs_path;
            }
            device.backend_viable = device.backend_viable || existing.backend_viable;
            if device.note.is_none() {
                device.note = existing.note;
            }
        }
        merged.insert(device.device_name.clone(), device);
    }

    merged.into_values().collect()
}

fn parse_display_header(line: &str) -> Option<u32> {
    line.strip_prefix("Display ")?.trim().parse::<u32>().ok()
}

fn parse_bus_number(value: &str) -> Option<u32> {
    value.rsplit('-').next()?.trim().parse::<u32>().ok()
}

fn parse_monitor_identity(value: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut parts = value.splitn(3, ':');
    let manufacturer = parts.next().and_then(normalize_optional);
    let model = parts.next().and_then(normalize_optional);
    let serial = parts.next().and_then(normalize_optional);
    (manufacturer, model, serial)
}

fn normalize_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn read_optional_u32(path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn read_optional_text(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let value = raw.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
