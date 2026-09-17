//! Windows Display Configuration (CCD) querying and output technology abstraction.
//!
//! Provides safe access to Windows Connecting and Configuring Displays (CCD) APIs
//! (`GetDisplayConfigBufferSizes`, `QueryDisplayConfig`, `DisplayConfigGetDeviceInfo`).

use std::fmt;
use windows_sys::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_COMPONENT_VIDEO,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_COMPOSITE_VIDEO,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EXTERNAL,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_USB_TUNNEL, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DVI,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_D_JPN, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HD15,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HDMI, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INDIRECT_VIRTUAL,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INDIRECT_WIRED, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_MIRACAST,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_OTHER, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_SDI,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_SDTVDONGLE, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_SVIDEO,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EXTERNAL,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
    QDC_ONLY_ACTIVE_PATHS, QDC_VIRTUAL_MODE_AWARE,
};

const DISPLAYCONFIG_PATH_ACTIVE: u32 = 0x0000_0001;
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_GEN_FAILURE, ERROR_INSUFFICIENT_BUFFER, ERROR_NOT_SUPPORTED,
    ERROR_SUCCESS,
};

/// High-level classification of Windows display device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowsDisplayKind {
    /// External desktop monitor (HDMI, DisplayPort, DVI, VGA).
    ExternalMonitor,
    /// Integrated internal panel (laptop screen via eDP, LVDS, internal bus).
    InternalPanel,
    /// Indirect display (DisplayLink USB dock, external USB adapter driving physical display).
    IndirectDisplay,
    /// Virtual or remote display (RDP, virtual GPU, Miracast).
    VirtualDisplay,
    /// Unknown or unclassified display output technology.
    Unknown,
}

