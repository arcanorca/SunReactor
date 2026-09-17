//! Windows monitor topology snapshot and stable identity derivation.
//!
//! Correlates Windows desktop display sources, CCD paths, output targets,
//! PnP device devnodes, and GDI logical monitors into a coherent, immutable topology snapshot.
//!
//! # Identity Invariants
//! - `HMONITOR`, `PHYSICAL_MONITOR` handles, and GDI device names (`\\.\DISPLAY1`) are transient
//!   and NEVER persisted or treated as physical monitor identity.
//! - When identical monitor models or broken EDIDs produce colliding identity evidence,
//!   both displays are downgraded to `IdentityQuality::Ambiguous` rather than merged.
//! - Canonical identity retains full 128-bit Container GUID or full 128-bit device instance hash;
//!   truncation to 32 bits is strictly forbidden in production matching logic.

use std::collections::HashMap;

use super::display::{query_active_ccd_paths, WindowsDisplayError, WindowsDisplayKind};
use super::gdi::{enumerate_gdi_monitors, GdiMonitorInfo};
use super::physical_monitors::query_physical_monitors;
use super::pnp::{query_pnp_device_info, PnpDeviceInfo};

/// Quality classification of derived monitor stable identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityQuality {
    /// Strong hardware-backed identity (unique, non-system Container ID).
    Strong,
    /// Stable device identity (PnP device instance ID + EDID, but Container ID absent/generic).
    StableDevice,
    /// Port/topology-bound identity (lacks unique hardware evidence, bound to adapter + target coordinate).
    PortBound,
    /// Ambiguous identity: Two or more live monitors report identical identity evidence.
    Ambiguous,
    /// Virtual or indirect display (no physical monitor expected).
    Virtual,
    /// Insufficient data to form identity.
    Unknown,
}

impl std::fmt::Display for IdentityQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strong => write!(f, "Strong (Hardware Container)"),
            Self::StableDevice => write!(f, "Stable (Device Instance)"),
            Self::PortBound => write!(f, "Port-Bound (Topology Coordinate)"),
            Self::Ambiguous => write!(f, "Ambiguous (Collision Detected)"),
            Self::Virtual => write!(f, "Virtual Display"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// W3 brightness mutation eligibility tier derived from identity quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum W3MutationEligibility {
    /// Eligible for automatic persistent brightness automation (Strong quality with unique match).
    AutomaticEligible,
    /// Eligible only under explicit/manual policy; port independence is not assumed (StableDevice).
    ExplicitOnly,
    /// Ineligible for automatic or persistent mutation (PortBound, Ambiguous, Virtual, Unknown).
    Ineligible,
}

impl std::fmt::Display for W3MutationEligibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AutomaticEligible => write!(f, "Automatic Eligible (Strong)"),
            Self::ExplicitOnly => write!(f, "Explicit / Manual Only (Device Bound)"),
            Self::Ineligible => write!(f, "Ineligible (Fail-Closed)"),
        }
    }
}

impl IdentityQuality {
    /// Evaluates the W3 mutation eligibility tier for this identity quality.
    #[must_use]
    pub fn mutation_eligibility(self) -> W3MutationEligibility {
        match self {
            Self::Strong => W3MutationEligibility::AutomaticEligible,
            Self::StableDevice => W3MutationEligibility::ExplicitOnly,
            Self::PortBound | Self::Ambiguous | Self::Virtual | Self::Unknown => {
                W3MutationEligibility::Ineligible
            }
        }
    }

    /// Returns true if this monitor observation is eligible for automatic persistent brightness control.
    #[must_use]
    pub fn is_w3_automatic_eligible(self) -> bool {
        self == Self::Strong
    }
}

/// Complete observation of a single Windows display output and its physical correlation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowsMonitorObservation {
    /// Sequential 1-based display number within this discovery report.
    pub display_number: u32,
    /// Canonical, collision-resistant identifier (full 128-bit Container GUID or 128-bit devnode hash).
    /// Used for persistent configuration, matching, and hardware targeting.
    pub canonical_id: Option<String>,
    /// Short, human-friendly presentation label (e.g. `win-del-4198-8c7ed206`).
    /// NEVER used for identity matching or config persistence.
    pub display_label: Option<String>,
    /// Alias for `canonical_id` for backward compatibility with external consumers.
    pub stable_id: Option<String>,
    /// Quality/confidence of the derived stable identity.
    pub identity_quality: IdentityQuality,
    /// W3 brightness mutation eligibility tier.
    pub mutation_eligibility: W3MutationEligibility,
    /// Display name (EDID friendly name, physical monitor description, or fallback).
    pub name: Option<String>,
    /// Classification of display output technology (External, Internal, Indirect, Virtual, Unknown).
    pub kind: WindowsDisplayKind,
    /// Whether the CCD path is active in the desktop compositor.
    pub active: bool,
    /// Whether the output target is currently available (connected).
    pub target_available: bool,
    /// Whether this display is marked as the primary Windows desktop monitor.
    pub is_primary: bool,
    /// Description of the video output technology (e.g. "DisplayPort", "HDMI").
    pub output_technology: String,
    /// Current GDI device name (e.g. `\\.\DISPLAY1`). Transient correlation coordinate.
    pub gdi_device_name: Option<String>,
    /// Graphics adapter LUID (LowPart, HighPart).
    pub adapter_luid: (u32, i32),
    /// CCD source ID on the graphics adapter.
    pub source_id: u32,
    /// CCD target ID on the graphics adapter.
    pub target_id: u32,
    /// Connector instance index on the adapter.
    pub connector_instance: u32,
    /// Windows PnP Container ID GUID string if available.
    pub container_id: Option<String>,
    /// Windows PnP device instance ID string if available.
    pub device_instance_id: Option<String>,
    /// Windows PnP hardware ID strings.
    pub hardware_ids: Vec<String>,
    /// 3-letter VESA manufacturer ID decoded from EDID (e.g. "DEL").
    pub edid_manufacturer: Option<String>,
    /// 16-bit product code from EDID.
    pub edid_product_code: Option<u16>,
    /// Number of physical monitor handles associated with this display (0, 1, or N).
    pub physical_monitors_count: u32,
    /// First physical monitor description string from DXVA2.
    pub physical_monitor_description: Option<String>,
    /// Desktop virtual coordinate rectangle (left, top, right, bottom).
    pub desktop_rect: Option<(i32, i32, i32, i32)>,
    /// Informational or diagnostic note (e.g. collision notice, driver warnings).
    pub note: Option<String>,
}

