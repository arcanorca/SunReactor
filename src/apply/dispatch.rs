use super::types::ApplySettings;
use crate::backends::{self, BackendError, BackendKind, BackendWrite, ProcessRunner};
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