impl fmt::Display for WindowsDisplayKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExternalMonitor => write!(f, "External"),
            Self::InternalPanel => write!(f, "Internal"),
            Self::IndirectDisplay => write!(f, "Indirect"),
            Self::VirtualDisplay => write!(f, "Virtual"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Structured error for Windows display topology operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WindowsDisplayError {
    #[error("display configuration API access denied (session or permission issue)")]
    AccessDenied,
    #[error("display configuration API not supported on this platform/driver")]
    NotSupported,
    #[error("topology query buffer race exceeded retry budget")]
    BufferRaceTimeout,
    #[error("display configuration query failed with Win32 error {code}")]
    Win32 { code: u32 },
}

/// Decodes an EDID 16-bit manufacturer ID into the standard VESA 3-letter PNP ID.
#[must_use]
pub fn decode_edid_manufacturer(id: u16) -> Option<String> {
    let try_decode = |val: u16| -> Option<String> {
        let c1 = ((val >> 10) & 0x1F) as u8;
        let c2 = ((val >> 5) & 0x1F) as u8;
        let c3 = (val & 0x1F) as u8;
        if (1..=26).contains(&c1) && (1..=26).contains(&c2) && (1..=26).contains(&c3) {
            Some(format!(
                "{}{}{}",
                (c1 - 1 + b'A') as char,
                (c2 - 1 + b'A') as char,
                (c3 - 1 + b'A') as char
            ))
        } else {
            None
        }
    };
    try_decode(id.to_be()).or_else(|| try_decode(id))
}

/// Classifies a Win32 `DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY` into a high-level `WindowsDisplayKind`.
#[must_use]
pub fn classify_output_technology(tech: i32) -> (WindowsDisplayKind, &'static str) {
    match tech {
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL => (WindowsDisplayKind::InternalPanel, "Internal"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED => {
            (WindowsDisplayKind::InternalPanel, "Embedded DisplayPort")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS => (WindowsDisplayKind::InternalPanel, "LVDS"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED => {
            (WindowsDisplayKind::InternalPanel, "Embedded UDI")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EXTERNAL => {
            (WindowsDisplayKind::ExternalMonitor, "DisplayPort")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_USB_TUNNEL => (
            WindowsDisplayKind::ExternalMonitor,
            "DisplayPort (USB Tunnel)",
        ),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HDMI => (WindowsDisplayKind::ExternalMonitor, "HDMI"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DVI => (WindowsDisplayKind::ExternalMonitor, "DVI"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HD15 => (WindowsDisplayKind::ExternalMonitor, "VGA (HD15)"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EXTERNAL => {
            (WindowsDisplayKind::ExternalMonitor, "External UDI")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_SDI => (WindowsDisplayKind::ExternalMonitor, "SDI"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_COMPONENT_VIDEO => {
            (WindowsDisplayKind::ExternalMonitor, "Component Video")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_COMPOSITE_VIDEO => {
            (WindowsDisplayKind::ExternalMonitor, "Composite Video")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_SVIDEO => (WindowsDisplayKind::ExternalMonitor, "S-Video"),
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_D_JPN => {
            (WindowsDisplayKind::ExternalMonitor, "D-Connector (Japan)")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_SDTVDONGLE => {
            (WindowsDisplayKind::ExternalMonitor, "SDTV Dongle")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INDIRECT_WIRED => {
            (WindowsDisplayKind::IndirectDisplay, "Indirect Wired")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INDIRECT_VIRTUAL => {
            (WindowsDisplayKind::VirtualDisplay, "Indirect Virtual")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_MIRACAST => {
            (WindowsDisplayKind::VirtualDisplay, "Miracast Wireless")
        }
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_OTHER => (WindowsDisplayKind::Unknown, "Other"),
        _ => (WindowsDisplayKind::Unknown, "Unknown"),
    }
}

/// Information queried about a CCD desktop display source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcdSourceInfo {
    pub adapter_luid: (u32, i32),
    pub source_id: u32,
    pub gdi_device_name: String,
}

/// Information queried about a CCD display output target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcdTargetInfo {
    pub adapter_luid: (u32, i32),
    pub target_id: u32,
    pub output_technology: String,
    pub output_tech_code: i32,
    pub kind: WindowsDisplayKind,
    pub connector_instance: u32,
    pub target_available: bool,
    pub friendly_name: Option<String>,
    pub friendly_name_from_edid: bool,
    pub edid_ids_valid: bool,
    pub edid_manufacturer: Option<String>,
    pub edid_product_code: Option<u16>,
    pub monitor_device_path: Option<String>,
}

/// Combined active CCD path information associating a source with a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcdPathInfo {
    pub source: CcdSourceInfo,
    pub target: CcdTargetInfo,
    pub active: bool,
}

/// Queries all active display paths from Windows CCD subsystem with bounded buffer-race retry.
#[allow(clippy::too_many_lines)]
pub fn query_active_ccd_paths() -> Result<Vec<CcdPathInfo>, WindowsDisplayError> {
    let flags = QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE;

    // Retry loop handles the documented race where the desktop display topology changes
    // between `GetDisplayConfigBufferSizes` and `QueryDisplayConfig`.
    let mut raw_paths = Vec::new();
    let mut last_error = 0;

    for _ in 0..5 {
        let mut path_count: u32 = 0;
        let mut mode_count: u32 = 0;

        // Safety justification:
        // - `GetDisplayConfigBufferSizes` fills scalar counts at valid mutable pointers.
        let status =
            unsafe { GetDisplayConfigBufferSizes(flags, &raw mut path_count, &raw mut mode_count) };

        if status == ERROR_ACCESS_DENIED {
            return Err(WindowsDisplayError::AccessDenied);
        }
        if status == ERROR_NOT_SUPPORTED || status == ERROR_GEN_FAILURE {
            return Err(WindowsDisplayError::NotSupported);
        }
        if status != ERROR_SUCCESS {
            last_error = status;
            continue;
        }

        if path_count == 0 {
            return Ok(Vec::new());
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![
            windows_sys::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO::default(
            );
            mode_count as usize
        ];

        // Safety justification:
        // - Buffer pointers and element count pointers match allocated capacity.
        let query_res = unsafe {
            QueryDisplayConfig(
                flags,
                &raw mut path_count,
                paths.as_mut_ptr(),
                &raw mut mode_count,
                modes.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };

        if query_res == ERROR_SUCCESS {
            paths.truncate(path_count as usize);
            raw_paths = paths;
            break;
        }

        if query_res == ERROR_INSUFFICIENT_BUFFER {
            // Topology changed, retry with fresh buffer size query
            continue;
        }

        if query_res == ERROR_ACCESS_DENIED {
            return Err(WindowsDisplayError::AccessDenied);
        }
        if query_res == ERROR_NOT_SUPPORTED {
            return Err(WindowsDisplayError::NotSupported);
        }

        last_error = query_res;
    }

    if raw_paths.is_empty() && last_error != 0 && last_error != ERROR_SUCCESS {
        return Err(WindowsDisplayError::Win32 { code: last_error });
    }

    let mut result = Vec::with_capacity(raw_paths.len());

    for path in raw_paths {
        let is_active = (path.flags & DISPLAYCONFIG_PATH_ACTIVE) != 0;
        let adapter_luid = (
            path.targetInfo.adapterId.LowPart,
            path.targetInfo.adapterId.HighPart,
        );

        // 1. Query Source Device Name (e.g. \\.\DISPLAY1)
        let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            },
            viewGdiDeviceName: [0; 32],
        };

        let gdi_name = unsafe {
            if DisplayConfigGetDeviceInfo(&raw mut source_name.header) == 0 {
                let len = source_name
                    .viewGdiDeviceName
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(source_name.viewGdiDeviceName.len());
                String::from_utf16_lossy(&source_name.viewGdiDeviceName[..len])
            } else {
                String::new()
            }
        };

        let source_info = CcdSourceInfo {
            adapter_luid: (
                path.sourceInfo.adapterId.LowPart,
                path.sourceInfo.adapterId.HighPart,
            ),
            source_id: path.sourceInfo.id,
            gdi_device_name: gdi_name,
        };

        // 2. Query Target Device Name (Friendly name, Device path, EDID IDs)
        let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };

        let target_info = unsafe {
            if DisplayConfigGetDeviceInfo(&raw mut target_name.header) == 0 {
                let friendly_len = target_name
                    .monitorFriendlyDeviceName
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(target_name.monitorFriendlyDeviceName.len());
                let friendly = if friendly_len > 0 {
                    Some(String::from_utf16_lossy(
                        &target_name.monitorFriendlyDeviceName[..friendly_len],
                    ))
                } else {
                    None
                };

                let path_len = target_name
                    .monitorDevicePath
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(target_name.monitorDevicePath.len());
                let dev_path = if path_len > 0 {
                    Some(String::from_utf16_lossy(
                        &target_name.monitorDevicePath[..path_len],
                    ))
                } else {
                    None
                };

                let flags_val = target_name.flags.Anonymous.value;
                let from_edid = (flags_val & 0x1) != 0;
                let edid_valid = (flags_val & 0x4) != 0;

                let (mfg, prod) = if edid_valid && target_name.edidManufactureId != 0 {
                    (
                        decode_edid_manufacturer(target_name.edidManufactureId),
                        Some(target_name.edidProductCodeId),
                    )
                } else {
                    (None, None)
                };

                let (kind, tech_name) = classify_output_technology(target_name.outputTechnology);

                CcdTargetInfo {
                    adapter_luid,
                    target_id: path.targetInfo.id,
                    output_technology: tech_name.to_string(),
                    output_tech_code: target_name.outputTechnology,
                    kind,
                    connector_instance: target_name.connectorInstance,
                    target_available: path.targetInfo.targetAvailable != 0,
                    friendly_name: friendly,
                    friendly_name_from_edid: from_edid,
                    edid_ids_valid: edid_valid,
                    edid_manufacturer: mfg,
                    edid_product_code: prod,
                    monitor_device_path: dev_path,
                }
            } else {
                let (kind, tech_name) =
                    classify_output_technology(path.targetInfo.outputTechnology);
                CcdTargetInfo {
                    adapter_luid,
                    target_id: path.targetInfo.id,
                    output_technology: tech_name.to_string(),
                    output_tech_code: path.targetInfo.outputTechnology,
                    kind,
                    connector_instance: 0,
                    target_available: path.targetInfo.targetAvailable != 0,
                    friendly_name: None,
                    friendly_name_from_edid: false,
                    edid_ids_valid: false,
                    edid_manufacturer: None,
                    edid_product_code: None,
                    monitor_device_path: None,
                }
            }
        };

        result.push(CcdPathInfo {
            source: source_info,
            target: target_info,
            active: is_active,
        });
    }

    Ok(result)
}