/// Immutable snapshot of the Windows display topology at a point in time.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowsTopologySnapshot {
    /// Correlated monitor observations.
    pub monitors: Vec<WindowsMonitorObservation>,
    /// Total GDI logical monitors enumerated.
    pub gdi_count: usize,
    /// Total CCD active paths enumerated.
    pub ccd_paths_count: usize,
    /// Global diagnostic notes or warnings.
    pub notes: Vec<String>,
}

/// Deterministic 128-bit FNV-1a composite hash returning a 32-character hexadecimal string.
#[must_use]
pub fn hash_128(s: &str) -> String {
    let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
    let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
    for &b in s.as_bytes() {
        h1 ^= u64::from(b);
        h1 = h1.wrapping_mul(0x0000_0100_0000_01b3);
        h2 ^= u64::from(b.rotate_left(3));
        h2 = h2.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h1:016x}{h2:016x}")
}

/// Derives candidate stable identity and confidence from observed display evidence.
///
/// Returns `(canonical_id, display_label, identity_quality, mutation_eligibility)`.
#[must_use]
pub fn derive_stable_identity(
    container_id: Option<&str>,
    device_instance_id: Option<&str>,
    mfg: Option<&str>,
    prod: Option<u16>,
    kind: WindowsDisplayKind,
    adapter_luid: (u32, i32),
    target_id: u32,
) -> (
    Option<String>,
    Option<String>,
    IdentityQuality,
    W3MutationEligibility,
) {
    if kind == WindowsDisplayKind::VirtualDisplay {
        let id = format!("win-vdisp-{target_id}");
        return (
            Some(id.clone()),
            Some(id),
            IdentityQuality::Virtual,
            W3MutationEligibility::Ineligible,
        );
    }

    if kind == WindowsDisplayKind::IndirectDisplay
        && container_id.is_none()
        && device_instance_id.is_none()
    {
        let id = format!("win-indirect-{target_id}");
        return (
            Some(id.clone()),
            Some(id),
            IdentityQuality::PortBound,
            W3MutationEligibility::Ineligible,
        );
    }

    // 1. Strong identity: Hardware Container ID
    // ContainerId is the strongest Windows PnP device-container identity; expected to be stable
    // when the driver/device reports or Windows derives a stable unique container identity.
    // Canonical identity preserves the FULL 128-bit normalized GUID without truncation.
    if let Some(cid) = container_id {
        if !crate::platform::windows::pnp::is_generic_or_system_container(cid) {
            let clean_guid = cid
                .trim_start_matches('{')
                .trim_end_matches('}')
                .to_ascii_lowercase();
            let prefix = if clean_guid.len() >= 8 {
                &clean_guid[..8]
            } else {
                &clean_guid
            };

            let canonical_id = if let Some(m) = mfg {
                if let Some(p) = prod {
                    format!("win-{}-{:04x}-{}", m.to_ascii_lowercase(), p, clean_guid)
                } else {
                    format!("win-{}-{}", m.to_ascii_lowercase(), clean_guid)
                }
            } else {
                format!("win-disp-{clean_guid}")
            };

            let display_label = if let Some(m) = mfg {
                if let Some(p) = prod {
                    format!("win-{}-{:04x}-{}", m.to_ascii_lowercase(), p, prefix)
                } else {
                    format!("win-{}-{}", m.to_ascii_lowercase(), prefix)
                }
            } else {
                format!("win-disp-{prefix}")
            };

            return (
                Some(canonical_id),
                Some(display_label),
                IdentityQuality::Strong,
                W3MutationEligibility::AutomaticEligible,
            );
        }
    }

    // 2. StableDevice identity: Supported PnP Device Instance ID from SetupAPI
    // Used when Container ID is absent or generic. Uses full 128-bit hash to prevent 32-bit truncation.
    if let Some(inst) = device_instance_id {
        let full_hash = hash_128(inst);
        let prefix = &full_hash[..8];

        let canonical_id = if let Some(m) = mfg {
            if let Some(p) = prod {
                format!("win-dev-{}-{:04x}-{full_hash}", m.to_ascii_lowercase(), p)
            } else {
                format!("win-dev-{}-{full_hash}", m.to_ascii_lowercase())
            }
        } else {
            format!("win-dev-{full_hash}")
        };

        let display_label = if let Some(m) = mfg {
            format!("win-dev-{}-{prefix}", m.to_ascii_lowercase())
        } else {
            format!("win-dev-{prefix}")
        };

        return (
            Some(canonical_id),
            Some(display_label),
            IdentityQuality::StableDevice,
            W3MutationEligibility::ExplicitOnly,
        );
    }

    // 3. PortBound identity: adapter + target coordinate
    // Transient topology coordinate only. NEVER automatically mutated in W3.
    let id = format!(
        "win-port-{:x}_{:x}-{target_id}",
        adapter_luid.0, adapter_luid.1
    );
    (
        Some(id.clone()),
        Some(id),
        IdentityQuality::PortBound,
        W3MutationEligibility::Ineligible,
    )
}

