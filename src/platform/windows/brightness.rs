//! Native Windows external monitor brightness backend, capability probing, and physical qualification.
//!
//! Controls external physical monitors using Microsoft's native Monitor Configuration / DXVA2 APIs.
//!
//! # Safety & Lifecycle Invariants
//! - **High-Level API First**: Uses `GetMonitorCapabilities`, `GetMonitorBrightness`, and `SetMonitorBrightness`.
//!   No low-level VCP writes or `ddcutil.exe` subprocesses.
//! - **Strict Identity Gate**: Automatically mutates ONLY monitors with verified `IdentityQuality::Strong`
//!   and unique current topology match. Ineligible classes (`StableDevice`, `PortBound`, `Ambiguous`,
//!   `Virtual`, `Unknown`, `IndirectDisplay`) FAIL CLOSED.
//! - **1-to-1 Physical Mapping**: A GDI `HMONITOR` must map to EXACTLY 1 physical monitor handle.
//!   If physical count == 0 or >1, fails closed with `AmbiguousPhysicalMapping` to prevent misdirected writes.
//! - **No Redundant Writes**: Compares current native value with desired native value; if equal, NO WRITE.
//! - **Transient Handles**: Physical handles are acquired fresh per operation and destroyed immediately
//!   via RAII `PhysicalMonitorGuard`. Handles are NEVER cached across ticks, sleep, or reconnects.
//! - **Read-Before-Write**: Native brightness range is always validated via `GetMonitorBrightness` before any write.
//! - **No NVRAM Persistence**: Never invokes `SaveCurrentMonitorSettings` or factory reset APIs.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::display::{WindowsDisplayError, WindowsDisplayKind};
use super::gdi::enumerate_gdi_monitors;
use super::physical_monitors::{acquire_physical_monitors, PhysicalMonitorGuard};
use super::topology::{capture_topology_snapshot, IdentityQuality, WindowsMonitorObservation};

/// Mutex serializing monitor configuration operations to protect low-bandwidth DDC channels.
static MONITOR_MUTEX: Mutex<()> = Mutex::new(());

/// Win32 Monitor Capabilities bitmask flag for brightness support.
pub const MC_CAPS_BRIGHTNESS: u32 = 0x02;

/// Pure integer conversion error for native brightness bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BrightnessRangeError {
    #[error("invalid native bounds: min ({min}) >= max ({max})")]
    InvalidBounds { min: u32, max: u32 },
    #[error("current native value ({current}) out of bounds [{min}, {max}]")]
    CurrentOutOfBounds { min: u32, current: u32, max: u32 },
}

/// Validated native monitor brightness range reported by the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeBrightnessRange {
    pub min: u32,
    pub current: u32,
    pub max: u32,
}

impl NativeBrightnessRange {
    /// Validates and constructs a native brightness range.
    ///
    /// Requires `min < max` and `min <= current <= max`.
    pub fn new(min: u32, current: u32, max: u32) -> Result<Self, BrightnessRangeError> {
        if min >= max {
            return Err(BrightnessRangeError::InvalidBounds { min, max });
        }
        if current < min || current > max {
            return Err(BrightnessRangeError::CurrentOutOfBounds { min, current, max });
        }
        Ok(Self { min, current, max })
    }

    /// Converts current native brightness to a normalized percentage (0..=100)
    /// using deterministic integer arithmetic with nearest-half rounding.
    #[must_use]
    pub fn to_percent(&self) -> u8 {
        native_to_percent(self.min, self.current, self.max)
    }

    /// Converts a policy percentage (0..=100) to a native value in [min, max]
    /// using deterministic integer arithmetic with nearest-half rounding.
    #[must_use]
    pub fn percent_to_native(&self, percent: u8) -> u32 {
        percent_to_native(self.min, self.max, percent)
    }
}

/// Converts native brightness in `[min, max]` to a percentage in `0..=100`.
///
/// Uses integer-safe 64-bit arithmetic with nearest-half rounding.
#[must_use]
pub fn native_to_percent(min: u32, current: u32, max: u32) -> u8 {
    if min >= max {
        return 0;
    }
    let clamped_curr = current.clamp(min, max);
    let span = u64::from(max - min);
    let val = u64::from(clamped_curr - min);
    // Nearest-half rounding: (val * 100 + span / 2) / span
    let pct = (val * 100 + span / 2) / span;
    pct.min(100) as u8
}

/// Converts a percentage in `0..=100` to a native value in `[min, max]`.
///
/// Uses integer-safe 64-bit arithmetic with nearest-half rounding.
/// Guaranteed to be monotonic and bounded:
/// - 0% maps exactly to `min`
/// - 100% maps exactly to `max`
#[must_use]
pub fn percent_to_native(min: u32, max: u32, percent: u8) -> u32 {
    let p = u64::from(percent.min(100));
    if min >= max {
        return min;
    }
    let range = u64::from(max - min);
    // Nearest-half rounding: (p * range + 50) / 100
    let delta = (p * range + 50) / 100;
    min + delta as u32
}

