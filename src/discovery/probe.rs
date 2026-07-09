use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::model::{
    build_backlight_stable_id, BackendStatus, BackendStatusKind, BacklightDeviceDiscovery,
    DdcMonitorDiscovery, DiscoveryBackends, DiscoverySnapshot, DiscoverySummary, RawDdcMonitor,
};
use super::runner::{command_failure_detail, CommandError, ProcessRunner};

const DDCUTIL_DETECT_TIMEOUT: Duration = Duration::from_secs(4);
const DDCUTIL_CAPABILITIES_TIMEOUT: Duration = Duration::from_secs(3);
const BRIGHTNESSCTL_LIST_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn discover_with_runner<R: ProcessRunner>(
    runner: &R,
    sysfs_root: &Path,
) -> DiscoverySnapshot {
    let (ddc_status, ddc_monitors) = discover_ddc_monitors(runner);
    let (brightnessctl_status, brightnessctl_devices) =
        discover_brightnessctl_backlights(runner, sysfs_root);
    let (sysfs_status, sysfs_devices) = discover_sysfs_backlights(sysfs_root);
    let mut backlight_devices = merge_backlight_devices(brightnessctl_devices, sysfs_devices);

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
                .filter(|device| device.backend_viable)
                .count(),
    };

    DiscoverySnapshot {
        summary,
        backends: DiscoveryBackends {
            ddcutil: ddc_status,
            brightnessctl: brightnessctl_status,
            sysfs: sysfs_status,
        },
        ddc_monitors,
        backlight_devices,
    }
}

fn discover_ddc_monitors<R: ProcessRunner>(
    runner: &R,
) -> (BackendStatus, Vec<DdcMonitorDiscovery>) {
    let args = vec![
        String::from("--noconfig"),
        String::from("--terse"),
        String::from("detect"),
    ];

    match runner.run("ddcutil", &args, DDCUTIL_DETECT_TIMEOUT) {
        Ok(output) if output.success() => {
            let mut monitors = parse_ddc_detect(&output.stdout);
            let mut capability_failures = 0usize;

            for monitor in &mut monitors {
                let capability_args = vec![
                    String::from("--noconfig"),
                    String::from("--display"),
                    monitor.display_number.to_string(),
                    String::from("capabilities"),
                ];

                match runner.run("ddcutil", &capability_args, DDCUTIL_CAPABILITIES_TIMEOUT) {
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
            )
        }
        Ok(output) => (
            BackendStatus {
                backend: String::from("ddcutil"),
                status: BackendStatusKind::Error,
                available: true,
                message: format!("ddcutil detect failed: {}", command_failure_detail(&output)),
                guidance: Some(String::from(
                    "Ensure the user can access the relevant /dev/i2c-* devices and rerun discovery.",
                )),
            },
            Vec::new(),
        ),
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
        ),
    }
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
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(device_name) = entry.file_name().into_string().ok() else {
            continue;
        };

        let max_brightness = read_optional_u32(&path.join("max_brightness"));
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
            probe_source: String::from("sysfs"),
            sysfs_path: path.display().to_string(),
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
            probe_source: String::from("brightnessctl"),
            sysfs_path: sysfs_path.display().to_string(),
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
