use serde::Serialize;

use crate::backends::BackendKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    InternalPanel,
    ExternalMonitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalIdentityStatus {
    Unique,
    Ambiguous,
    TopologyBound,
    #[default]
    Insufficient,
}

#[derive(Debug, Clone)]
pub struct TargetDescriptor {
    pub id: String,
    pub label: String,
    pub kind: TargetKind,
    pub backend: BackendKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendStatusKind {
    Ok,
    Missing,
    Timeout,
    Error,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendStatus {
    pub backend: String,
    pub status: BackendStatusKind,
    pub available: bool,
    pub message: String,
    pub guidance: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryBackends {
    pub ddcutil: BackendStatus,
    pub brightnessctl: BackendStatus,
    pub sysfs: BackendStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoverySummary {
    pub ddc_monitors: usize,
    pub backlight_devices: usize,
    pub viable_targets: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DdcMonitorDiscovery {
    /// Unique only within one discovery report; not a physical identity.
    #[serde(default)]
    pub occurrence_id: String,
    pub stable_id: String,
    #[serde(default)]
    pub identity_status: PhysicalIdentityStatus,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub display_number: u32,
    pub bus_number: Option<u32>,
    pub connector: Option<String>,
    pub brightness_vcp_supported: Option<bool>,
    pub backend_viable: bool,
    pub note: Option<String>,
}

impl DdcMonitorDiscovery {
    pub(crate) fn target_label(&self) -> String {
        let mut parts = Vec::new();
        if let Some(manufacturer) = &self.manufacturer {
            parts.push(manufacturer.clone());
        }
        if let Some(model) = &self.model {
            parts.push(model.clone());
        }
        if let Some(serial) = &self.serial {
            parts.push(format!("#{serial}"));
        }

        if parts.is_empty() {
            format!("Display {}", self.display_number)
        } else {
            parts.join(" ")
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklightDeviceDiscovery {
    pub stable_id: String,
    pub device_name: String,
    pub class: String,
    pub max_brightness: Option<u32>,
    /// Kernel `/sys/class/backlight/<device>/type`, when readable.
    pub backlight_type: Option<String>,
    pub probe_source: String,
    pub sysfs_path: String,
    /// Topology evidence: DRM connector directory name (e.g. `card1-DP-1`)
    /// whose `ddcci_backlight` symlink converges on this device's `device`
    /// link. `Some` only when the kernel/driver topology link is proven;
    /// never inferred from names, ordering, or backlight type.
    #[serde(default)]
    pub ddcci_connector: Option<String>,
    pub backend_viable: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct WindowsDisplayDiscovery {
    pub display_number: u32,
    pub canonical_id: Option<String>,
    pub display_label: Option<String>,
    pub stable_id: Option<String>,
    pub identity_quality: String,
    pub mutation_eligibility: String,
    pub name: Option<String>,
    pub kind: String,
    pub active: bool,
    pub target_available: bool,
    pub is_primary: bool,
    pub output_technology: String,
    pub gdi_device_name: Option<String>,
    pub adapter_luid: String,
    pub source_id: u32,
    pub target_id: u32,
    pub connector_instance: u32,
    pub container_id: Option<String>,
    pub device_instance_id: Option<String>,
    pub hardware_ids: Vec<String>,
    pub edid_manufacturer: Option<String>,
    pub edid_product_code: Option<u16>,
    pub physical_monitors_count: u32,
    pub physical_monitor_description: Option<String>,
    pub desktop_rect: Option<String>,
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brightness_capable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_current: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_max: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_current_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryReport {
    pub summary: DiscoverySummary,
    pub backends: DiscoveryBackends,
    /// False when ddcutil emitted an invalid/partial detection record. Such a
    /// scan may describe some displays correctly, but is not authoritative
    /// enough for automatic monitor onboarding.
    #[serde(default)]
    pub ddc_observation_complete: bool,
    pub ddc_monitors: Vec<DdcMonitorDiscovery>,
    pub backlight_devices: Vec<BacklightDeviceDiscovery>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows_displays: Vec<WindowsDisplayDiscovery>,
    pub notes: Vec<String>,
    pub config_snippet: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoverySnapshot {
    pub(crate) summary: DiscoverySummary,
    pub(crate) backends: DiscoveryBackends,
    pub(crate) ddc_observation_complete: bool,
    pub(crate) ddc_monitors: Vec<DdcMonitorDiscovery>,
    pub(crate) backlight_devices: Vec<BacklightDeviceDiscovery>,
}

#[derive(Debug)]
pub(crate) struct RawDdcMonitor {
    pub(crate) manufacturer: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) serial: Option<String>,
    pub(crate) display_number: u32,
    pub(crate) bus_number: Option<u32>,
    pub(crate) connector: Option<String>,
}

impl RawDdcMonitor {
    pub(crate) fn new(display_number: u32) -> Self {
        Self {
            manufacturer: None,
            model: None,
            serial: None,
            display_number,
            bus_number: None,
            connector: None,
        }
    }

    pub(crate) fn into_discovery(self) -> DdcMonitorDiscovery {
        DdcMonitorDiscovery {
            occurrence_id: String::new(),
            stable_id: build_ddc_stable_id(&self),
            identity_status: PhysicalIdentityStatus::Insufficient,
            manufacturer: self.manufacturer,
            model: self.model,
            serial: self.serial,
            display_number: self.display_number,
            bus_number: self.bus_number,
            connector: self.connector,
            brightness_vcp_supported: None,
            backend_viable: false,
            note: None,
        }
    }
}

pub(crate) fn build_backlight_stable_id(device_name: &str) -> String {
    let slug = slugify(device_name);
    if slug.is_empty() {
        String::from("backlight:device")
    } else {
        format!("backlight:{slug}")
    }
}

pub(crate) fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash && !slug.is_empty() {
            slug.push('-');
            previous_was_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

fn build_ddc_stable_id(raw: &RawDdcMonitor) -> String {
    let mut parts = vec![String::from("ddc")];

    if let Some(manufacturer) = raw.manufacturer.as_deref() {
        let slug = slugify(manufacturer);
        if !slug.is_empty() {
            parts.push(slug);
        }
    }

    if let Some(model) = raw.model.as_deref() {
        let slug = slugify(model);
        if !slug.is_empty() {
            parts.push(slug);
        }
    }

    if let Some(serial) = raw.serial.as_deref() {
        let slug = slugify(serial);
        if !slug.is_empty() {
            parts.push(slug);
        }
    } else if let Some(bus_number) = raw.bus_number {
        parts.push(format!("bus-{bus_number}"));
    } else {
        parts.push(format!("display-{}", raw.display_number));
    }

    parts.join(":")
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
#[must_use]
pub fn windows_discovery_deferred_report() -> DiscoveryReport {
    DiscoveryReport {
        summary: DiscoverySummary {
            ddc_monitors: 0,
            backlight_devices: 0,
            viable_targets: 0,
        },
        ddc_observation_complete: false,
        backends: DiscoveryBackends {
            ddcutil: BackendStatus {
                backend: String::from("ddcutil"),
                available: false,
                status: BackendStatusKind::Unavailable,
                message: String::from("DDC monitor discovery on Windows is deferred to Phase W2"),
                guidance: None,
            },
            brightnessctl: BackendStatus {
                backend: String::from("brightnessctl"),
                available: false,
                status: BackendStatusKind::Unavailable,
                message: String::from(
                    "brightnessctl is Linux-specific; native Windows backlight deferred to W2",
                ),
                guidance: None,
            },
            sysfs: BackendStatus {
                backend: String::from("sysfs"),
                available: false,
                status: BackendStatusKind::Unavailable,
                message: String::from(
                    "sysfs is Linux-specific; native Windows display topology deferred to W2",
                ),
                guidance: None,
            },
        },
        ddc_monitors: Vec::new(),
        backlight_devices: Vec::new(),
        windows_displays: Vec::new(),
        notes: vec![String::from(
            "Windows monitor discovery is deferred to W2; hardware enumeration not supported in W1",
        )],
        config_snippet: String::new(),
    }
}
