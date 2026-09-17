//! Windows DXVA2 physical monitor enumeration and handle RAII management.
//!
//! Enumerates physical monitor count and descriptions associated with an `HMONITOR`
//! using `GetNumberOfPhysicalMonitorsFromHMONITOR` and `GetPhysicalMonitorsFromHMONITOR`.
//!
//! # Safety & Lifecycle Invariants
//! - Absolutely READ-ONLY during W2: no brightness or capability mutations are invoked.
//! - All physical monitor handles are transient OS resources and are destroyed immediately
//!   via `DestroyPhysicalMonitors` inside `PhysicalMonitorGuard` on drop.

use windows_sys::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR,
};
use windows_sys::Win32::Graphics::Gdi::HMONITOR;

/// Safe RAII guard for DXVA2 physical monitor handles.
/// Ensures `DestroyPhysicalMonitors` is invoked on all exit paths, preventing resource leaks.
pub struct PhysicalMonitorGuard {
    handles: Vec<PHYSICAL_MONITOR>,
}

impl PhysicalMonitorGuard {
    /// Creates a new guard wrapping an array of physical monitors.
    #[must_use]
    pub fn new(handles: Vec<PHYSICAL_MONITOR>) -> Self {
        Self { handles }
    }

    /// Returns the number of physical monitor handles owned by this guard.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.handles.len() as u32
    }

    /// Returns a slice of the owned physical monitor structs.
    #[must_use]
    pub fn handles(&self) -> &[PHYSICAL_MONITOR] {
        &self.handles
    }

    /// Extracts the informational description string of the first physical monitor.
    #[must_use]
    pub fn first_description(&self) -> Option<String> {
        self.handles.first().and_then(|pm| {
            let desc = pm.szPhysicalMonitorDescription;
            let len = desc.iter().position(|&c| c == 0).unwrap_or(desc.len());
            if len > 0 {
                Some(String::from_utf16_lossy(&desc[..len]))
            } else {
                None
            }
        })
    }
}

impl Drop for PhysicalMonitorGuard {
    fn drop(&mut self) {
        if !self.handles.is_empty() {
            // Safety justification:
            // - `handles` contains valid `PHYSICAL_MONITOR` structs returned by `GetPhysicalMonitorsFromHMONITOR`.
            // - `self.handles.len() as u32` exactly matches the allocated array length.
            unsafe {
                DestroyPhysicalMonitors(self.handles.len() as u32, self.handles.as_ptr());
            }
            self.handles.clear();
        }
    }
}

/// Observed physical monitor metadata for an HMONITOR.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhysicalMonitorObservation {
    /// Count of physical monitor handles associated with this GDI logical monitor (0, 1, or N).
    pub count: u32,
    /// Informational description from the driver (e.g. "DELL U2723QE").
    pub description: Option<String>,
}

/// Queries physical monitor handles for an HMONITOR, reads descriptions, and releases them immediately.
#[must_use]
pub fn query_physical_monitors(hmonitor: usize) -> PhysicalMonitorObservation {
    let hm = hmonitor as HMONITOR;
    let mut count: u32 = 0;

    // Safety justification:
    // - `count` is a valid pointer on the stack.
    let ok = unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hm, &raw mut count) };
    if ok == 0 || count == 0 {
        return PhysicalMonitorObservation {
            count: 0,
            description: None,
        };
    }

    let mut handles = vec![PHYSICAL_MONITOR::default(); count as usize];
    let ok = unsafe { GetPhysicalMonitorsFromHMONITOR(hm, count, handles.as_mut_ptr()) };
    if ok == 0 {
        return PhysicalMonitorObservation {
            count: 0,
            description: None,
        };
    }

    // Wrap handles immediately in RAII guard so they are guaranteed destroyed on drop
    let guard = PhysicalMonitorGuard::new(handles);
    let desc = guard.first_description();
    let total = guard.count();

    // Guard drops here and invokes `DestroyPhysicalMonitors`
    drop(guard);

    PhysicalMonitorObservation {
        count: total,
        description: desc,
    }
}

/// Acquires physical monitor handles for an HMONITOR wrapped in a safe RAII guard.
///
/// Returns `Ok(PhysicalMonitorGuard)` which ensures `DestroyPhysicalMonitors` is called
/// when dropped, or `Err(win32_error_code)` if the Win32 call failed.
pub fn acquire_physical_monitors(hmonitor: usize) -> Result<PhysicalMonitorGuard, u32> {
    let hm = hmonitor as HMONITOR;
    let mut count: u32 = 0;

    // Safety justification:
    // - `count` is a valid stack pointer.
    let ok = unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hm, &raw mut count) };
    if ok == 0 {
        let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        return Err(err);
    }
    if count == 0 {
        return Ok(PhysicalMonitorGuard::new(Vec::new()));
    }

    let mut handles = vec![PHYSICAL_MONITOR::default(); count as usize];
    // Safety justification:
    // - `handles` has allocated capacity for `count` elements.
    let ok = unsafe { GetPhysicalMonitorsFromHMONITOR(hm, count, handles.as_mut_ptr()) };
    if ok == 0 {
        let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        return Err(err);
    }

    Ok(PhysicalMonitorGuard::new(handles))
}