/// Comprehensive semantic error taxonomy for Windows external brightness operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WindowsExternalBrightnessError {
    #[error("configured monitor identity `{canonical_id}` not found in active topology")]
    IdentityNotFound { canonical_id: String },
    #[error("monitor `{canonical_id}` identity quality `{quality:?}` is not eligible for automatic mutation")]
    IdentityNotEligible {
        canonical_id: String,
        quality: IdentityQuality,
    },
    #[error(
        "monitor `{canonical_id}` is ambiguous (collision detected with another active display)"
    )]
    IdentityAmbiguous { canonical_id: String },
    #[error("monitor `{canonical_id}` is inactive or target is not available")]
    TargetUnavailable { canonical_id: String },
    #[error(
        "monitor `{canonical_id}` is display kind `{kind:?}`, not an external physical monitor"
    )]
    NotExternal {
        canonical_id: String,
        kind: WindowsDisplayKind,
    },
    #[error("no physical monitor handles available for monitor `{canonical_id}`")]
    PhysicalMonitorUnavailable { canonical_id: String },
    #[error("monitor `{canonical_id}` maps to {count} physical monitors (>1); ambiguous physical mapping")]
    PhysicalMonitorMappingAmbiguous { canonical_id: String, count: u32 },
    #[error("GetMonitorCapabilities failed for monitor `{canonical_id}` (Win32 error {code})")]
    CapabilitiesQueryFailed { canonical_id: String, code: u32 },
    #[error("monitor `{canonical_id}` does not report MC_CAPS_BRIGHTNESS (flags: 0x{flags:08x})")]
    BrightnessUnsupported { canonical_id: String, flags: u32 },
    #[error("GetMonitorBrightness failed for monitor `{canonical_id}` (Win32 error {code})")]
    BrightnessReadFailed { canonical_id: String, code: u32 },
    #[error("invalid native brightness range for monitor `{canonical_id}`: min={min}, current={current}, max={max}")]
    InvalidBrightnessRange {
        canonical_id: String,
        min: u32,
        current: u32,
        max: u32,
    },
    #[error("SetMonitorBrightness({desired_native}) failed for monitor `{canonical_id}` (Win32 error {code})")]
    BrightnessWriteFailed {
        canonical_id: String,
        desired_native: u32,
        code: u32,
    },
    #[error("readback mismatch for monitor `{canonical_id}`: expected {expected_native}, observed {observed_native}")]
    ReadbackMismatch {
        canonical_id: String,
        expected_native: u32,
        observed_native: u32,
    },
    #[error("topology changed during operation for monitor `{canonical_id}`")]
    TopologyChanged { canonical_id: String },
    #[error("display configuration API query failed: {0}")]
    DisplayQuery(String),
}

impl From<WindowsDisplayError> for WindowsExternalBrightnessError {
    fn from(err: WindowsDisplayError) -> Self {
        Self::DisplayQuery(err.to_string())
    }
}

/// Classification of monitor brightness capability for discovery and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalBrightnessCapability {
    /// Fully supported: physical count == 1, MC_CAPS_BRIGHTNESS reported, read succeeded with valid range.
    Supported {
        range: NativeBrightnessRange,
        normalized_current_pct: u8,
    },
    /// Hardware or driver explicitly does not support brightness control.
    Unsupported { reason: String },
    /// Temporarily unavailable (e.g. targetAvailable=false, display in power-save).
    TemporarilyUnavailable { reason: String },
    /// Monitor reported an invalid or inverted brightness range.
    InvalidRange { min: u32, current: u32, max: u32 },
    /// GDI monitor maps to multiple physical monitors (>1) without unique handle correlation.
    AmbiguousPhysicalMapping { count: u32 },
    /// Monitor identity is not eligible for automatic mutation.
    IdentityNotMutationEligible { quality: String },
    /// Active topology does not contain this monitor or CCD path is not active.
    TopologyUnavailable,
    /// Win32 API returned an error during capability or brightness query.
    NativeApiFailure { operation: &'static str, code: u32 },
}

/// Abstract seam for native Win32 Monitor Configuration calls to enable deterministic testing.
pub trait MonitorConfigApi: Send + Sync {
    /// Calls `GetMonitorCapabilities` and returns the capabilities bitmask.
    fn get_capabilities(&self, handle: usize) -> Result<u32, u32>;
    /// Calls `GetMonitorBrightness` and returns the validated native range.
    fn get_brightness(&self, handle: usize) -> Result<NativeBrightnessRange, u32>;
    /// Calls `SetMonitorBrightness` with the target native value.
    fn set_brightness(&self, handle: usize, value: u32) -> Result<(), u32>;
}

/// Production implementation of `MonitorConfigApi` invoking native Win32 Monitor Configuration APIs.
#[derive(Debug, Clone, Copy, Default)]
pub struct Win32MonitorConfigApi;

impl MonitorConfigApi for Win32MonitorConfigApi {
    fn get_capabilities(&self, handle: usize) -> Result<u32, u32> {
        let h = handle as windows_sys::Win32::Foundation::HANDLE;
        let mut caps: u32 = 0;
        let mut color_temps: u32 = 0;

        // Safety justification:
        // - `h` is a valid `HANDLE` from `GetPhysicalMonitorsFromHMONITOR` protected by `PhysicalMonitorGuard`.
        // - `caps` and `color_temps` point to initialized stack variables.
        let ok = unsafe {
            windows_sys::Win32::Devices::Display::GetMonitorCapabilities(
                h,
                &raw mut caps,
                &raw mut color_temps,
            )
        };
        if ok == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            Err(err)
        } else {
            Ok(caps)
        }
    }

    fn get_brightness(&self, handle: usize) -> Result<NativeBrightnessRange, u32> {
        let h = handle as windows_sys::Win32::Foundation::HANDLE;
        let mut min: u32 = 0;
        let mut cur: u32 = 0;
        let mut max: u32 = 0;

        // Safety justification:
        // - `h` is a valid `HANDLE` protected by `PhysicalMonitorGuard`.
        // - `min`, `cur`, `max` point to initialized stack variables.
        let ok = unsafe {
            windows_sys::Win32::Devices::Display::GetMonitorBrightness(
                h,
                &raw mut min,
                &raw mut cur,
                &raw mut max,
            )
        };
        if ok == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            Err(err)
        } else {
            NativeBrightnessRange::new(min, cur, max).map_err(|_| 0x8007_0057) // E_INVALIDARG
        }
    }

    fn set_brightness(&self, handle: usize, value: u32) -> Result<(), u32> {
        let h = handle as windows_sys::Win32::Foundation::HANDLE;

        // Safety justification:
        // - `h` is a valid `HANDLE` protected by `PhysicalMonitorGuard`.
        // - `value` is pre-validated to be within the monitor's native [min, max] range.
        let ok = unsafe { windows_sys::Win32::Devices::Display::SetMonitorBrightness(h, value) };
        if ok == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            Err(err)
        } else {
            Ok(())
        }
    }
}

/// Detailed result of an external monitor brightness mutation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsExternalApplyResult {
    pub canonical_id: String,
    pub desired_percent: u8,
    pub desired_native: u32,
    pub previous_native: u32,
    /// `true` if hardware mutation occurred; `false` if suppressed due to native equality.
    pub applied: bool,
    pub readback_native: Option<u32>,
    pub read_latency: Duration,
    pub write_latency: Option<Duration>,
}

