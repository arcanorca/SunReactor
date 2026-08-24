mod model;
mod probe;
mod render;
mod runner;

use std::path::Path;

use crate::backends::BackendKind;

pub use model::{
    BackendStatus, BackendStatusKind, BacklightDeviceDiscovery, DdcMonitorDiscovery,
    DiscoveryBackends, DiscoveryReport, DiscoverySummary, PhysicalIdentityStatus, TargetDescriptor,
    TargetKind,
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
                    id: monitor.occurrence_id.clone(),
                    label: monitor.target_label(),
                    kind: TargetKind::ExternalMonitor,
                    backend: BackendKind::Ddc,
                });
            }
        }

        for device in &self.backlight_devices {
            if device.backend_viable && !is_ddcci_alias_of_viable_ddc(device, &self.ddc_monitors) {
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

/// Returns true when a backlight device is topology-aliased to the same
/// physical monitor as at least one *viable* DDC monitor.
///
/// This is the only cross-backend duplication signal SunReactor accepts.
/// It requires explicit evidence: `ddcci_connector` (the proven kernel/driver
/// `ddcci_backlight` symlink convergence) must match a DDC monitor's DRM
/// connector. Names, ordering, and backlight type are never evidence.
pub(crate) fn is_ddcci_alias_of_viable_ddc(
    backlight: &BacklightDeviceDiscovery,
    ddc_monitors: &[DdcMonitorDiscovery],
) -> bool {
    let Some(connector) = backlight.ddcci_connector.as_deref() else {
        return false;
    };
    ddc_monitors
        .iter()
        .any(|monitor| monitor.backend_viable && monitor.connector.as_deref() == Some(connector))
}

pub(crate) fn assign_ddc_occurrence_ids(monitors: &mut [DdcMonitorDiscovery]) {
    let selectors = monitors.iter().map(discovery_selector).collect::<Vec<_>>();

    for (index, monitor) in monitors.iter_mut().enumerate() {
        let topology = monitor
            .bus_number
            .map(|bus| format!("bus-{bus}"))
            .or_else(|| monitor.connector.as_deref().map(model::slugify))
            .unwrap_or_else(|| format!("display-{index}"));
        monitor.occurrence_id = format!("ddc-occurrence:{topology}:record-{index}");

        let ambiguous = selectors.iter().enumerate().any(|(other_index, other)| {
            other_index != index
                && crate::backends::ddc::selector_relation(&selectors[index], other)
                    != Ok(crate::backends::ddc::DdcSelectorRelation::ProvablyDisjoint)
        });

        monitor.identity_status = if ambiguous {
            PhysicalIdentityStatus::Ambiguous
        } else if monitor.serial.is_some() || monitor.model.is_some() {
            PhysicalIdentityStatus::Unique
        } else if monitor.bus_number.is_some() || monitor.connector.is_some() {
            PhysicalIdentityStatus::TopologyBound
        } else {
            PhysicalIdentityStatus::Insufficient
        };
    }
}

fn discovery_selector(monitor: &DdcMonitorDiscovery) -> crate::config::MonitorSelector {
    crate::config::MonitorSelector {
        connector: monitor.connector.clone(),
        serial: monitor.serial.clone(),
        model: monitor.model.clone(),
        edid: None,
        sysfs_path: None,
        ddc_bus: if monitor.serial.is_none() && monitor.model.is_none() {
            monitor.bus_number.map(|bus| bus as u8)
        } else {
            None
        },
        ddc_address: None,
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
pub(crate) fn discover_with_roots<R: runner::ProcessRunner>(
    runner: &R,
    sysfs_root: &Path,
    drm_root: &Path,
) -> DiscoveryReport {
    let snapshot = probe::discover_with_roots(runner, sysfs_root, drm_root);
    render::build_report(snapshot)
}

#[cfg(test)]
pub(crate) use runner::{CommandError, CommandOutput, ProcessRunner};

#[cfg(test)]
mod tests;
