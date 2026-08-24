use std::collections::HashSet;

use super::model::{
    slugify, BackendStatus, BackendStatusKind, BacklightDeviceDiscovery, DdcMonitorDiscovery,
    DiscoveryReport, DiscoverySnapshot, DiscoverySummary,
};
use crate::backends::ddc::{
    selector_match_count, selector_plan, selector_relation, DdcSelectorRelation,
};
use crate::config::MonitorSelector;

pub(super) fn build_report(snapshot: DiscoverySnapshot) -> DiscoveryReport {
    let mut notes = build_notes(
        &snapshot.backends,
        &snapshot.summary,
        &snapshot.backlight_devices,
    );
    let (config_snippet, config_notes) =
        build_config_snippet(&snapshot.ddc_monitors, &snapshot.backlight_devices);
    notes.extend(config_notes);

    DiscoveryReport {
        summary: snapshot.summary,
        backends: snapshot.backends,
        ddc_monitors: snapshot.ddc_monitors,
        backlight_devices: snapshot.backlight_devices,
        notes,
        config_snippet,
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn render_human(report: &DiscoveryReport) -> String {
    let backend_rows = vec![
        vec![
            report.backends.ddcutil.backend.clone(),
            backend_status_label(report.backends.ddcutil.status).to_owned(),
            backend_detail(&report.backends.ddcutil),
        ],
        vec![
            report.backends.brightnessctl.backend.clone(),
            backend_status_label(report.backends.brightnessctl.status).to_owned(),
            backend_detail(&report.backends.brightnessctl),
        ],
        vec![
            report.backends.sysfs.backend.clone(),
            backend_status_label(report.backends.sysfs.status).to_owned(),
            backend_detail(&report.backends.sysfs),
        ],
    ];

    let ddc_rows = report
        .ddc_monitors
        .iter()
        .map(|monitor| {
            vec![
                monitor.display_number.to_string(),
                display_opt_u32(monitor.bus_number),
                display_opt(&monitor.manufacturer),
                display_opt(&monitor.model),
                display_opt(&monitor.serial),
                display_opt(&monitor.connector),
                display_opt_bool(monitor.brightness_vcp_supported),
                display_bool(monitor.backend_viable),
                display_opt(&monitor.note),
            ]
        })
        .collect::<Vec<_>>();

    let backlight_rows = report
        .backlight_devices
        .iter()
        .map(|device| {
            vec![
                device.device_name.clone(),
                device.class.clone(),
                device
                    .backlight_type
                    .clone()
                    .unwrap_or_else(|| String::from("-")),
                display_opt_u32(device.max_brightness),
                device.probe_source.clone(),
                display_bool(device.backend_viable),
                device.sysfs_path.clone(),
                display_opt(&device.note),
            ]
        })
        .collect::<Vec<_>>();

    let mut sections = Vec::new();
    sections.push(format!(
        "Discovery summary\nviable targets: {}\nexternal monitors: {}\nbacklight devices: {}",
        report.summary.viable_targets,
        report.summary.ddc_monitors,
        report.summary.backlight_devices,
    ));
    sections.push(format!(
        "Backend status\n{}",
        render_table(&["backend", "status", "detail"], &backend_rows)
    ));

    if ddc_rows.is_empty() {
        sections.push(String::from("DDC monitors\nNo external monitors reported."));
    } else {
        sections.push(format!(
            "DDC monitors\n{}",
            render_table(
                &[
                    "display",
                    "bus",
                    "manufacturer",
                    "model",
                    "serial",
                    "connector",
                    "vcp_0x10",
                    "viable",
                    "note",
                ],
                &ddc_rows,
            )
        ));
    }

    if backlight_rows.is_empty() {
        sections.push(String::from(
            "Backlight devices\nNo internal backlight devices reported.",
        ));
    } else {
        sections.push(format!(
            "Backlight devices\n{}",
            render_table(
                &[
                    "device",
                    "class",
                    "type",
                    "max",
                    "source",
                    "viable",
                    "sysfs_path",
                    "note",
                ],
                &backlight_rows,
            )
        ));
    }

    if !report.notes.is_empty() {
        sections.push(format!("Notes\n{}", report.notes.join("\n")));
    }

    sections.push(format!(
        "Candidate config snippet\n{}",
        report.config_snippet
    ));
    sections.join("\n\n")
}

pub(super) fn render_json(report: &DiscoveryReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| String::from("{}"))
}

fn build_notes(
    backends: &super::DiscoveryBackends,
    summary: &DiscoverySummary,
    backlight_devices: &[BacklightDeviceDiscovery],
) -> Vec<String> {
    let mut notes = Vec::new();

    if matches!(backends.brightnessctl.status, BackendStatusKind::Missing)
        && !backlight_devices.is_empty()
    {
        notes.push(String::from(
            "brightnessctl is missing; internal panel discovery used sysfs fallback where available.",
        ));
    }

    if summary.viable_targets == 0 {
        notes.push(String::from(
            "No brightness-capable devices were discovered. Install `ddcutil` for external monitors and `brightnessctl` for internal panels, or ensure `/sys/class/backlight` exposes a usable device.",
        ));
    }

    notes
}

#[allow(clippy::too_many_lines)]
fn build_config_snippet(
    ddc_monitors: &[DdcMonitorDiscovery],
    backlight_devices: &[BacklightDeviceDiscovery],
) -> (String, Vec<String>) {
    let mut blocks = Vec::new();
    let mut used_ids = HashSet::new();
    let mut ambiguity_notes = Vec::new();
    let candidates = ddc_monitors
        .iter()
        .filter(|monitor| monitor.backend_viable)
        .map(|monitor| (monitor, discovery_selector(monitor)))
        .collect::<Vec<_>>();
    let mut withheld = HashSet::new();
    for (candidate_index, (monitor, selector)) in candidates.iter().enumerate() {
        let match_count = selector_match_count(selector, ddc_monitors);
        if match_count != Ok(1) {
            withheld.insert(candidate_index);
            ambiguity_notes.push(format!(
                "Withheld enabled DDC config for {}: selector is not unique in the current discovery set (matched {:?} monitor(s)).",
                monitor.target_label(),
                match_count.ok()
            ));
        }
    }
    for (left_index, (_, left)) in candidates.iter().enumerate() {
        for (right_index, (_, right)) in candidates.iter().enumerate().skip(left_index + 1) {
            match selector_relation(left, right) {
                Ok(DdcSelectorRelation::ProvablyDisjoint) => {}
                Ok(relation) => {
                    withheld.insert(left_index);
                    withheld.insert(right_index);
                    ambiguity_notes.push(format!(
                        "Withheld enabled DDC config for {} and {}: selectors may overlap ({relation:?}); neither pair is proven to address a distinct display.",
                        candidates[left_index].0.target_label(),
                        candidates[right_index].0.target_label()
                    ));
                }
                Err(error) => {
                    withheld.insert(left_index);
                    withheld.insert(right_index);
                    ambiguity_notes.push(format!(
                        "Withheld enabled DDC config for {} and {}: selector comparison failed ({error}).",
                        candidates[left_index].0.target_label(),
                        candidates[right_index].0.target_label()
                    ));
                }
            }
        }
    }

    for (candidate_index, (monitor, selector)) in candidates.iter().enumerate() {
        if withheld.contains(&candidate_index) {
            continue;
        }
        let logical_id = allocate_logical_id(ddc_logical_id_base(monitor), &mut used_ids);
        let effective_bus_only = selector_plan(selector).is_ok_and(|plan| plan.is_bus_only());
        let enabled = !effective_bus_only;
        if effective_bus_only {
            ambiguity_notes.push(format!(
                "Generated disabled DDC candidate for {}: bus-only selection follows an I2C topology slot; enable only with allow_topology_retargeting=true.",
                monitor.target_label()
            ));
        }
        let mut lines = vec![
            String::from("[[monitors]]"),
            format!("logical_id = \"{logical_id}\""),
            String::from("backend = \"ddc\""),
            format!("enabled = {enabled}"),
            String::from("allow_topology_retargeting = false"),
            String::from(
                "# Bus-only selectors follow a topology slot; opt in explicitly before enabling.",
            ),
            String::from("min_pct = 0"),
            String::from("max_pct = 100"),
            String::from("gain = 1.0"),
        ];

        if let Some(model) = &monitor.model {
            lines.push(format!("model = \"{}\"", escape_toml_string(model)));
        }
        if let Some(serial) = &monitor.serial {
            lines.push(format!("serial = \"{}\"", escape_toml_string(serial)));
        }
        if let Some(connector) = &monitor.connector {
            lines.push(format!("connector = \"{}\"", escape_toml_string(connector)));
        }
        if let Some(bus_number) = monitor.bus_number {
            lines.push(format!("ddc_bus = {bus_number}"));
        }

        blocks.push(lines.join("\n"));
    }

    for device in backlight_devices.iter().filter(|device| {
        device.backend_viable && !super::is_ddcci_alias_of_viable_ddc(device, ddc_monitors)
    }) {
        let logical_id = allocate_logical_id(backlight_logical_id_base(device), &mut used_ids);
        let lines = [
            String::from("[[monitors]]"),
            format!("logical_id = \"{logical_id}\""),
            String::from("backend = \"backlight\""),
            String::from("enabled = true"),
            String::from("min_pct = 0"),
            String::from("max_pct = 100"),
            String::from("gain = 1.0"),
            format!(
                "sysfs_path = \"{}\"",
                escape_toml_string(&device.sysfs_path)
            ),
        ];
        blocks.push(lines.join("\n"));
    }

    if blocks.is_empty() {
        (
            String::from("# No viable brightness-capable devices were discovered."),
            ambiguity_notes,
        )
    } else {
        (blocks.join("\n\n"), ambiguity_notes)
    }
}

fn discovery_selector(monitor: &DdcMonitorDiscovery) -> MonitorSelector {
    MonitorSelector {
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

fn allocate_logical_id(base: String, used_ids: &mut HashSet<String>) -> String {
    if used_ids.insert(base.clone()) {
        return base;
    }

    let mut index = 2usize;
    loop {
        let candidate = format!("{base}-{index}");
        if used_ids.insert(candidate.clone()) {
            return candidate;
        }
        index += 1;
    }
}

fn ddc_logical_id_base(monitor: &DdcMonitorDiscovery) -> String {
    if let Some(serial) = &monitor.serial {
        let model = monitor
            .model
            .as_deref()
            .map(slugify)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| String::from("monitor"));
        let serial = slugify(serial);
        if serial.is_empty() {
            return model;
        }
        return format!("{model}-{serial}");
    }

    monitor
        .model
        .as_deref()
        .map(slugify)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("display-{}", monitor.display_number))
}

fn backlight_logical_id_base(device: &BacklightDeviceDiscovery) -> String {
    let slug = slugify(&device.device_name);
    if slug.is_empty() {
        String::from("backlight")
    } else {
        slug
    }
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn display_opt(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| String::from("-"))
}

fn display_opt_u32(value: Option<u32>) -> String {
    value.map_or_else(|| String::from("-"), |value| value.to_string())
}

fn display_opt_bool(value: Option<bool>) -> String {
    match value {
        Some(true) => String::from("yes"),
        Some(false) => String::from("no"),
        None => String::from("unknown"),
    }
}

fn display_bool(value: bool) -> String {
    if value {
        String::from("yes")
    } else {
        String::from("no")
    }
}

fn backend_status_label(status: BackendStatusKind) -> &'static str {
    match status {
        BackendStatusKind::Ok => "ok",
        BackendStatusKind::Missing => "missing",
        BackendStatusKind::Timeout => "timeout",
        BackendStatusKind::Error => "error",
        BackendStatusKind::Unavailable => "unavailable",
    }
}

fn backend_detail(status: &BackendStatus) -> String {
    match &status.guidance {
        Some(guidance) => format!("{} {}", status.message, guidance),
        None => status.message.clone(),
    }
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<_>>();

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            let width = cell.chars().count();
            if width > widths[index] {
                widths[index] = width;
            }
        }
    }

    let mut lines = Vec::new();
    lines.push(render_row(
        &headers
            .iter()
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>(),
        &widths,
    ));
    lines.push(render_row(
        &widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>(),
        &widths,
    ));

    for row in rows {
        lines.push(render_row(row, &widths));
    }

    lines.join("\n")
}

fn render_row(cells: &[String], widths: &[usize]) -> String {
    cells
        .iter()
        .enumerate()
        .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
        .collect::<Vec<_>>()
        .join("  ")
}