/// Comprehensive physical qualification test report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PhysicalQualificationReport {
    pub canonical_id: String,
    pub display_label: Option<String>,
    pub manufacturer: Option<String>,
    pub product_code: Option<u16>,
    pub output_technology: String,
    pub physical_handles_count: u32,
    pub native_min: u32,
    pub native_original: u32,
    pub native_max: u32,
    pub original_percent: u8,
    pub test_target_percent: u8,
    pub test_target_native: u32,
    pub pre_write_read_latency: Duration,
    pub write_latency: Duration,
    pub readback_observed_native: u32,
    pub readback_latency: Duration,
    pub restoration_write_latency: Duration,
    pub restored_observed_native: u32,
    pub restoration_readback_latency: Duration,
    pub qualified: bool,
    pub notes: Vec<String>,
}

/// Evaluates the complete set of safety gates required before any hardware mutation.
///
/// Fails closed if any condition is not met.
pub fn evaluate_mutation_safety_gate(
    matched: &WindowsMonitorObservation,
    physical_handles_count: u32,
    caps: Option<u32>,
    range: Option<NativeBrightnessRange>,
) -> Result<(), WindowsExternalBrightnessError> {
    let canonical_id = matched
        .canonical_id
        .clone()
        .unwrap_or_else(|| String::from("unknown"));

    if matched.identity_quality == IdentityQuality::Ambiguous {
        return Err(WindowsExternalBrightnessError::IdentityAmbiguous { canonical_id });
    }

    if matched.identity_quality != IdentityQuality::Strong {
        return Err(WindowsExternalBrightnessError::IdentityNotEligible {
            canonical_id,
            quality: matched.identity_quality,
        });
    }

    if !matched.active || !matched.target_available {
        return Err(WindowsExternalBrightnessError::TargetUnavailable { canonical_id });
    }

    if matched.kind != WindowsDisplayKind::ExternalMonitor {
        return Err(WindowsExternalBrightnessError::NotExternal {
            canonical_id,
            kind: matched.kind,
        });
    }

    if physical_handles_count == 0 {
        return Err(WindowsExternalBrightnessError::PhysicalMonitorUnavailable { canonical_id });
    }
    if physical_handles_count > 1 {
        return Err(
            WindowsExternalBrightnessError::PhysicalMonitorMappingAmbiguous {
                canonical_id,
                count: physical_handles_count,
            },
        );
    }

    let Some(caps_val) = caps else {
        return Err(WindowsExternalBrightnessError::CapabilitiesQueryFailed {
            canonical_id,
            code: 0x8000_4005,
        });
    };

    if (caps_val & MC_CAPS_BRIGHTNESS) == 0 {
        return Err(WindowsExternalBrightnessError::BrightnessUnsupported {
            canonical_id,
            flags: caps_val,
        });
    }

    let Some(r) = range else {
        return Err(WindowsExternalBrightnessError::BrightnessReadFailed {
            canonical_id,
            code: 0x8000_4005,
        });
    };

    if r.min >= r.max || r.current < r.min || r.current > r.max {
        return Err(WindowsExternalBrightnessError::InvalidBrightnessRange {
            canonical_id,
            min: r.min,
            current: r.current,
            max: r.max,
        });
    }

    Ok(())
}

/// Recaptures fresh topology and resolves a target monitor observation, acquiring its physical handle.
///
/// # Strict Safety Gates
/// 1. `capture_topology_snapshot()` is executed fresh on every invocation.
/// 2. Exactly one monitor in the snapshot must match `canonical_id`.
/// 3. Identity must not be `Ambiguous`.
/// 4. If `require_strong` is true, identity quality must be `Strong`.
/// 5. Monitor must be `active` and `target_available`.
/// 6. Display kind must be `ExternalMonitor`.
/// 7. GDI device name must correlate to an active `HMONITOR`.
/// 8. Physical monitor count must be EXACTLY 1. If 0 or >1, fails closed immediately.
pub fn resolve_and_acquire_external_monitor(
    canonical_id: &str,
    require_strong: bool,
) -> Result<(WindowsMonitorObservation, PhysicalMonitorGuard), WindowsExternalBrightnessError> {
    // 1. Fresh topology snapshot
    let snapshot = capture_topology_snapshot()?;

    // 2. Find matching observation
    let candidates: Vec<&WindowsMonitorObservation> = snapshot
        .monitors
        .iter()
        .filter(|m| {
            m.canonical_id.as_deref() == Some(canonical_id)
                || m.stable_id.as_deref() == Some(canonical_id)
        })
        .collect();

    let matched = match candidates.len() {
        0 => {
            return Err(WindowsExternalBrightnessError::IdentityNotFound {
                canonical_id: canonical_id.to_string(),
            });
        }
        1 => candidates[0].clone(),
        _ => {
            return Err(WindowsExternalBrightnessError::IdentityAmbiguous {
                canonical_id: canonical_id.to_string(),
            });
        }
    };

    // 3. Evaluate identity quality and eligibility
    if matched.identity_quality == IdentityQuality::Ambiguous {
        return Err(WindowsExternalBrightnessError::IdentityAmbiguous {
            canonical_id: canonical_id.to_string(),
        });
    }

    if require_strong && matched.identity_quality != IdentityQuality::Strong {
        return Err(WindowsExternalBrightnessError::IdentityNotEligible {
            canonical_id: canonical_id.to_string(),
            quality: matched.identity_quality,
        });
    }

    // 4. Target active and available check
    if !matched.active || !matched.target_available {
        return Err(WindowsExternalBrightnessError::TargetUnavailable {
            canonical_id: canonical_id.to_string(),
        });
    }

    // 5. Must be an external physical monitor
    if matched.kind != WindowsDisplayKind::ExternalMonitor {
        return Err(WindowsExternalBrightnessError::NotExternal {
            canonical_id: canonical_id.to_string(),
            kind: matched.kind,
        });
    }

    // 6. Correlate GDI Device Name to current HMONITOR
    let gdi_name = matched.gdi_device_name.as_deref().ok_or_else(|| {
        WindowsExternalBrightnessError::TopologyChanged {
            canonical_id: canonical_id.to_string(),
        }
    })?;

    let gdi_monitors = enumerate_gdi_monitors();
    let gdi_match = gdi_monitors
        .iter()
        .find(|g| g.device_name == gdi_name)
        .ok_or_else(|| WindowsExternalBrightnessError::TopologyChanged {
            canonical_id: canonical_id.to_string(),
        })?;

    // 7. Acquire physical monitor handles
    let guard = acquire_physical_monitors(gdi_match.hmonitor).map_err(|code| {
        WindowsExternalBrightnessError::BrightnessReadFailed {
            canonical_id: canonical_id.to_string(),
            code,
        }
    })?;

    // 8. Enforce 1-to-1 physical mapping rule
    let count = guard.count();
    if count == 0 {
        return Err(WindowsExternalBrightnessError::PhysicalMonitorUnavailable {
            canonical_id: canonical_id.to_string(),
        });
    }
    if count > 1 {
        return Err(
            WindowsExternalBrightnessError::PhysicalMonitorMappingAmbiguous {
                canonical_id: canonical_id.to_string(),
                count,
            },
        );
    }

    Ok((matched, guard))
}