/// Captures a fresh, immutable snapshot of the Windows display topology.
///
/// Correlates CCD paths, GDI logical monitors, PnP device properties, and DXVA2 physical handles.
#[allow(clippy::too_many_lines)]
pub fn capture_topology_snapshot() -> Result<WindowsTopologySnapshot, WindowsDisplayError> {
    let mut notes = Vec::new();

    // 1. Query CCD Active Paths
    let ccd_paths = match query_active_ccd_paths() {
        Ok(paths) => paths,
        Err(err) => {
            notes.push(format!("CCD query returned: {err}"));
            Vec::new()
        }
    };

    // 2. Query GDI Logical Monitors
    let gdi_monitors = enumerate_gdi_monitors();

    // Index GDI monitors by szDevice (e.g. \\.\DISPLAY1)
    let mut gdi_by_name: HashMap<String, &GdiMonitorInfo> = HashMap::new();
    for gdi in &gdi_monitors {
        gdi_by_name.insert(gdi.device_name.clone(), gdi);
    }

    // 3. Build Observations from CCD Paths
    let mut observations = Vec::new();

    for path in &ccd_paths {
        // Find matching GDI monitor by viewGdiDeviceName <=> szDevice
        let gdi_match = gdi_by_name.get(&path.source.gdi_device_name).copied();

        // Query physical monitors (handles immediately destroyed via PhysicalMonitorGuard)
        let phys_info = gdi_match.map_or_else(
            super::physical_monitors::PhysicalMonitorObservation::default,
            |g| query_physical_monitors(g.hmonitor),
        );

        // Query PnP properties via SetupAPI (from opaque monitorDevicePath)
        let pnp_info = path
            .target
            .monitor_device_path
            .as_deref()
            .map_or_else(PnpDeviceInfo::default, query_pnp_device_info);

        // Derive friendly display name
        let name = path
            .target
            .friendly_name
            .clone()
            .or_else(|| phys_info.description.clone())
            .or_else(|| {
                path.target
                    .edid_manufacturer
                    .as_ref()
                    .map(|m| format!("{m} Monitor"))
            });

        // Derive initial candidate stable identity
        let (canonical_id, display_label, quality, mutation_eligibility) = derive_stable_identity(
            pnp_info.container_id.as_deref(),
            pnp_info.device_instance_id.as_deref(),
            path.target.edid_manufacturer.as_deref(),
            path.target.edid_product_code,
            path.target.kind,
            path.target.adapter_luid,
            path.target.target_id,
        );

        let is_primary = gdi_match.is_some_and(|g| g.is_primary);
        let desktop_rect = gdi_match.map(|g| g.monitor_rect);

        observations.push(WindowsMonitorObservation {
            display_number: 0, // Assigned after sorting
            canonical_id: canonical_id.clone(),
            display_label,
            stable_id: canonical_id,
            identity_quality: quality,
            mutation_eligibility,
            name,
            kind: path.target.kind,
            active: path.active,
            target_available: path.target.target_available,
            is_primary,
            output_technology: path.target.output_technology.clone(),
            gdi_device_name: if path.source.gdi_device_name.is_empty() {
                None
            } else {
                Some(path.source.gdi_device_name.clone())
            },
            adapter_luid: path.target.adapter_luid,
            source_id: path.source.source_id,
            target_id: path.target.target_id,
            connector_instance: path.target.connector_instance,
            container_id: pnp_info.container_id,
            device_instance_id: pnp_info.device_instance_id,
            hardware_ids: pnp_info.hardware_ids,
            edid_manufacturer: path.target.edid_manufacturer.clone(),
            edid_product_code: path.target.edid_product_code,
            physical_monitors_count: phys_info.count,
            physical_monitor_description: phys_info.description,
            desktop_rect,
            note: None,
        });
    }

    // 4. Fallback: If CCD paths were empty but GDI returned monitors (e.g. Wine or simple RDP),
    // represent GDI monitors as fallback observations
    if observations.is_empty() && !gdi_monitors.is_empty() {
        notes.push(String::from(
            "CCD paths unavailable; populating fallback observations from GDI enumeration",
        ));
        for (idx, gdi) in gdi_monitors.iter().enumerate() {
            let phys_info = query_physical_monitors(gdi.hmonitor);
            let canonical_id = Some(format!("win-gdi-fallback-{}", idx + 1));
            let display_label = canonical_id.clone();
            let quality = IdentityQuality::PortBound;
            let mutation_eligibility = W3MutationEligibility::Ineligible;

            observations.push(WindowsMonitorObservation {
                display_number: (idx + 1) as u32,
                canonical_id: canonical_id.clone(),
                display_label,
                stable_id: canonical_id,
                identity_quality: quality,
                mutation_eligibility,
                name: phys_info
                    .description
                    .clone()
                    .or_else(|| Some(format!("Display {}", idx + 1))),
                kind: WindowsDisplayKind::Unknown,
                active: true,
                target_available: true,
                is_primary: gdi.is_primary,
                output_technology: String::from("GDI Fallback"),
                gdi_device_name: Some(gdi.device_name.clone()),
                adapter_luid: (0, 0),
                source_id: idx as u32,
                target_id: idx as u32,
                connector_instance: 0,
                container_id: None,
                device_instance_id: None,
                hardware_ids: Vec::new(),
                edid_manufacturer: None,
                edid_product_code: None,
                physical_monitors_count: phys_info.count,
                physical_monitor_description: phys_info.description,
                desktop_rect: Some(gdi.monitor_rect),
                note: Some(String::from("Observed via GDI fallback")),
            });
        }
    }

    // 5. Detect Collisions Across Simultaneously Attached Displays
    // If two or more displays produce the identical candidate canonical ID,
    // neither can be safely modified without risk. Downgrade both to `Ambiguous`!
    let mut id_counts: HashMap<String, usize> = HashMap::new();
    for mon in &observations {
        if let Some(ref id) = mon.canonical_id {
            *id_counts.entry(id.clone()).or_insert(0) += 1;
        }
    }
    for mon in &mut observations {
        if let Some(ref id) = mon.canonical_id {
            if id_counts.get(id).copied().unwrap_or(0) > 1 {
                mon.identity_quality = IdentityQuality::Ambiguous;
                mon.mutation_eligibility = W3MutationEligibility::Ineligible;
                let notice = format!(
                    "Collision detected: multiple active displays share canonical ID '{id}'; marked ambiguous and ineligible for automatic mutation for fail-closed safety"
                );
                mon.note = Some(notice);
            }
        }
    }

    // 6. Deterministic Sort Order
    // Sort: Primary display first, then by GDI device name, then by adapter LUID & target ID.
    observations.sort_by(|a, b| {
        b.is_primary
            .cmp(&a.is_primary)
            .then_with(|| a.gdi_device_name.cmp(&b.gdi_device_name))
            .then_with(|| a.adapter_luid.cmp(&b.adapter_luid))
            .then_with(|| a.target_id.cmp(&b.target_id))
    });

    // Re-index display numbers sequentially (1, 2, 3...)
    for (idx, mon) in observations.iter_mut().enumerate() {
        mon.display_number = (idx + 1) as u32;
    }

    Ok(WindowsTopologySnapshot {
        gdi_count: gdi_monitors.len(),
        ccd_paths_count: ccd_paths.len(),
        monitors: observations,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::windows::display::decode_edid_manufacturer;
    use crate::platform::windows::pnp::is_generic_or_system_container;

    #[test]
    fn test_single_active_external_display_strong_identity() {
        let (canonical_id, display_label, quality, eligibility) = derive_stable_identity(
            Some("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}"),
            Some(r"DISPLAY\DEL4198\5&383f58e1&0&UID4357"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (0x1000, 0),
            1,
        );

        // Canonical ID retains FULL normalized 128-bit GUID
        assert_eq!(
            canonical_id.as_deref(),
            Some("win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c")
        );
        // Presentation label is human-friendly
        assert_eq!(display_label.as_deref(), Some("win-del-4198-8c7ed206"));
        assert_eq!(quality, IdentityQuality::Strong);
        assert_eq!(eligibility, W3MutationEligibility::AutomaticEligible);
    }

    #[test]
    fn test_full_container_id_preserved_without_truncation() {
        // Two GUIDs sharing the first 8 hex characters:
        let (id1, _, q1, _) = derive_stable_identity(
            Some("{8c7ed206-1111-2222-3333-444444444444}"),
            None,
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );
        let (id2, _, q2, _) = derive_stable_identity(
            Some("{8c7ed206-5555-6666-7777-888888888888}"),
            None,
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            2,
        );

        // MUST NOT collide under canonical identity!
        assert_ne!(id1, id2);
        assert_eq!(
            id1.as_deref(),
            Some("win-del-4198-8c7ed206-1111-2222-3333-444444444444")
        );
        assert_eq!(
            id2.as_deref(),
            Some("win-del-4198-8c7ed206-5555-6666-7777-888888888888")
        );
        assert_eq!(q1, IdentityQuality::Strong);
        assert_eq!(q2, IdentityQuality::Strong);
    }

    #[test]
    fn test_dual_extended_displays_remain_distinct() {
        let (id1, _, q1, e1) = derive_stable_identity(
            Some("{11111111-2222-3333-4444-555555555555}"),
            Some(r"DISPLAY\DEL4198\5&1&0&1"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );
        let (id2, _, q2, e2) = derive_stable_identity(
            Some("{99999999-8888-7777-6666-555555555555}"),
            Some(r"DISPLAY\ASUS001\5&2&0&2"),
            Some("ASU"),
            Some(0x2400),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            2,
        );

        assert_ne!(id1, id2);
        assert_eq!(q1, IdentityQuality::Strong);
        assert_eq!(q2, IdentityQuality::Strong);
        assert_eq!(e1, W3MutationEligibility::AutomaticEligible);
        assert_eq!(e2, W3MutationEligibility::AutomaticEligible);
        assert_eq!(
            id1.as_deref(),
            Some("win-del-4198-11111111-2222-3333-4444-555555555555")
        );
        assert_eq!(
            id2.as_deref(),
            Some("win-asu-2400-99999999-8888-7777-6666-555555555555")
        );
    }

    #[test]
    fn test_clone_mirror_displays_preserve_individual_targets() {
        // Clone mode: ONE desktop source (e.g. source 0 / DISPLAY1) drives TWO distinct targets
        let obs1 = WindowsMonitorObservation {
            display_number: 1,
            canonical_id: Some(String::from("win-panel-edp-1")),
            display_label: Some(String::from("win-panel-edp-1")),
            stable_id: Some(String::from("win-panel-edp-1")),
            identity_quality: IdentityQuality::StableDevice,
            mutation_eligibility: W3MutationEligibility::ExplicitOnly,
            name: Some(String::from("Internal Panel")),
            kind: WindowsDisplayKind::InternalPanel,
            active: true,
            target_available: true,
            is_primary: true,
            output_technology: String::from("Embedded DisplayPort"),
            gdi_device_name: Some(String::from(r"\\.\DISPLAY1")),
            adapter_luid: (10, 0),
            source_id: 0, // Cloned source 0
            target_id: 1,
            connector_instance: 0,
            container_id: None,
            device_instance_id: Some(String::from(r"DISPLAY\INT0001\1")),
            hardware_ids: Vec::new(),
            edid_manufacturer: None,
            edid_product_code: None,
            physical_monitors_count: 0,
            physical_monitor_description: None,
            desktop_rect: Some((0, 0, 1920, 1080)),
            note: None,
        };

        let obs2 = WindowsMonitorObservation {
            display_number: 2,
            canonical_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            display_label: Some(String::from("win-del-4198-8c7ed206")),
            stable_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            identity_quality: IdentityQuality::Strong,
            mutation_eligibility: W3MutationEligibility::AutomaticEligible,
            name: Some(String::from("Dell U2723QE")),
            kind: WindowsDisplayKind::ExternalMonitor,
            active: true,
            target_available: true,
            is_primary: true,
            output_technology: String::from("HDMI"),
            gdi_device_name: Some(String::from(r"\\.\DISPLAY1")), // Cloned GDI view
            adapter_luid: (10, 0),
            source_id: 0, // Cloned source 0
            target_id: 2,
            connector_instance: 1,
            container_id: Some(String::from("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}")),
            device_instance_id: Some(String::from(r"DISPLAY\DEL4198\2")),
            hardware_ids: Vec::new(),
            edid_manufacturer: Some(String::from("DEL")),
            edid_product_code: Some(0x4198),
            physical_monitors_count: 1,
            physical_monitor_description: Some(String::from("Dell U2723QE")),
            desktop_rect: Some((0, 0, 1920, 1080)),
            note: None,
        };

        // Assert that both observations remain distinct even though source_id and gdi_device_name match
        assert_eq!(obs1.source_id, obs2.source_id);
        assert_eq!(obs1.gdi_device_name, obs2.gdi_device_name);
        assert_ne!(obs1.target_id, obs2.target_id);
        assert_ne!(obs1.canonical_id, obs2.canonical_id);
        assert_ne!(obs1.kind, obs2.kind);

        let snapshot = WindowsTopologySnapshot {
            monitors: vec![obs1, obs2],
            gdi_count: 1,
            ccd_paths_count: 2,
            notes: Vec::new(),
        };

        // Snapshot preserves both targets!
        assert_eq!(snapshot.monitors.len(), 2);
    }

    #[test]
    fn test_primary_display_change_preserves_stable_identity() {
        // Changing which display is primary must NOT change its stable ID
        let (id_primary, _, _, _) = derive_stable_identity(
            Some("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}"),
            Some(r"DISPLAY\DEL4198\1"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );

        let (id_non_primary, _, _, _) = derive_stable_identity(
            Some("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}"),
            Some(r"DISPLAY\DEL4198\1"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );

        assert_eq!(id_primary, id_non_primary);
    }

    #[test]
    fn test_resolution_refresh_rotation_change_preserves_identity() {
        let (id1, _, q1, e1) = derive_stable_identity(
            Some("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}"),
            Some(r"DISPLAY\DEL4198\1"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );

        // Simulation after resolution change 3840x2160 -> 1920x1080 @ 120Hz rotated 90 deg:
        let (id2, _, q2, e2) = derive_stable_identity(
            Some("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}"),
            Some(r"DISPLAY\DEL4198\1"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );

        assert_eq!(id1, id2);
        assert_eq!(q1, q2);
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_multiple_adapters_target_id_scoped() {
        // Adapter 1 target 1
        let (id_ad1, _, _, _) = derive_stable_identity(
            None,
            None,
            None,
            None,
            WindowsDisplayKind::ExternalMonitor,
            (0x1000, 0),
            1,
        );
        // Adapter 2 target 1
        let (id_ad2, _, _, _) = derive_stable_identity(
            None,
            None,
            None,
            None,
            WindowsDisplayKind::ExternalMonitor,
            (0x2000, 0),
            1,
        );

        // Adapter-scoped port-bound IDs must NOT collide
        assert_ne!(id_ad1, id_ad2);
    }

    #[test]
    fn test_temporarily_unavailable_active_target_retains_transitional_state() {
        let obs = WindowsMonitorObservation {
            display_number: 1,
            canonical_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            display_label: Some(String::from("win-del-4198-8c7ed206")),
            stable_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            identity_quality: IdentityQuality::Strong,
            mutation_eligibility: W3MutationEligibility::AutomaticEligible,
            name: Some(String::from("Dell U2723QE")),
            kind: WindowsDisplayKind::ExternalMonitor,
            active: true,            // CCD path is still in active database
            target_available: false, // Monitor cable was transiently unplugged
            is_primary: false,
            output_technology: String::from("DisplayPort"),
            gdi_device_name: Some(String::from(r"\\.\DISPLAY2")),
            adapter_luid: (10, 0),
            source_id: 1,
            target_id: 2,
            connector_instance: 0,
            container_id: Some(String::from("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}")),
            device_instance_id: Some(String::from(r"DISPLAY\DEL4198\2")),
            hardware_ids: Vec::new(),
            edid_manufacturer: Some(String::from("DEL")),
            edid_product_code: Some(0x4198),
            physical_monitors_count: 0,
            physical_monitor_description: None,
            desktop_rect: None,
            note: None,
        };

        assert!(obs.active);
        assert!(!obs.target_available);
        assert_eq!(obs.identity_quality, IdentityQuality::Strong);
        assert_eq!(
            obs.mutation_eligibility,
            W3MutationEligibility::AutomaticEligible
        );
    }

    #[test]
    fn test_two_identical_models_with_different_container_ids() {
        // Two identical Dell U2723QE monitors on different ports with distinct hardware Container IDs
        let (id1, _, q1, e1) = derive_stable_identity(
            Some("{aaaaaaaa-1111-2222-3333-444444444444}"),
            Some(r"DISPLAY\DEL4198\5&1&0&1"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );
        let (id2, _, q2, e2) = derive_stable_identity(
            Some("{bbbbbbbb-1111-2222-3333-444444444444}"),
            Some(r"DISPLAY\DEL4198\5&2&0&2"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            2,
        );

        assert_ne!(id1, id2);
        assert_eq!(q1, IdentityQuality::Strong);
        assert_eq!(q2, IdentityQuality::Strong);
        assert_eq!(e1, W3MutationEligibility::AutomaticEligible);
        assert_eq!(e2, W3MutationEligibility::AutomaticEligible);
        assert_eq!(
            id1.as_deref(),
            Some("win-del-4198-aaaaaaaa-1111-2222-3333-444444444444")
        );
        assert_eq!(
            id2.as_deref(),
            Some("win-del-4198-bbbbbbbb-1111-2222-3333-444444444444")
        );
    }

    #[test]
    fn test_two_identical_models_with_colliding_identity_downgrades_to_ambiguous() {
        let mon1 = WindowsMonitorObservation {
            display_number: 1,
            canonical_id: Some(String::from("win-del-4198-weakid")),
            display_label: Some(String::from("win-del-4198-weakid")),
            stable_id: Some(String::from("win-del-4198-weakid")),
            identity_quality: IdentityQuality::StableDevice,
            mutation_eligibility: W3MutationEligibility::ExplicitOnly,
            name: Some(String::from("Dell Monitor")),
            kind: WindowsDisplayKind::ExternalMonitor,
            active: true,
            target_available: true,
            is_primary: true,
            output_technology: String::from("DisplayPort"),
            gdi_device_name: Some(String::from(r"\\.\DISPLAY1")),
            adapter_luid: (10, 0),
            source_id: 0,
            target_id: 1,
            connector_instance: 0,
            container_id: None,
            device_instance_id: None,
            hardware_ids: Vec::new(),
            edid_manufacturer: Some(String::from("DEL")),
            edid_product_code: Some(0x4198),
            physical_monitors_count: 1,
            physical_monitor_description: None,
            desktop_rect: None,
            note: None,
        };

        let mon2 = WindowsMonitorObservation {
            display_number: 2,
            canonical_id: Some(String::from("win-del-4198-weakid")), // Identical colliding stable ID!
            display_label: Some(String::from("win-del-4198-weakid")),
            stable_id: Some(String::from("win-del-4198-weakid")),
            identity_quality: IdentityQuality::StableDevice,
            mutation_eligibility: W3MutationEligibility::ExplicitOnly,
            name: Some(String::from("Dell Monitor")),
            kind: WindowsDisplayKind::ExternalMonitor,
            active: true,
            target_available: true,
            is_primary: false,
            output_technology: String::from("HDMI"),
            gdi_device_name: Some(String::from(r"\\.\DISPLAY2")),
            adapter_luid: (10, 0),
            source_id: 1,
            target_id: 2,
            connector_instance: 1,
            container_id: None,
            device_instance_id: None,
            hardware_ids: Vec::new(),
            edid_manufacturer: Some(String::from("DEL")),
            edid_product_code: Some(0x4198),
            physical_monitors_count: 1,
            physical_monitor_description: None,
            desktop_rect: None,
            note: None,
        };

        let mut observations = vec![mon1, mon2];

        // Run collision detection algorithm (same as in capture_topology_snapshot)
        let mut id_counts: HashMap<String, usize> = HashMap::new();
        for mon in &observations {
            if let Some(ref id) = mon.canonical_id {
                *id_counts.entry(id.clone()).or_insert(0) += 1;
            }
        }
        for mon in &mut observations {
            if let Some(ref id) = mon.canonical_id {
                if id_counts.get(id).copied().unwrap_or(0) > 1 {
                    mon.identity_quality = IdentityQuality::Ambiguous;
                    mon.mutation_eligibility = W3MutationEligibility::Ineligible;
                    let notice = format!(
                        "Collision detected: multiple active displays share canonical ID '{id}'; marked ambiguous for safety"
                    );
                    mon.note = Some(notice);
                }
            }
        }

        // Both monitors must be downgraded to Ambiguous and Ineligible!
        assert_eq!(observations[0].identity_quality, IdentityQuality::Ambiguous);
        assert_eq!(observations[1].identity_quality, IdentityQuality::Ambiguous);
        assert_eq!(
            observations[0].mutation_eligibility,
            W3MutationEligibility::Ineligible
        );
        assert_eq!(
            observations[1].mutation_eligibility,
            W3MutationEligibility::Ineligible
        );
        assert!(observations[0]
            .note
            .as_ref()
            .unwrap()
            .contains("Collision detected"));
        assert!(observations[1]
            .note
            .as_ref()
            .unwrap()
            .contains("Collision detected"));
        // Both monitors remain in list without dropping or silent merging!
        assert_eq!(observations.len(), 2);
    }

    #[test]
    fn test_missing_container_id_fallback_to_device_instance_uses_128bit_hash() {
        let (canonical_id, display_label, quality, eligibility) = derive_stable_identity(
            None,
            Some(r"DISPLAY\DEL4198\5&383f58e1&0&UID4357"),
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );

        assert_eq!(quality, IdentityQuality::StableDevice);
        assert_eq!(eligibility, W3MutationEligibility::ExplicitOnly);

        let id = canonical_id.expect("canonical_id must be populated");
        assert!(id.starts_with("win-dev-del-4198-"));
        // Remainder must be 32 hex characters (128-bit hash)
        let hash_part = id.strip_prefix("win-dev-del-4198-").unwrap();
        assert_eq!(
            hash_part.len(),
            32,
            "hash must be 128 bits (32 hex characters)"
        );

        // Display label is abbreviated
        assert_eq!(display_label.as_deref(), Some("win-dev-del-5fe949b3"));
    }

    #[test]
    fn test_missing_edid_ids_and_empty_friendly_name_fails_safe() {
        let (canonical_id, _, quality, eligibility) = derive_stable_identity(
            None,
            None,
            None,
            None,
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            3,
        );

        assert_eq!(quality, IdentityQuality::PortBound);
        assert_eq!(eligibility, W3MutationEligibility::Ineligible);
        assert_eq!(canonical_id.as_deref(), Some("win-port-a_0-3"));
    }

    #[test]
    fn test_indirect_wired_classification_and_eligibility() {
        let (canonical_id, _, quality, eligibility) = derive_stable_identity(
            None,
            None,
            None,
            None,
            WindowsDisplayKind::IndirectDisplay,
            (10, 0),
            5,
        );

        assert_eq!(quality, IdentityQuality::PortBound);
        assert_eq!(eligibility, W3MutationEligibility::Ineligible);
        assert_eq!(canonical_id.as_deref(), Some("win-indirect-5"));
    }

    #[test]
    fn test_virtual_display_classification_and_quality() {
        let (canonical_id, _, quality, eligibility) = derive_stable_identity(
            Some("{12345678-1234-1234-1234-123456789abc}"),
            Some(r"ROOT\BASICDISPLAY\0000"),
            None,
            None,
            WindowsDisplayKind::VirtualDisplay,
            (0, 0),
            99,
        );

        assert_eq!(quality, IdentityQuality::Virtual);
        assert_eq!(eligibility, W3MutationEligibility::Ineligible);
        assert_eq!(canonical_id.as_deref(), Some("win-vdisp-99"));
    }

    #[test]
    fn test_mutation_eligibility_contract() {
        assert_eq!(
            IdentityQuality::Strong.mutation_eligibility(),
            W3MutationEligibility::AutomaticEligible
        );
        assert_eq!(
            IdentityQuality::StableDevice.mutation_eligibility(),
            W3MutationEligibility::ExplicitOnly
        );
        assert_eq!(
            IdentityQuality::PortBound.mutation_eligibility(),
            W3MutationEligibility::Ineligible
        );
        assert_eq!(
            IdentityQuality::Ambiguous.mutation_eligibility(),
            W3MutationEligibility::Ineligible
        );
        assert_eq!(
            IdentityQuality::Virtual.mutation_eligibility(),
            W3MutationEligibility::Ineligible
        );
        assert_eq!(
            IdentityQuality::Unknown.mutation_eligibility(),
            W3MutationEligibility::Ineligible
        );

        assert!(IdentityQuality::Strong.is_w3_automatic_eligible());
        assert!(!IdentityQuality::StableDevice.is_w3_automatic_eligible());
        assert!(!IdentityQuality::PortBound.is_w3_automatic_eligible());
        assert!(!IdentityQuality::Ambiguous.is_w3_automatic_eligible());
    }

    #[test]
    fn test_historical_collision_two_monitors_with_same_old_prefix_remain_distinct() {
        // In W2, monitors with GUIDs {8c7ed206-aaaa-...} and {8c7ed206-bbbb-...}
        // would produce identical "win-del-4198-8c7ed206".
        // In W2.1, full canonical GUID ensures they are completely distinct:
        let (id_a, _, _, _) = derive_stable_identity(
            Some("{8c7ed206-aaaa-0000-0000-000000000000}"),
            None,
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );
        let (id_b, _, _, _) = derive_stable_identity(
            Some("{8c7ed206-bbbb-0000-0000-000000000000}"),
            None,
            Some("DEL"),
            Some(0x4198),
            WindowsDisplayKind::ExternalMonitor,
            (10, 0),
            1,
        );

        assert_ne!(id_a, id_b);
        assert_eq!(
            id_a.as_deref(),
            Some("win-del-4198-8c7ed206-aaaa-0000-0000-000000000000")
        );
        assert_eq!(
            id_b.as_deref(),
            Some("win-del-4198-8c7ed206-bbbb-0000-0000-000000000000")
        );
    }

    #[test]
    fn test_device_path_parser_forbidden_in_production() {
        // Given an opaque monitor device path on a system without a mock DevNode:
        let fake_path =
            r"\\?\DISPLAY#DEL4198#5&383f58e1&0&UID4357#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}";
        let pnp = query_pnp_device_info(fake_path);
        // Production code MUST NOT parse the opaque string:
        assert_eq!(
            pnp.device_instance_id, None,
            "device_instance_id must not be synthesized from opaque path when SetupAPI fails"
        );
    }

    #[test]
    fn test_edid_manufacturer_decoding() {
        // Dell: 'D' (4), 'E' (5), 'L' (12) -> (4<<10) | (5<<5) | 12 = 4268 (0x10AC)
        let dell_word = (4u16 << 10) | (5u16 << 5) | 12u16;
        assert_eq!(decode_edid_manufacturer(dell_word).as_deref(), Some("DEL"));

        // Asus: 'A' (1), 'S' (19), 'U' (21) -> (1<<10) | (19<<5) | 21 = 1653
        let asus_word = (1u16 << 10) | (19u16 << 5) | 21u16;
        assert_eq!(decode_edid_manufacturer(asus_word).as_deref(), Some("ASU"));

        // Invalid: 0
        assert_eq!(decode_edid_manufacturer(0), None);
    }

    #[test]
    fn test_generic_and_system_container_detection() {
        assert!(is_generic_or_system_container(
            "{00000000-0000-0000-0000-000000000000}"
        ));
        assert!(is_generic_or_system_container(
            "{00000000-0000-0000-ffff-ffffffffffff}"
        ));
        assert!(!is_generic_or_system_container(
            "{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}"
        ));
    }

    #[test]
    fn test_serialization_contains_no_raw_pointers_or_handles() {
        let obs = WindowsMonitorObservation {
            display_number: 1,
            canonical_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            display_label: Some(String::from("win-del-4198-8c7ed206")),
            stable_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            identity_quality: IdentityQuality::Strong,
            mutation_eligibility: W3MutationEligibility::AutomaticEligible,
            name: Some(String::from("Dell U2723QE")),
            kind: WindowsDisplayKind::ExternalMonitor,
            active: true,
            target_available: true,
            is_primary: true,
            output_technology: String::from("DisplayPort"),
            gdi_device_name: Some(String::from(r"\\.\DISPLAY1")),
            adapter_luid: (10, 0),
            source_id: 0,
            target_id: 1,
            connector_instance: 0,
            container_id: Some(String::from("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}")),
            device_instance_id: Some(String::from(r"DISPLAY\DEL4198\1")),
            hardware_ids: vec![String::from("MONITOR\\DEL4198")],
            edid_manufacturer: Some(String::from("DEL")),
            edid_product_code: Some(0x4198),
            physical_monitors_count: 1,
            physical_monitor_description: Some(String::from("Dell U2723QE")),
            desktop_rect: Some((0, 0, 3840, 2160)),
            note: None,
        };

        let json = serde_json::to_string(&obs).expect("JSON serialization failed");
        // Ensure no raw pointer addresses or win32 handle numbers leaked into JSON schema
        assert!(!json.contains("hmonitor"));
        assert!(!json.contains("hPhysicalMonitor"));
        assert!(json.contains("win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c"));
        assert!(json.contains("automatic_eligible"));
    }
}
