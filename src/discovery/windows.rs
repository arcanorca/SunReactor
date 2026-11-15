//! Windows display discovery: CCD/SetupAPI topology plus DXVA2 brightness
//! capability probes, mapped onto the platform-neutral discovery report.

use super::model::{
    self, BackendStatus, BackendStatusKind, BacklightDeviceDiscovery, DdcMonitorDiscovery,
    DiscoveryBackends, DiscoveryReport, DiscoverySummary, PhysicalIdentityStatus,
};
use crate::platform::windows::brightness::{
    probe_external_capability_with_api, ExternalBrightnessCapability, Win32MonitorConfigApi,
};
use crate::platform::windows::display::WindowsDisplayKind;
use crate::platform::windows::topology::{
    capture_topology_snapshot, IdentityQuality, WindowsMonitorObservation,
};

#[must_use]
pub fn windows_discover_report() -> DiscoveryReport {
    let snapshot = match capture_topology_snapshot() {
        Ok(s) => s,
        Err(e) => return topology_error_report(&e),
    };

    let mut ddc_monitors = Vec::new();
    let mut backlight_devices = Vec::new();
    let mut windows_displays = Vec::new();

    for mon in &snapshot.monitors {
        let brightness = BrightnessView::probe(mon);
        windows_displays.push(display_discovery(mon, &brightness));
        if mon.kind == WindowsDisplayKind::ExternalMonitor {
            ddc_monitors.push(external_monitor_discovery(mon, brightness));
        } else if mon.kind == WindowsDisplayKind::InternalPanel {
            backlight_devices.push(internal_panel_discovery(mon));
        }
    }

    let summary = DiscoverySummary {
        ddc_monitors: ddc_monitors.len(),
        backlight_devices: backlight_devices.len(),
        viable_targets: ddc_monitors.iter().filter(|m| m.backend_viable).count()
            + backlight_devices
                .iter()
                .filter(|b| b.backend_viable)
                .count(),
    };

    let mut notes = snapshot.notes;
    notes.push(String::from(
        "SunReactor Windows W2: Read-only display topology & stable identity qualification active.",
    ));

    let config_snippet = generate_candidate_windows_config(&snapshot.monitors);

    DiscoveryReport {
        summary,
        backends: active_backends(),
        ddc_observation_complete: true,
        ddc_monitors,
        backlight_devices,
        windows_displays,
        notes,
        config_snippet,
    }
}

/// The report returned when the topology snapshot itself fails: every backend
/// unavailable, no targets.
fn topology_error_report(e: &impl std::fmt::Display) -> DiscoveryReport {
    DiscoveryReport {
        summary: DiscoverySummary {
            ddc_monitors: 0,
            backlight_devices: 0,
            viable_targets: 0,
        },
        backends: DiscoveryBackends {
            ddcutil: BackendStatus {
                backend: String::from("DXVA2 / DDC"),
                available: false,
                status: BackendStatusKind::Error,
                message: format!("Topology query failed: {e}"),
                guidance: None,
            },
            brightnessctl: BackendStatus {
                backend: String::from("WMI / WinRT"),
                available: false,
                status: BackendStatusKind::Unavailable,
                message: String::from("Internal panel brightness deferred to W3"),
                guidance: None,
            },
            sysfs: BackendStatus {
                backend: String::from("CCD / SetupAPI"),
                available: false,
                status: BackendStatusKind::Error,
                message: format!("CCD failure: {e}"),
                guidance: None,
            },
        },
        ddc_monitors: Vec::new(),
        backlight_devices: Vec::new(),
        ddc_observation_complete: false,
        windows_displays: Vec::new(),
        notes: vec![format!("Error discovering Windows display topology: {e}")],
        config_snippet: String::new(),
    }
}

fn active_backends() -> DiscoveryBackends {
    DiscoveryBackends {
        ddcutil: BackendStatus {
            backend: String::from("DXVA2 / DDC"),
            available: true,
            status: BackendStatusKind::Ok,
            message: String::from(
                "Native Windows physical monitor enumeration active (mutation deferred to W3)",
            ),
            guidance: None,
        },
        brightnessctl: BackendStatus {
            backend: String::from("WMI / WinRT"),
            available: false,
            status: BackendStatusKind::Unavailable,
            message: String::from("Internal backlight control deferred to W3"),
            guidance: None,
        },
        sysfs: BackendStatus {
            backend: String::from("CCD / SetupAPI"),
            available: true,
            status: BackendStatusKind::Ok,
            message: String::from("Native Windows CCD display topology & PnP devnodes active"),
            guidance: None,
        },
    }
}

