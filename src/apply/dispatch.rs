use super::types::ApplySettings;
#[cfg(not(target_os = "windows"))]
use crate::backends;
use crate::backends::{BackendError, BackendKind, BackendWrite, ProcessRunner};
use crate::config::MonitorConfig;
use crate::policy::PerMonitorTarget;

pub(crate) fn apply_monitor_target<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    target: &PerMonitorTarget,
    applied_percent: u8,
    settings: &ApplySettings,
) -> Result<BackendWrite, BackendError> {
    debug_assert_eq!(monitor.logical_id, target.logical_id);

    #[cfg(target_os = "windows")]
    {
        let _ = (runner, target, settings);
        match monitor.backend {
            BackendKind::Ddc => {
                crate::backends::windows_external::apply_windows_external_brightness(
                    monitor,
                    applied_percent,
                )
            }
            BackendKind::Backlight => Err(BackendError::Io {
                backend: BackendKind::Backlight,
                program: String::from("WMI/WinRT"),
                message: String::from("Internal laptop panel brightness deferred to W3B"),
                attempts: 0,
            }),
        }
    }

    #[cfg(not(target_os = "windows"))]
    match monitor.backend {
        BackendKind::Backlight => backends::backlight::apply_with_runner(
            runner,
            monitor,
            applied_percent,
            settings.backlight_timeout,
        ),
        BackendKind::Ddc => {
            backends::ddc::apply_with_runner(runner, monitor, applied_percent, settings.ddc_timeout)
        }
    }
}

pub(crate) fn read_monitor_percent<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    settings: &ApplySettings,
) -> Result<crate::backends::BackendObservation, BackendError> {
    #[cfg(target_os = "windows")]
    {
        let _ = (runner, settings);
        match monitor.backend {
            BackendKind::Ddc => {
                crate::backends::windows_external::read_windows_external_brightness(monitor)
            }
            BackendKind::Backlight => Err(BackendError::Io {
                backend: BackendKind::Backlight,
                program: String::from("WMI/WinRT"),
                message: String::from("Internal laptop panel brightness deferred to W3B"),
                attempts: 0,
            }),
        }
    }

    #[cfg(not(target_os = "windows"))]
    match monitor.backend {
        BackendKind::Backlight => {
            backends::backlight::read_with_runner(runner, monitor, settings.backlight_timeout)
        }
        BackendKind::Ddc => backends::ddc::read_with_runner(runner, monitor, settings.ddc_timeout),
    }
}

/// A quick brightness read for wake probes: DDC monitors only over their
/// verified bus, backlights through sysfs. `Ok(None)` means the monitor cannot
/// be probed cheaply and is left to regular ticks.
pub(crate) fn probe_monitor_percent<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    settings: &ApplySettings,
) -> Result<Option<crate::backends::BackendObservation>, BackendError> {
    #[cfg(target_os = "windows")]
    {
        let _ = (runner, monitor, settings);
        Ok(None)
    }

    #[cfg(not(target_os = "windows"))]
    match monitor.backend {
        BackendKind::Backlight => {
            backends::backlight::read_with_runner(runner, monitor, settings.backlight_timeout)
                .map(Some)
        }
        BackendKind::Ddc => {
            backends::ddc::read_verified_bus_with_runner(runner, monitor, settings.ddc_timeout)
        }
    }
}
