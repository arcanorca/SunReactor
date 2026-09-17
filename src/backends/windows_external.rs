//! Windows native external monitor brightness backend implementation.
//!
//! Bridges native DXVA2 Monitor Configuration APIs to SunReactor's apply/reconcile engine.

use crate::backends::{BackendError, BackendKind, BackendObservation, BackendWrite};
use crate::config::MonitorConfig;
use crate::platform::windows::brightness::{
    apply_external_brightness_with_api, read_external_brightness_with_api, Win32MonitorConfigApi,
    WindowsExternalBrightnessError,
};

/// Applies brightness to a Windows external monitor using native DXVA2 high-level APIs.
pub(crate) fn apply_windows_external_brightness(
    monitor: &MonitorConfig,
    applied_percent: u8,
) -> Result<BackendWrite, BackendError> {
    let canonical_id = &monitor.logical_id;
    let api = Win32MonitorConfigApi;

    match apply_external_brightness_with_api(&api, canonical_id, applied_percent, false) {
        Ok(result) => {
            let detail = if result.applied {
                format!(
                    "DXVA2 SetMonitorBrightness: applied {} native (was {}) on {canonical_id}",
                    result.desired_native, result.previous_native
                )
            } else {
                format!(
                    "DXVA2 redundant write suppressed: hardware already at native {} on {canonical_id}",
                    result.desired_native
                )
            };
            Ok(BackendWrite {
                backend: BackendKind::Ddc,
                applied_percent,
                attempts: 1,
                detail,
            })
        }
        Err(err) => Err(map_windows_error_to_backend_error(canonical_id, &err)),
    }
}

/// Reads current brightness from a Windows external monitor using native DXVA2 high-level APIs.
pub(crate) fn read_windows_external_brightness(
    monitor: &MonitorConfig,
) -> Result<BackendObservation, BackendError> {
    let canonical_id = &monitor.logical_id;
    let api = Win32MonitorConfigApi;

    match read_external_brightness_with_api(&api, canonical_id) {
        Ok((_range, pct)) => Ok(BackendObservation { percent: pct }),
        Err(err) => Err(map_windows_error_to_backend_error(canonical_id, &err)),
    }
}

fn map_windows_error_to_backend_error(
    _canonical_id: &str,
    err: &WindowsExternalBrightnessError,
) -> BackendError {
    match err {
        WindowsExternalBrightnessError::IdentityNotFound { .. }
        | WindowsExternalBrightnessError::IdentityNotEligible { .. }
        | WindowsExternalBrightnessError::IdentityAmbiguous { .. }
        | WindowsExternalBrightnessError::NotExternal { .. }
        | WindowsExternalBrightnessError::BrightnessUnsupported { .. }
        | WindowsExternalBrightnessError::InvalidBrightnessRange { .. }
        | WindowsExternalBrightnessError::PhysicalMonitorMappingAmbiguous { .. } => {
            BackendError::InvalidSelector {
                backend: BackendKind::Ddc,
                field: "canonical_id",
                message: err.to_string(),
            }
        }
        WindowsExternalBrightnessError::TargetUnavailable { .. }
        | WindowsExternalBrightnessError::PhysicalMonitorUnavailable { .. }
        | WindowsExternalBrightnessError::TopologyChanged { .. }
        | WindowsExternalBrightnessError::DisplayQuery(..) => BackendError::CommandFailed {
            backend: BackendKind::Ddc,
            program: String::from("DXVA2"),
            exit_code: None,
            detail: err.to_string(),
            transient: true,
            attempts: 1,
        },
        WindowsExternalBrightnessError::CapabilitiesQueryFailed { code, .. }
        | WindowsExternalBrightnessError::BrightnessReadFailed { code, .. }
        | WindowsExternalBrightnessError::BrightnessWriteFailed { code, .. } => {
            BackendError::CommandFailed {
                backend: BackendKind::Ddc,
                program: String::from("DXVA2"),
                exit_code: Some(*code as i32),
                detail: err.to_string(),
                transient: false,
                attempts: 1,
            }
        }
        WindowsExternalBrightnessError::ReadbackMismatch { .. } => BackendError::Io {
            backend: BackendKind::Ddc,
            program: String::from("DXVA2/Readback"),
            message: err.to_string(),
            attempts: 1,
        },
    }
}