/// What discovery reports about one display's brightness control.
struct BrightnessView {
    capable: bool,
    native_min: Option<u32>,
    native_current: Option<u32>,
    native_max: Option<u32>,
    normalized_pct: Option<u8>,
    note: String,
}

impl BrightnessView {
    fn unsupported(note: String) -> Self {
        Self {
            capable: false,
            native_min: None,
            native_current: None,
            native_max: None,
            normalized_pct: None,
            note,
        }
    }

    /// Only external monitors with a canonical id are probed over DXVA2.
    fn probe(mon: &WindowsMonitorObservation) -> Self {
        match mon.kind {
            WindowsDisplayKind::ExternalMonitor => match mon.canonical_id {
                Some(ref cid) => Self::from_capability(probe_external_capability_with_api(
                    &Win32MonitorConfigApi,
                    cid,
                )),
                None => Self::unsupported(String::from("No canonical ID")),
            },
            WindowsDisplayKind::InternalPanel => {
                Self::unsupported(String::from("Internal panel brightness deferred to W3B"))
            }
            _ => Self::unsupported(format!("{} display (DDC unsupported)", mon.kind)),
        }
    }

    fn from_capability(capability: ExternalBrightnessCapability) -> Self {
        match capability {
            ExternalBrightnessCapability::Supported {
                range,
                normalized_current_pct,
            } => Self {
                capable: true,
                native_min: Some(range.min),
                native_current: Some(range.current),
                native_max: Some(range.max),
                normalized_pct: Some(normalized_current_pct),
                note: String::from("DXVA2 High-Level API / Unique physical mapping"),
            },
            ExternalBrightnessCapability::Unsupported { reason }
            | ExternalBrightnessCapability::TemporarilyUnavailable { reason } => {
                Self::unsupported(reason)
            }
            ExternalBrightnessCapability::InvalidRange { min, current, max } => Self {
                native_min: Some(min),
                native_current: Some(current),
                native_max: Some(max),
                ..Self::unsupported(String::from("Invalid native brightness range"))
            },
            ExternalBrightnessCapability::AmbiguousPhysicalMapping { count } => {
                Self::unsupported(format!("Ambiguous physical mapping: {count} handles"))
            }
            ExternalBrightnessCapability::IdentityNotMutationEligible { quality } => {
                Self::unsupported(format!("Identity not mutation eligible ({quality})"))
            }
            ExternalBrightnessCapability::TopologyUnavailable => {
                Self::unsupported(String::from("Topology unavailable"))
            }
            ExternalBrightnessCapability::NativeApiFailure { operation, code } => {
                Self::unsupported(format!("{operation} failed with Win32 error {code}"))
            }
        }
    }
}

fn display_discovery(
    mon: &WindowsMonitorObservation,
    brightness: &BrightnessView,
) -> model::WindowsDisplayDiscovery {
    model::WindowsDisplayDiscovery {
        display_number: mon.display_number,
        canonical_id: mon.canonical_id.clone(),
        display_label: mon.display_label.clone(),
        stable_id: mon.canonical_id.clone(),
        identity_quality: mon.identity_quality.to_string(),
        mutation_eligibility: mon.mutation_eligibility.to_string(),
        name: mon.name.clone(),
        kind: mon.kind.to_string(),
        active: mon.active,
        target_available: mon.target_available,
        is_primary: mon.is_primary,
        output_technology: mon.output_technology.clone(),
        gdi_device_name: mon.gdi_device_name.clone(),
        adapter_luid: format!("{:x}_{:x}", mon.adapter_luid.0, mon.adapter_luid.1),
        source_id: mon.source_id,
        target_id: mon.target_id,
        connector_instance: mon.connector_instance,
        container_id: mon.container_id.clone(),
        device_instance_id: mon.device_instance_id.clone(),
        hardware_ids: mon.hardware_ids.clone(),
        edid_manufacturer: mon.edid_manufacturer.clone(),
        edid_product_code: mon.edid_product_code,
        physical_monitors_count: mon.physical_monitors_count,
        physical_monitor_description: mon.physical_monitor_description.clone(),
        desktop_rect: mon
            .desktop_rect
            .map(|(l, t, r, b)| format!("{}x{} @ {},{}", r - l, b - t, l, t)),
        note: mon.note.clone(),
        brightness_capable: Some(brightness.capable),
        native_min: brightness.native_min,
        native_current: brightness.native_current,
        native_max: brightness.native_max,
        normalized_current_percent: brightness.normalized_pct,
        capability_note: Some(brightness.note.clone()),
    }
}

