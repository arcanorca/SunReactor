//! Windows GDI logical monitor enumeration and desktop geometry.
//!
//! Enumerates logical display monitors (`HMONITOR`) via `EnumDisplayMonitors`
//! and inspects device names (`szDevice`, e.g. `\\.\DISPLAY1`) and bounds via `GetMonitorInfoW`.

use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};

/// Logical GDI monitor observed from the Windows desktop subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GdiMonitorInfo {
    /// Opaque pointer represented as `usize` for safe correlation (never serialized as persistent ID).
    pub hmonitor: usize,
    /// GDI device name matching CCD source `viewGdiDeviceName` (e.g. `\\.\DISPLAY1`).
    pub device_name: String,
    /// True if this logical monitor is the primary Windows desktop display.
    pub is_primary: bool,
    /// Full virtual desktop coordinate rectangle (left, top, right, bottom).
    pub monitor_rect: (i32, i32, i32, i32),
    /// Usable desktop work area excluding taskbar (left, top, right, bottom).
    pub work_rect: (i32, i32, i32, i32),
}

unsafe extern "system" fn monitor_enum_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _clip_rect: *mut RECT,
    data: LPARAM,
) -> windows_sys::core::BOOL {
    let list = unsafe { &mut *(data as *mut Vec<GdiMonitorInfo>) };

    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    // Safety justification:
    // - `hmonitor` is guaranteed valid by `EnumDisplayMonitors`.
    // - `info` size is properly initialized before query.
    let ok = unsafe { GetMonitorInfoW(hmonitor, &raw mut info.monitorInfo) };
    if ok != 0 {
        let name_len = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        let device_name = String::from_utf16_lossy(&info.szDevice[..name_len]);

        // MONITORINFOF_PRIMARY = 1
        let is_primary = (info.monitorInfo.dwFlags & 1) != 0;
        let m_rect = (
            info.monitorInfo.rcMonitor.left,
            info.monitorInfo.rcMonitor.top,
            info.monitorInfo.rcMonitor.right,
            info.monitorInfo.rcMonitor.bottom,
        );
        let w_rect = (
            info.monitorInfo.rcWork.left,
            info.monitorInfo.rcWork.top,
            info.monitorInfo.rcWork.right,
            info.monitorInfo.rcWork.bottom,
        );

        list.push(GdiMonitorInfo {
            hmonitor: hmonitor as usize,
            device_name,
            is_primary,
            monitor_rect: m_rect,
            work_rect: w_rect,
        });
    }

    1 // Continue enumeration
}

/// Enumerates all active logical desktop monitors via Win32 GDI.
#[must_use]
pub fn enumerate_gdi_monitors() -> Vec<GdiMonitorInfo> {
    let mut list = Vec::new();

    // Safety justification:
    // - `list` remains valid on stack for the duration of the synchronous enumeration call.
    unsafe {
        EnumDisplayMonitors(
            null_mut(),
            null(),
            Some(monitor_enum_callback),
            &raw mut list as LPARAM,
        );
    }

    list
}
