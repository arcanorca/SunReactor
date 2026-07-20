pub(crate) mod model;
mod probe;
mod render;
mod runner;

use std::path::Path;

use crate::backends::BackendKind;

pub use model::{
    BackendStatus, BackendStatusKind, BacklightDeviceDiscovery, DdcMonitorDiscovery,
    DiscoveryBackends, DiscoveryReport, DiscoverySummary, TargetDescriptor, TargetKind,
};

use runner::RealProcessRunner;

const SYSFS_BACKLIGHT_ROOT: &str = "/sys/class/backlight";

#[must_use]
pub fn discover() -> DiscoveryReport {
    discover_with_runner(&RealProcessRunner, Path::new(SYSFS_BACKLIGHT_ROOT))
}

#[must_use]
pub fn discover_targets() -> Vec<TargetDescriptor> {
    discover().viable_targets()
}

impl DiscoveryReport {
    #[must_use]
    pub fn render_human(&self) -> String {
        render::render_human(self)
    }

    #[must_use]
    pub fn render_json(&self) -> String {
        render::render_json(self)
    }

    #[must_use]
    pub fn viable_targets(&self) -> Vec<TargetDescriptor> {
        let mut targets = Vec::new();

        for monitor in &self.ddc_monitors {
            if monitor.backend_viable {
                targets.push(TargetDescriptor {
                    id: monitor.stable_id.clone(),
                    label: monitor.target_label(),
                    kind: TargetKind::ExternalMonitor,
                    backend: BackendKind::Ddc,
                });
            }
        }

        for device in &self.backlight_devices {
            if device.backend_viable {
                targets.push(TargetDescriptor {
                    id: device.stable_id.clone(),
                    label: device.device_name.clone(),
                    kind: TargetKind::InternalPanel,
                    backend: BackendKind::Backlight,
                });
            }
        }

        targets
    }
}

pub(crate) fn discover_with_runner<R: runner::ProcessRunner>(
    runner: &R,
    sysfs_root: &Path,
) -> DiscoveryReport {
    let snapshot = probe::discover_with_runner(runner, sysfs_root);
    render::build_report(snapshot)
}

#[cfg(test)]
pub(crate) use runner::{CommandError, CommandOutput, ProcessRunner};

#[cfg(test)]
mod tests;