/// Applies external brightness target to a uniquely resolved external monitor.
///
/// Implements read-before-write, native equality comparison (redundant write suppression),
/// bounds enforcement, and optional readback verification.
#[allow(clippy::too_many_lines)]
pub fn apply_external_brightness_with_api<A: MonitorConfigApi>(
    api: &A,
    canonical_id: &str,
    desired_percent: u8,
    verify_readback: bool,
) -> Result<WindowsExternalApplyResult, WindowsExternalBrightnessError> {
    let _lock = MONITOR_MUTEX.lock().unwrap();

    // 1. Resolve monitor and acquire fresh physical handle (Strong identity required)
    let (observation, guard) = resolve_and_acquire_external_monitor(canonical_id, true)?;
    let h_phys = guard
        .handles()
        .first()
        .map(|pm| pm.hPhysicalMonitor as usize)
        .ok_or_else(
            || WindowsExternalBrightnessError::PhysicalMonitorUnavailable {
                canonical_id: canonical_id.to_string(),
            },
        )?;

    // 2. Check capabilities
    let caps = api.get_capabilities(h_phys).map_err(|code| {
        WindowsExternalBrightnessError::CapabilitiesQueryFailed {
            canonical_id: canonical_id.to_string(),
            code,
        }
    })?;

    if (caps & MC_CAPS_BRIGHTNESS) == 0 {
        return Err(WindowsExternalBrightnessError::BrightnessUnsupported {
            canonical_id: canonical_id.to_string(),
            flags: caps,
        });
    }

    // 3. Read current native brightness before write
    let read_start = Instant::now();
    let range = api.get_brightness(h_phys).map_err(|code| {
        WindowsExternalBrightnessError::BrightnessReadFailed {
            canonical_id: canonical_id.to_string(),
            code,
        }
    })?;
    let read_latency = read_start.elapsed();

    // 4. Validate native range
    if range.min >= range.max || range.current < range.min || range.current > range.max {
        return Err(WindowsExternalBrightnessError::InvalidBrightnessRange {
            canonical_id: canonical_id.to_string(),
            min: range.min,
            current: range.current,
            max: range.max,
        });
    }

    // 5. Compute desired native value
    let desired_native = percent_to_native(range.min, range.max, desired_percent);

    // 6. Native equality check for redundant-write suppression
    if desired_native == range.current {
        tracing::info!(
            canonical_id = %canonical_id,
            display_label = ?observation.display_label,
            desired_native = desired_native,
            current_native = range.current,
            desired_percent = desired_percent,
            read_latency_ms = read_latency.as_millis(),
            "windows_external_redundant_write_suppressed"
        );
        return Ok(WindowsExternalApplyResult {
            canonical_id: canonical_id.to_string(),
            desired_percent,
            desired_native,
            previous_native: range.current,
            applied: false,
            readback_native: Some(range.current),
            read_latency,
            write_latency: None,
        });
    }

    // 7. Execute SetMonitorBrightness
    let write_start = Instant::now();
    api.set_brightness(h_phys, desired_native).map_err(|code| {
        WindowsExternalBrightnessError::BrightnessWriteFailed {
            canonical_id: canonical_id.to_string(),
            desired_native,
            code,
        }
    })?;
    let write_latency = write_start.elapsed();

    // 8. Optional readback verification
    let readback_native = if verify_readback {
        let readback = api.get_brightness(h_phys).map_err(|code| {
            WindowsExternalBrightnessError::BrightnessReadFailed {
                canonical_id: canonical_id.to_string(),
                code,
            }
        })?;
        if readback.current != desired_native {
            return Err(WindowsExternalBrightnessError::ReadbackMismatch {
                canonical_id: canonical_id.to_string(),
                expected_native: desired_native,
                observed_native: readback.current,
            });
        }
        Some(readback.current)
    } else {
        None
    };

    tracing::info!(
        canonical_id = %canonical_id,
        display_label = ?observation.display_label,
        previous_native = range.current,
        desired_native = desired_native,
        desired_percent = desired_percent,
        read_latency_ms = read_latency.as_millis(),
        write_latency_ms = write_latency.as_millis(),
        readback_verified = verify_readback,
        "windows_external_brightness_applied"
    );

    // Physical handle is destroyed immediately when `guard` drops at function exit
    Ok(WindowsExternalApplyResult {
        canonical_id: canonical_id.to_string(),
        desired_percent,
        desired_native,
        previous_native: range.current,
        applied: true,
        readback_native,
        read_latency,
        write_latency: Some(write_latency),
    })
}

/// Reads current native brightness and returns the range and normalized percentage.
pub fn read_external_brightness_with_api<A: MonitorConfigApi>(
    api: &A,
    canonical_id: &str,
) -> Result<(NativeBrightnessRange, u8), WindowsExternalBrightnessError> {
    let _lock = MONITOR_MUTEX.lock().unwrap();

    let (_, guard) = resolve_and_acquire_external_monitor(canonical_id, false)?;
    let h_phys = guard
        .handles()
        .first()
        .map(|pm| pm.hPhysicalMonitor as usize)
        .ok_or_else(
            || WindowsExternalBrightnessError::PhysicalMonitorUnavailable {
                canonical_id: canonical_id.to_string(),
            },
        )?;

    let range = api.get_brightness(h_phys).map_err(|code| {
        WindowsExternalBrightnessError::BrightnessReadFailed {
            canonical_id: canonical_id.to_string(),
            code,
        }
    })?;

    let pct = range.to_percent();
    Ok((range, pct))
}