fn external_monitor_discovery(
    mon: &WindowsMonitorObservation,
    brightness: BrightnessView,
) -> DdcMonitorDiscovery {
    let identity_status = match mon.identity_quality {
        IdentityQuality::Strong | IdentityQuality::StableDevice => PhysicalIdentityStatus::Unique,
        IdentityQuality::Ambiguous => PhysicalIdentityStatus::Ambiguous,
        IdentityQuality::PortBound => PhysicalIdentityStatus::TopologyBound,
        _ => PhysicalIdentityStatus::Insufficient,
    };
    DdcMonitorDiscovery {
        occurrence_id: format!("win-disp-{}", mon.display_number),
        stable_id: mon
            .stable_id
            .clone()
            .unwrap_or_else(|| format!("win-disp-{}", mon.display_number)),
        identity_status,
        manufacturer: mon.edid_manufacturer.clone(),
        model: mon.name.clone(),
        serial: mon.container_id.clone(),
        display_number: mon.display_number,
        bus_number: None,
        connector: Some(mon.output_technology.clone()),
        brightness_vcp_supported: Some(brightness.capable),
        backend_viable: brightness.capable && mon.active && mon.target_available,
        // The capability note is always present, so it always wins over the
        // topology note.
        note: Some(brightness.note),
    }
}

fn internal_panel_discovery(mon: &WindowsMonitorObservation) -> BacklightDeviceDiscovery {
    BacklightDeviceDiscovery {
        stable_id: mon
            .stable_id
            .clone()
            .unwrap_or_else(|| format!("win-panel-{}", mon.display_number)),
        device_name: mon
            .name
            .clone()
            .unwrap_or_else(|| format!("Internal Panel {}", mon.display_number)),
        class: String::from("internal_panel"),
        max_brightness: None,
        backlight_type: Some(String::from("eDP")),
        probe_source: String::from("CCD"),
        sysfs_path: mon.gdi_device_name.clone().unwrap_or_default(),
        ddcci_connector: Some(mon.output_technology.clone()),
        backend_viable: mon.active && mon.target_available,
        note: mon.note.clone(),
    }
}

fn generate_candidate_windows_config(
    monitors: &[crate::platform::windows::topology::WindowsMonitorObservation],
) -> String {
    use std::fmt::Write;
    let mut snippet = String::from(
        "# Candidate Windows monitor configuration generated by sunreactorctl discover\n",
    );
    let mut candidate_count = 0;
    for mon in monitors {
        match mon.identity_quality {
            crate::platform::windows::topology::IdentityQuality::Strong => {
                if let Some(ref id) = mon.canonical_id {
                    let label = mon.name.as_deref().unwrap_or("Display");
                    let _ = writeln!(
                        snippet,
                        "\n# [[monitors]]\n# id = \"{id}\"\n# name = \"{label}\"\n# min_brightness = 10\n# max_brightness = 100"
                    );
                    candidate_count += 1;
                }
            }
            crate::platform::windows::topology::IdentityQuality::StableDevice => {
                if let Some(ref id) = mon.canonical_id {
                    let label = mon.name.as_deref().unwrap_or("Display");
                    let _ = writeln!(
                        snippet,
                        "\n# Qualified device-bound candidate (port independence unverified):\n# [[monitors]]\n# id = \"{id}\"\n# name = \"{label}\"\n# min_brightness = 10\n# max_brightness = 100"
                    );
                    candidate_count += 1;
                }
            }
            _ => {}
        }
    }
    if candidate_count == 0 {
        snippet.push_str(
            "\n# No monitors with Strong or StableDevice identity detected.\n# Ineligible monitors (PortBound, Ambiguous, Virtual, Unknown) are excluded from candidate config.\n",
        );
    }
    snippet
}
