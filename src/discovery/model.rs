use serde::Serialize;

use crate::backends::BackendKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    InternalPanel,
    ExternalMonitor,
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
    pub stable_id: String,
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
    pub probe_source: String,
    pub sysfs_path: String,
    pub backend_viable: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryReport {
    pub summary: DiscoverySummary,
    pub backends: DiscoveryBackends,
    pub ddc_monitors: Vec<DdcMonitorDiscovery>,
    pub backlight_devices: Vec<BacklightDeviceDiscovery>,
    pub notes: Vec<String>,
    pub config_snippet: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoverySnapshot {
    pub(crate) summary: DiscoverySummary,
    pub(crate) backends: DiscoveryBackends,
    pub(crate) ddc_monitors: Vec<DdcMonitorDiscovery>,
    pub(crate) backlight_devices: Vec<BacklightDeviceDiscovery>,
}

#[derive(Debug)]
pub struct RawDdcMonitor {
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
            stable_id: build_ddc_stable_id(&self),
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