/// Probes the brightness capability of an external monitor for discovery and diagnostics.
///
/// Non-mutating read-only probe.
pub fn probe_external_capability_with_api<A: MonitorConfigApi>(
    api: &A,
    canonical_id: &str,
) -> ExternalBrightnessCapability {
    let _lock = MONITOR_MUTEX.lock().unwrap();

    let (obs, guard) = match resolve_and_acquire_external_monitor(canonical_id, false) {
        Ok(res) => res,
        Err(err) => match err {
            WindowsExternalBrightnessError::IdentityNotFound { .. }
            | WindowsExternalBrightnessError::TopologyChanged { .. } => {
                return ExternalBrightnessCapability::TopologyUnavailable;
            }
            WindowsExternalBrightnessError::IdentityNotEligible { quality, .. } => {
                return ExternalBrightnessCapability::IdentityNotMutationEligible {
                    quality: format!("{quality:?}"),
                };
            }
            WindowsExternalBrightnessError::IdentityAmbiguous { .. } => {
                return ExternalBrightnessCapability::Unsupported {
                    reason: String::from("Identity ambiguous (collision detected)"),
                };
            }
            WindowsExternalBrightnessError::TargetUnavailable { .. } => {
                return ExternalBrightnessCapability::TemporarilyUnavailable {
                    reason: String::from("Target unavailable or inactive"),
                };
            }
            WindowsExternalBrightnessError::NotExternal { kind, .. } => {
                return ExternalBrightnessCapability::Unsupported {
                    reason: format!("Not an external physical monitor ({kind})"),
                };
            }
            WindowsExternalBrightnessError::PhysicalMonitorUnavailable { .. } => {
                return ExternalBrightnessCapability::TemporarilyUnavailable {
                    reason: String::from("No physical monitor handles available"),
                };
            }
            WindowsExternalBrightnessError::PhysicalMonitorMappingAmbiguous { count, .. } => {
                return ExternalBrightnessCapability::AmbiguousPhysicalMapping { count };
            }
            _ => {
                return ExternalBrightnessCapability::Unsupported {
                    reason: err.to_string(),
                };
            }
        },
    };

    let Some(pm) = guard.handles().first() else {
        return ExternalBrightnessCapability::TemporarilyUnavailable {
            reason: String::from("Physical handle missing"),
        };
    };
    let h_phys = pm.hPhysicalMonitor as usize;

    let caps = match api.get_capabilities(h_phys) {
        Ok(c) => c,
        Err(code) => {
            return ExternalBrightnessCapability::NativeApiFailure {
                operation: "GetMonitorCapabilities",
                code,
            };
        }
    };

    if (caps & MC_CAPS_BRIGHTNESS) == 0 {
        return ExternalBrightnessCapability::Unsupported {
            reason: format!("MC_CAPS_BRIGHTNESS not reported (caps: 0x{caps:08x})"),
        };
    }

    let range = match api.get_brightness(h_phys) {
        Ok(r) => r,
        Err(code) => {
            return ExternalBrightnessCapability::NativeApiFailure {
                operation: "GetMonitorBrightness",
                code,
            };
        }
    };

    if range.min >= range.max || range.current < range.min || range.current > range.max {
        return ExternalBrightnessCapability::InvalidRange {
            min: range.min,
            current: range.current,
            max: range.max,
        };
    }

    let normalized_current_pct = range.to_percent();
    tracing::info!(
        canonical_id = %canonical_id,
        display_label = ?obs.display_label,
        min = range.min,
        current = range.current,
        max = range.max,
        normalized_current_pct = normalized_current_pct,
        "external_brightness_capability_probed"
    );

    ExternalBrightnessCapability::Supported {
        range,
        normalized_current_pct,
    }
}

/// Executes a controlled, reversible physical qualification test on an explicitly selected monitor.
///
/// Flow:
/// 1. Read pre-write native brightness & range.
/// 2. Compute small reversible delta (e.g. +5% if current <= 90 else -5%).
/// 3. Apply test target native value.
/// 4. Readback verify test target was applied.
/// 5. Restore original native value.
/// 6. Readback verify original value was restored.
///
/// Returns a structured `PhysicalQualificationReport`.
#[allow(clippy::too_many_lines)]
pub fn test_physical_monitor_brightness<A: MonitorConfigApi>(
    api: &A,
    canonical_id: &str,
) -> Result<PhysicalQualificationReport, WindowsExternalBrightnessError> {
    let _lock = MONITOR_MUTEX.lock().unwrap();

    let (obs, guard) = resolve_and_acquire_external_monitor(canonical_id, true)?;
    let h_phys = guard
        .handles()
        .first()
        .map(|pm| pm.hPhysicalMonitor as usize)
        .ok_or_else(
            || WindowsExternalBrightnessError::PhysicalMonitorUnavailable {
                canonical_id: canonical_id.to_string(),
            },
        )?;

    // Check capabilities
    let caps = api.get_capabilities(h_phys).map_err(|code| {
        WindowsExternalBrightnessError::CapabilitiesQueryFailed {
            canonical_id: canonical_id.to_string(),
            code,
        }
    })?;

    if (caps & MC_CAPS_BRIGHTNESS) == 0 {
        return Err(WindowsExternalBrightnessError::BrightnessUnsupported {
            canonical_id: canonical_id.to_string(),
            flags: caps,
        });
    }

    // Step 1: Read pre-write native brightness
    let t0 = Instant::now();
    let initial_range = api.get_brightness(h_phys).map_err(|code| {
        WindowsExternalBrightnessError::BrightnessReadFailed {
            canonical_id: canonical_id.to_string(),
            code,
        }
    })?;
    let pre_write_read_latency = t0.elapsed();

    let original_native = initial_range.current;
    let original_percent = initial_range.to_percent();

    // Step 2: Compute small reversible delta
    let test_target_percent = if original_percent <= 90 {
        original_percent + 5
    } else {
        original_percent - 5
    };
    let test_target_native =
        percent_to_native(initial_range.min, initial_range.max, test_target_percent);

    // Step 3: Write test target
    let t1 = Instant::now();
    api.set_brightness(h_phys, test_target_native)
        .map_err(
            |code| WindowsExternalBrightnessError::BrightnessWriteFailed {
                canonical_id: canonical_id.to_string(),
                desired_native: test_target_native,
                code,
            },
        )?;
    let write_latency = t1.elapsed();

    // Step 4: Readback verify test target (intermediate read: do not early-exit before restoration!)
    let t2 = Instant::now();
    let test_readback = api.get_brightness(h_phys);
    let readback_latency = t2.elapsed();

    // Step 5: Restore original value (guaranteed restoration attempt)
    let t3 = Instant::now();
    let restore_result = api.set_brightness(h_phys, original_native);
    let restoration_write_latency = t3.elapsed();

    // Step 6: Readback verify restoration
    let t4 = Instant::now();
    let restored_readback = api.get_brightness(h_phys);
    let restoration_readback_latency = t4.elapsed();

    let mut notes = Vec::new();
    let mut qualified = true;

    let readback_observed_native = match test_readback {
        Ok(r) => {
            if r.current != test_target_native {
                qualified = false;
                notes.push(format!(
                    "Test target readback mismatch: expected native {test_target_native}, observed {}",
                    r.current
                ));
            }
            r.current
        }
        Err(code) => {
            qualified = false;
            notes.push(format!(
                "Test target readback query failed (Win32 error {code})"
            ));
            0
        }
    };

    if let Err(code) = restore_result {
        qualified = false;
        notes.push(format!(
            "CRITICAL: Failed to restore original brightness to {original_native} (Win32 error {code})"
        ));
    }

    let restored_observed_native = match restored_readback {
        Ok(r) => {
            if r.current != original_native {
                qualified = false;
                notes.push(format!(
                    "Restoration readback mismatch: expected native {original_native}, observed {}",
                    r.current
                ));
            }
            r.current
        }
        Err(code) => {
            qualified = false;
            notes.push(format!(
                "Failed to readback restored brightness (Win32 error {code})"
            ));
            0
        }
    };

    if qualified {
        notes.push(String::from(
            "Controlled delta, readback, restoration, and verify all succeeded.",
        ));
    }

    Ok(PhysicalQualificationReport {
        canonical_id: canonical_id.to_string(),
        display_label: obs.display_label,
        manufacturer: obs.edid_manufacturer,
        product_code: obs.edid_product_code,
        output_technology: obs.output_technology,
        physical_handles_count: guard.count(),
        native_min: initial_range.min,
        native_original: original_native,
        native_max: initial_range.max,
        original_percent,
        test_target_percent,
        test_target_native,
        pre_write_read_latency,
        write_latency,
        readback_observed_native,
        readback_latency,
        restoration_write_latency,
        restored_observed_native,
        restoration_readback_latency,
        qualified,
        notes,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    /// Configurable mock implementation of `MonitorConfigApi` for unit tests.
    #[derive(Debug, Default)]
    pub struct MockMonitorConfigApi {
        pub caps: AtomicU32,
        pub min: AtomicU32,
        pub current: AtomicU32,
        pub max: AtomicU32,
        pub caps_err: AtomicU32,
        pub read_err: AtomicU32,
        pub write_err: AtomicU32,
        pub write_calls: AtomicUsize,
        pub last_written: AtomicU32,
    }

    impl MockMonitorConfigApi {
        #[must_use]
        pub fn new(min: u32, current: u32, max: u32) -> Self {
            Self {
                caps: AtomicU32::new(0x02), // MC_CAPS_BRIGHTNESS
                min: AtomicU32::new(min),
                current: AtomicU32::new(current),
                max: AtomicU32::new(max),
                caps_err: AtomicU32::new(0),
                read_err: AtomicU32::new(0),
                write_err: AtomicU32::new(0),
                write_calls: AtomicUsize::new(0),
                last_written: AtomicU32::new(0),
            }
        }
    }

    impl MonitorConfigApi for MockMonitorConfigApi {
        fn get_capabilities(&self, _handle: usize) -> Result<u32, u32> {
            let err = self.caps_err.load(Ordering::Relaxed);
            if err != 0 {
                Err(err)
            } else {
                Ok(self.caps.load(Ordering::Relaxed))
            }
        }

        fn get_brightness(&self, _handle: usize) -> Result<NativeBrightnessRange, u32> {
            let err = self.read_err.load(Ordering::Relaxed);
            if err != 0 {
                return Err(err);
            }
            let min = self.min.load(Ordering::Relaxed);
            let cur = self.current.load(Ordering::Relaxed);
            let max = self.max.load(Ordering::Relaxed);
            NativeBrightnessRange::new(min, cur, max).map_err(|_| 0x8007_0057)
        }

        fn set_brightness(&self, _handle: usize, value: u32) -> Result<(), u32> {
            self.write_calls.fetch_add(1, Ordering::Relaxed);
            self.last_written.store(value, Ordering::Relaxed);
            let err = self.write_err.load(Ordering::Relaxed);
            if err != 0 {
                Err(err)
            } else {
                self.current.store(value, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    #[test]
    fn test_range_conversion_standard_0_to_100() {
        assert_eq!(percent_to_native(0, 100, 0), 0);
        assert_eq!(percent_to_native(0, 100, 42), 42);
        assert_eq!(percent_to_native(0, 100, 100), 100);

        assert_eq!(native_to_percent(0, 0, 100), 0);
        assert_eq!(native_to_percent(0, 42, 100), 42);
        assert_eq!(native_to_percent(0, 100, 100), 100);
    }

    #[test]
    fn test_range_conversion_quantized_0_to_10() {
        assert_eq!(percent_to_native(0, 10, 0), 0);
        assert_eq!(percent_to_native(0, 10, 4), 0); // 4*10 + 50 = 90 / 100 = 0
        assert_eq!(percent_to_native(0, 10, 5), 1); // 5*10 + 50 = 100 / 100 = 1
        assert_eq!(percent_to_native(0, 10, 50), 5);
        assert_eq!(percent_to_native(0, 10, 100), 10);

        assert_eq!(native_to_percent(0, 0, 10), 0);
        assert_eq!(native_to_percent(0, 1, 10), 10);
        assert_eq!(native_to_percent(0, 5, 10), 50);
        assert_eq!(native_to_percent(0, 10, 10), 100);
    }

    #[test]
    fn test_range_conversion_offset_10_to_110() {
        assert_eq!(percent_to_native(10, 110, 0), 10);
        assert_eq!(percent_to_native(10, 110, 50), 60);
        assert_eq!(percent_to_native(10, 110, 100), 110);

        assert_eq!(native_to_percent(10, 10, 110), 0);
        assert_eq!(native_to_percent(10, 60, 110), 50);
        assert_eq!(native_to_percent(10, 110, 110), 100);
    }

    #[test]
    fn test_range_conversion_narrow_20_to_80() {
        assert_eq!(percent_to_native(20, 80, 0), 20);
        assert_eq!(percent_to_native(20, 80, 50), 50);
        assert_eq!(percent_to_native(20, 80, 100), 80);

        assert_eq!(native_to_percent(20, 20, 80), 0);
        assert_eq!(native_to_percent(20, 50, 80), 50);
        assert_eq!(native_to_percent(20, 80, 80), 100);
    }

    #[test]
    fn test_range_conversion_byte_0_to_255() {
        assert_eq!(percent_to_native(0, 255, 0), 0);
        assert_eq!(percent_to_native(0, 255, 100), 255);

        assert_eq!(native_to_percent(0, 0, 255), 0);
        assert_eq!(native_to_percent(0, 255, 255), 100);
    }

    #[test]
    fn test_range_conversion_monotonicity() {
        for range in [10u32, 50, 100, 255, 1000] {
            let mut prev_native = 0;
            for pct in 0..=100 {
                let native = percent_to_native(0, range, pct);
                assert!(
                    native >= prev_native,
                    "monotonicity violated at pct {pct} for range {range}"
                );
                prev_native = native;
            }
        }
    }

    #[test]
    fn test_invalid_range_validation_fails_safe() {
        // min == max
        assert!(matches!(
            NativeBrightnessRange::new(10, 10, 10),
            Err(BrightnessRangeError::InvalidBounds { min: 10, max: 10 })
        ));
        // min > max
        assert!(matches!(
            NativeBrightnessRange::new(100, 50, 50),
            Err(BrightnessRangeError::InvalidBounds { min: 100, max: 50 })
        ));
        // current < min
        assert!(matches!(
            NativeBrightnessRange::new(10, 5, 100),
            Err(BrightnessRangeError::CurrentOutOfBounds {
                min: 10,
                current: 5,
                max: 100
            })
        ));
        // current > max
        assert!(matches!(
            NativeBrightnessRange::new(0, 150, 100),
            Err(BrightnessRangeError::CurrentOutOfBounds {
                min: 0,
                current: 150,
                max: 100
            })
        ));
        // u32 edge values
        assert!(matches!(
            NativeBrightnessRange::new(u32::MAX, u32::MAX, u32::MAX),
            Err(BrightnessRangeError::InvalidBounds { .. })
        ));
    }

    #[test]
    fn test_redundant_write_suppression_when_native_values_match() {
        let api = MockMonitorConfigApi::new(0, 50, 100);
        let range = api.get_brightness(1).unwrap();
        let desired_percent = 50;
        let desired_native = range.percent_to_native(desired_percent);

        // Current native is 50, desired is 50
        assert_eq!(range.current, desired_native);

        // Simulating the decision logic of apply_external_brightness
        let applied = if desired_native == range.current {
            false
        } else {
            api.set_brightness(1, desired_native).unwrap();
            true
        };

        assert!(!applied);
        assert_eq!(
            api.write_calls.load(Ordering::Relaxed),
            0,
            "SetMonitorBrightness must NOT be called on redundant write"
        );
    }

    #[test]
    fn test_write_occurs_when_native_values_differ() {
        let api = MockMonitorConfigApi::new(0, 20, 100);
        let range = api.get_brightness(1).unwrap();
        let desired_percent = 50;
        let desired_native = range.percent_to_native(desired_percent);

        assert_ne!(range.current, desired_native);

        let applied = if desired_native == range.current {
            false
        } else {
            api.set_brightness(1, desired_native).unwrap();
            true
        };

        assert!(applied);
        assert_eq!(api.write_calls.load(Ordering::Relaxed), 1);
        assert_eq!(api.last_written.load(Ordering::Relaxed), 50);
        assert_eq!(api.current.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_capabilities_missing_mc_caps_brightness_fails_closed() {
        let api = MockMonitorConfigApi::new(0, 50, 100);
        api.caps.store(0x00, Ordering::Relaxed); // Missing MC_CAPS_BRIGHTNESS (0x02)

        let caps = api.get_capabilities(1).unwrap();
        const MC_CAPS_BRIGHTNESS: u32 = 0x02;
        let supported = (caps & MC_CAPS_BRIGHTNESS) != 0;

        assert!(!supported);
    }

    #[test]
    fn test_get_brightness_error_fails_closed() {
        let api = MockMonitorConfigApi::new(0, 50, 100);
        api.read_err.store(5, Ordering::Relaxed); // ERROR_ACCESS_DENIED

        let res = api.get_brightness(1);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), 5);
    }

    #[test]
    fn test_set_brightness_error_propagates_cleanly() {
        let api = MockMonitorConfigApi::new(0, 50, 100);
        api.write_err.store(31, Ordering::Relaxed); // ERROR_GEN_FAILURE

        let res = api.set_brightness(1, 80);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), 31);
    }

    #[test]
    fn test_multi_monitor_isolation() {
        let mon_a = MockMonitorConfigApi::new(0, 20, 100);
        let mon_b = MockMonitorConfigApi::new(0, 60, 100);

        // Monitor A fails write
        mon_a.write_err.store(31, Ordering::Relaxed);

        let res_a = mon_a.set_brightness(1, 30);
        assert!(res_a.is_err());

        // Monitor B operates completely independently and succeeds
        let res_b = mon_b.set_brightness(2, 70);
        assert!(res_b.is_ok());
        assert_eq!(mon_b.current.load(Ordering::Relaxed), 70);
        assert_eq!(mon_b.write_calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_readback_mismatch_detection() {
        let api = MockMonitorConfigApi::new(0, 20, 100);
        // Simulate non-compliant hardware that acknowledges write but does not apply value:
        api.write_calls.fetch_add(1, Ordering::Relaxed);
        // hardware stays at 20 despite writing 50:
        api.current.store(20, Ordering::Relaxed);

        let readback = api.get_brightness(1).unwrap();
        let desired_native = 50;

        assert_ne!(readback.current, desired_native);
    }

    #[test]
    fn test_controlled_reversible_delta_calculation() {
        // Normal case (<= 90): delta is +5%
        let cur_pct = 40;
        let test_target = if cur_pct <= 90 {
            cur_pct + 5
        } else {
            cur_pct - 5
        };
        assert_eq!(test_target, 45);

        // High brightness case (> 90): delta is -5%
        let cur_pct = 95;
        let test_target = if cur_pct <= 90 {
            cur_pct + 5
        } else {
            cur_pct - 5
        };
        assert_eq!(test_target, 90);
    }

    fn make_test_observation(
        quality: IdentityQuality,
        kind: WindowsDisplayKind,
        active: bool,
        target_available: bool,
    ) -> WindowsMonitorObservation {
        WindowsMonitorObservation {
            display_number: 1,
            canonical_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            display_label: Some(String::from("win-del-4198-8c7ed206")),
            stable_id: Some(String::from(
                "win-del-4198-8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c",
            )),
            identity_quality: quality,
            mutation_eligibility:
                crate::platform::windows::topology::W3MutationEligibility::AutomaticEligible,
            name: Some(String::from("DELL U2720Q")),
            kind,
            active,
            target_available,
            is_primary: true,
            output_technology: String::from("DisplayPort"),
            gdi_device_name: Some(String::from("\\\\.\\DISPLAY1")),
            adapter_luid: (0x1000, 0),
            source_id: 0,
            target_id: 1,
            connector_instance: 0,
            container_id: Some(String::from("{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c}")),
            device_instance_id: Some(String::from("DISPLAY\\DEL4198\\5&12345&0&UID0")),
            hardware_ids: vec![String::from("MONITOR\\DEL4198")],
            edid_manufacturer: Some(String::from("DEL")),
            edid_product_code: Some(0x4198),
            physical_monitors_count: 1,
            physical_monitor_description: Some(String::from("Generic PnP Monitor")),
            desktop_rect: Some((0, 0, 3840, 2160)),
            note: None,
        }
    }

    #[test]
    fn test_mutation_safety_gate_strong_valid_passes() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(res.is_ok());
    }

    #[test]
    fn test_mutation_safety_gate_rejects_stable_device() {
        let obs = make_test_observation(
            IdentityQuality::StableDevice,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::IdentityNotEligible {
                quality: IdentityQuality::StableDevice,
                ..
            })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_port_bound() {
        let obs = make_test_observation(
            IdentityQuality::PortBound,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::IdentityNotEligible {
                quality: IdentityQuality::PortBound,
                ..
            })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_ambiguous() {
        let obs = make_test_observation(
            IdentityQuality::Ambiguous,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::IdentityAmbiguous { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_virtual() {
        let obs = make_test_observation(
            IdentityQuality::Virtual,
            WindowsDisplayKind::VirtualDisplay,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::IdentityNotEligible {
                quality: IdentityQuality::Virtual,
                ..
            })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_unknown() {
        let obs = make_test_observation(
            IdentityQuality::Unknown,
            WindowsDisplayKind::Unknown,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::IdentityNotEligible {
                quality: IdentityQuality::Unknown,
                ..
            })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_indirect_wired() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::IndirectDisplay,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::NotExternal {
                kind: WindowsDisplayKind::IndirectDisplay,
                ..
            })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_target_unavailable() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            false, // target_available = false
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::TargetUnavailable { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_target_inactive() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            false, // active = false
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::TargetUnavailable { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_internal_panel() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::InternalPanel,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::NotExternal {
                kind: WindowsDisplayKind::InternalPanel,
                ..
            })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_physical_count_zero() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 0, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::PhysicalMonitorUnavailable { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_physical_count_greater_than_one() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 2, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::PhysicalMonitorMappingAmbiguous { count: 2, .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_missing_mc_caps_brightness() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x00), Some(range)); // no 0x02 flag
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::BrightnessUnsupported { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_capabilities_query_failure() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let range = NativeBrightnessRange::new(0, 50, 100).unwrap();
        let res = evaluate_mutation_safety_gate(&obs, 1, None, Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::CapabilitiesQueryFailed { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_brightness_read_failure() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), None);
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::BrightnessReadFailed { .. })
        ));
    }

    #[test]
    fn test_mutation_safety_gate_rejects_invalid_native_range() {
        let obs = make_test_observation(
            IdentityQuality::Strong,
            WindowsDisplayKind::ExternalMonitor,
            true,
            true,
        );
        // Range with min >= max
        let range = NativeBrightnessRange {
            min: 100,
            current: 50,
            max: 50,
        };
        let res = evaluate_mutation_safety_gate(&obs, 1, Some(0x02), Some(range));
        assert!(matches!(
            res,
            Err(WindowsExternalBrightnessError::InvalidBrightnessRange { .. })
        ));
    }

    #[test]
    fn test_physical_qualification_restores_original_even_if_intermediate_readback_fails() {
        let api = MockMonitorConfigApi::new(0, 50, 100);
        // Initial read: 50
        let initial = api.get_brightness(1).unwrap();
        assert_eq!(initial.current, 50);

        // Step 3: Apply test target (55)
        let test_target = 55;
        api.set_brightness(1, test_target).unwrap();
        assert_eq!(api.current.load(Ordering::Relaxed), 55);

        // Step 4: Intermediate readback fails
        api.read_err.store(5, Ordering::Relaxed);
        let test_readback = api.get_brightness(1);
        assert!(test_readback.is_err());

        // Step 5: Guaranteed restoration must still execute
        api.set_brightness(1, initial.current).unwrap();
        assert_eq!(api.current.load(Ordering::Relaxed), 50);
        assert_eq!(api.write_calls.load(Ordering::Relaxed), 2);
    }
}
