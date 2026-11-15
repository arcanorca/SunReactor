use std::collections::HashSet;

use super::model::{
    slugify, BackendStatus, BackendStatusKind, BacklightDeviceDiscovery, DdcMonitorDiscovery,
    DiscoveryReport, DiscoverySnapshot, DiscoverySummary,
};
use crate::backends::ddc::{
    selector_match_count, selector_plan, selector_relation, DdcSelectorRelation,
};
use crate::config::{MonitorConfig, MonitorSelector};

pub(super) fn build_report(snapshot: DiscoverySnapshot) -> DiscoveryReport {
    let mut notes = build_notes(
        &snapshot.backends,
        &snapshot.summary,
        &snapshot.backlight_devices,
    );
    let (config_candidates, config_notes) =
        build_monitor_config_candidates(&snapshot.ddc_monitors, &snapshot.backlight_devices);
    let config_snippet = render_config_snippet(&config_candidates);
    notes.extend(config_notes);

    DiscoveryReport {
        summary: snapshot.summary,
        backends: snapshot.backends,
        ddc_observation_complete: snapshot.ddc_observation_complete,
        ddc_monitors: snapshot.ddc_monitors,
        backlight_devices: snapshot.backlight_devices,
        windows_displays: Vec::new(),
        notes,
        config_snippet,
    }
}

/// Returns only candidates which discovery has proved safe to enable without
/// asking the user to opt into topology retargeting.
#[cfg(any(feature = "tui", test))]
pub(super) fn importable_monitor_configs(report: &DiscoveryReport) -> Vec<MonitorConfig> {
    build_monitor_config_candidates(&report.ddc_monitors, &report.backlight_devices)
        .0
        .into_iter()
        .filter(|monitor| monitor.enabled)
        .collect()
}

// Linear, table-like report rendering; splitting would only scatter the layout.
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

    if !report.windows_displays.is_empty() {
        use std::fmt::Write;
        let mut win_section = String::from("Windows display topology\n");
        for disp in &report.windows_displays {
            let primary_tag = if disp.is_primary { " (Primary)" } else { "" };
            let title = disp.name.as_deref().unwrap_or("Display");
            let num = disp.display_number;
            let _ = writeln!(win_section, "  [{num}] {title}{primary_tag}");
            let _ = writeln!(
                win_section,
                "      Kind:              {} ({})",
                disp.kind, disp.output_technology
            );
            let status = if disp.active && disp.target_available {
                "Active / Connected"
            } else if disp.active {
                "Active / Temporarily Unavailable"
            } else {
                "Inactive"
            };
            let _ = writeln!(win_section, "      Status:            {status}");
            if let Some(ref gdi) = disp.gdi_device_name {
                let rect = disp
                    .desktop_rect
                    .as_deref()
                    .unwrap_or("coordinates unknown");
                let _ = writeln!(win_section, "      GDI Device:        {gdi} ({rect})");
            }
            let _ = writeln!(
                win_section,
                "      CCD Path:          Adapter {} / Target {} (Connector #{})",
                disp.adapter_luid, disp.target_id, disp.connector_instance
            );
            let _ = writeln!(
                win_section,
                "      Physical Monitors: {} (Handles transient & destroyed via RAII)",
                disp.physical_monitors_count
            );
            if let Some(ref cid) = disp.container_id {
                let _ = writeln!(win_section, "      Container ID:      {cid}");
            }
            if let Some(ref inst) = disp.device_instance_id {
                let _ = writeln!(win_section, "      Device Instance:   {inst}");
            }
            let _ = writeln!(
                win_section,
                "      Identity Quality:  {}",
                disp.identity_quality
            );
            let _ = writeln!(
                win_section,
                "      W3 Mutation Tier:  {}",
                disp.mutation_eligibility
            );
            if let Some(ref cid) = disp.canonical_id {
                let _ = writeln!(win_section, "      Canonical ID:      {cid}");
            }
            if let Some(ref label) = disp.display_label {
                let _ = writeln!(win_section, "      Display Label:     {label}");
            }
            if let Some(ref capable) = disp.brightness_capable {
                if *capable {
                    let min = disp.native_min.unwrap_or(0);
                    let cur = disp.native_current.unwrap_or(0);
                    let max = disp.native_max.unwrap_or(100);
                    let pct = disp.normalized_current_percent.unwrap_or(0);
                    let _ = writeln!(
                        win_section,
                        "      Brightness Backend: Capable (DXVA2 High-Level, Native: {min}..{max}, Current: {cur} [{pct}%])"
                    );
                } else if let Some(ref note) = disp.capability_note {
                    let _ = writeln!(win_section, "      Brightness Backend: {note}");
                }
            } else if let Some(ref note) = disp.capability_note {
                let _ = writeln!(win_section, "      Brightness Backend: {note}");
            }
            if let Some(ref note) = disp.note {
                let _ = writeln!(win_section, "      Note:              {note}");
            }
            win_section.push('\n');
        }
        sections.push(win_section.trim_end().to_string());
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

// Linear TOML snippet emission, one block per candidate kind.
#[allow(clippy::too_many_lines)]
fn build_monitor_config_candidates(
    ddc_monitors: &[DdcMonitorDiscovery],
    backlight_devices: &[BacklightDeviceDiscovery],
) -> (Vec<MonitorConfig>, Vec<String>) {
    let mut configs = Vec::new();
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
        configs.push(MonitorConfig {
            logical_id,
            backend: crate::backends::BackendKind::Ddc,
            enabled,
            allow_topology_retargeting: false,
            min_pct: 0,
            max_pct: 100,
            gain: 1.0,
            // Reuse the exact selector considered during ambiguity analysis.
            // Adding a bus alongside model/serial changes `ddcutil` selector
            // precedence and can turn an otherwise disjoint pair into an
            // unsafe bus-vs-identity overlap.
            selector: selector.clone(),
            ..MonitorConfig::default()
        });
    }

    for device in backlight_devices.iter().filter(|device| {
        device.backend_viable && !super::is_ddcci_alias_of_viable_ddc(device, ddc_monitors)
    }) {
        configs.push(MonitorConfig {
            logical_id: allocate_logical_id(backlight_logical_id_base(device), &mut used_ids),
            backend: crate::backends::BackendKind::Backlight,
            enabled: true,
            min_pct: 0,
            max_pct: 100,
            gain: 1.0,
            selector: MonitorSelector {
                sysfs_path: Some(device.sysfs_path.clone()),
                ..MonitorSelector::default()
            },
            ..MonitorConfig::default()
        });
    }

    (configs, ambiguity_notes)
}

fn render_config_snippet(configs: &[MonitorConfig]) -> String {
    if configs.is_empty() {
        return String::from("# No viable brightness-capable devices were discovered.");
    }

    configs
        .iter()
        .map(render_monitor_config)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_monitor_config(monitor: &MonitorConfig) -> String {
    let backend = match monitor.backend {
        crate::backends::BackendKind::Ddc => "ddc",
        crate::backends::BackendKind::Backlight => "backlight",
    };
    let mut lines = vec![
        String::from("[[monitors]]"),
        format!(
            "logical_id = \"{}\"",
            escape_toml_string(&monitor.logical_id)
        ),
        format!("backend = \"{backend}\""),
        format!("enabled = {}", monitor.enabled),
    ];

    if monitor.backend == crate::backends::BackendKind::Ddc {
        lines.push(format!(
            "allow_topology_retargeting = {}",
            monitor.allow_topology_retargeting
        ));
        lines.push(String::from(
            "# Bus-only selectors follow a topology slot; opt in explicitly before enabling.",
        ));
    }

    lines.extend([
        format!("min_pct = {}", monitor.min_pct),
        format!("max_pct = {}", monitor.max_pct),
        format!("gain = {}", monitor.gain),
    ]);

    if let Some(model) = &monitor.selector.model {
        lines.push(format!("model = \"{}\"", escape_toml_string(model)));
    }
    if let Some(serial) = &monitor.selector.serial {
        lines.push(format!("serial = \"{}\"", escape_toml_string(serial)));
    }
    if let Some(connector) = &monitor.selector.connector {
        lines.push(format!("connector = \"{}\"", escape_toml_string(connector)));
    }
    if let Some(path) = &monitor.selector.sysfs_path {
        lines.push(format!("sysfs_path = \"{}\"", escape_toml_string(path)));
    }
    if let Some(bus) = monitor.selector.ddc_bus {
        lines.push(format!("ddc_bus = {bus}"));
    }

    lines.join("\n")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::model::{
        DiscoveryBackends, PhysicalIdentityStatus, WindowsDisplayDiscovery,
    };

    fn status(name: &str, kind: BackendStatusKind, message: &str) -> BackendStatus {
        BackendStatus {
            backend: name.to_owned(),
            status: kind,
            available: kind == BackendStatusKind::Ok,
            message: message.to_owned(),
            guidance: None,
        }
    }

    fn windows_display(number: u32) -> WindowsDisplayDiscovery {
        WindowsDisplayDiscovery {
            display_number: number,
            canonical_id: None,
            display_label: None,
            stable_id: None,
            identity_quality: String::from("strong"),
            mutation_eligibility: String::from("eligible"),
            name: None,
            kind: String::from("external_monitor"),
            active: true,
            target_available: true,
            is_primary: false,
            output_technology: String::from("DisplayPort"),
            gdi_device_name: None,
            adapter_luid: String::from("0x1"),
            source_id: 0,
            target_id: 4,
            connector_instance: 2,
            container_id: None,
            device_instance_id: None,
            hardware_ids: Vec::new(),
            edid_manufacturer: None,
            edid_product_code: None,
            physical_monitors_count: 1,
            physical_monitor_description: None,
            desktop_rect: None,
            note: None,
            brightness_capable: None,
            native_min: None,
            native_current: None,
            native_max: None,
            normalized_current_percent: None,
            capability_note: None,
        }
    }

    fn representative_report(with_devices: bool) -> DiscoveryReport {
        let mut report = DiscoveryReport {
            summary: DiscoverySummary {
                ddc_monitors: 0,
                backlight_devices: 0,
                viable_targets: 0,
            },
            backends: DiscoveryBackends {
                ddcutil: status("ddcutil", BackendStatusKind::Ok, "ddcutil 2.1.4"),
                brightnessctl: status("brightnessctl", BackendStatusKind::Missing, ""),
                sysfs: status("sysfs", BackendStatusKind::Unavailable, "no devices"),
            },
            ddc_observation_complete: true,
            ddc_monitors: Vec::new(),
            backlight_devices: Vec::new(),
            windows_displays: Vec::new(),
            notes: Vec::new(),
            config_snippet: String::from("# No viable brightness-capable devices were discovered."),
        };
        if !with_devices {
            return report;
        }
        report.summary = DiscoverySummary {
            ddc_monitors: 2,
            backlight_devices: 1,
            viable_targets: 2,
        };
        report.ddc_monitors = vec![
            DdcMonitorDiscovery {
                occurrence_id: String::from("a"),
                stable_id: String::from("dell-u2720q-abc"),
                identity_status: PhysicalIdentityStatus::Unique,
                manufacturer: Some(String::from("DEL")),
                model: Some(String::from("DELL U2720Q")),
                serial: Some(String::from("ABC123")),
                display_number: 1,
                bus_number: Some(5),
                connector: Some(String::from("card1-DP-1")),
                brightness_vcp_supported: Some(true),
                backend_viable: true,
                note: None,
            },
            DdcMonitorDiscovery {
                occurrence_id: String::from("b"),
                stable_id: String::from("bus-7"),
                identity_status: PhysicalIdentityStatus::Insufficient,
                manufacturer: None,
                model: None,
                serial: None,
                display_number: 2,
                bus_number: None,
                connector: None,
                brightness_vcp_supported: None,
                backend_viable: false,
                note: Some(String::from("capabilities probe timed out")),
            },
        ];
        report.backlight_devices = vec![BacklightDeviceDiscovery {
            stable_id: String::from("intel-backlight"),
            device_name: String::from("intel_backlight"),
            class: String::from("backlight"),
            max_brightness: Some(96000),
            backlight_type: None,
            probe_source: String::from("sysfs"),
            sysfs_path: String::from("/sys/class/backlight/intel_backlight"),
            ddcci_connector: None,
            backend_viable: true,
            note: None,
        }];
        report.notes = vec![String::from("first note"), String::from("second note")];
        report
    }

    #[test]
    fn render_human_empty_report_is_pinned() {
        let text = render_human(&representative_report(false));
        assert_eq!(text, EXPECTED_EMPTY);
    }

    #[test]
    fn render_human_populated_report_is_pinned() {
        let text = render_human(&representative_report(true));
        assert_eq!(text, EXPECTED_POPULATED);
    }

    #[test]
    fn render_human_windows_topology_is_pinned() {
        let mut report = representative_report(false);
        let mut full = windows_display(1);
        full.name = Some(String::from("Dell U2720Q"));
        full.is_primary = true;
        full.gdi_device_name = Some(String::from("\\\\.\\DISPLAY1"));
        full.desktop_rect = Some(String::from("0,0 2560x1440"));
        full.container_id = Some(String::from("{container}"));
        full.device_instance_id = Some(String::from("DISPLAY\\DEL\\1"));
        full.canonical_id = Some(String::from("canon-1"));
        full.display_label = Some(String::from("Desk left"));
        full.brightness_capable = Some(true);
        full.native_min = Some(0);
        full.native_current = Some(30);
        full.native_max = Some(100);
        full.normalized_current_percent = Some(30);
        full.note = Some(String::from("primary note"));
        let mut unavailable = windows_display(2);
        unavailable.target_available = false;
        unavailable.gdi_device_name = Some(String::from("\\\\.\\DISPLAY2"));
        unavailable.brightness_capable = Some(false);
        unavailable.capability_note = Some(String::from("DXVA2 unsupported"));
        let mut inactive = windows_display(3);
        inactive.active = false;
        inactive.capability_note = Some(String::from("no handle"));
        report.windows_displays = vec![full, unavailable, inactive];
        let text = render_human(&report);
        assert_eq!(text, EXPECTED_WINDOWS);
    }

    const EXPECTED_EMPTY: &str = "Discovery summary\nviable targets: 0\nexternal monitors: 0\nbacklight devices: 0\n\nBackend status\nbackend        status       detail       \n-------------  -----------  -------------\nddcutil        ok           ddcutil 2.1.4\nbrightnessctl  missing                   \nsysfs          unavailable  no devices   \n\nDDC monitors\nNo external monitors reported.\n\nBacklight devices\nNo internal backlight devices reported.\n\nCandidate config snippet\n# No viable brightness-capable devices were discovered.";
    const EXPECTED_POPULATED: &str = "Discovery summary\nviable targets: 2\nexternal monitors: 2\nbacklight devices: 1\n\nBackend status\nbackend        status       detail       \n-------------  -----------  -------------\nddcutil        ok           ddcutil 2.1.4\nbrightnessctl  missing                   \nsysfs          unavailable  no devices   \n\nDDC monitors\ndisplay  bus  manufacturer  model        serial  connector   vcp_0x10  viable  note                        \n-------  ---  ------------  -----------  ------  ----------  --------  ------  ----------------------------\n1        5    DEL           DELL U2720Q  ABC123  card1-DP-1  yes       yes     -                           \n2        -    -             -            -       -           unknown   no      capabilities probe timed out\n\nBacklight devices\ndevice           class      type  max    source  viable  sysfs_path                            note\n---------------  ---------  ----  -----  ------  ------  ------------------------------------  ----\nintel_backlight  backlight  -     96000  sysfs   yes     /sys/class/backlight/intel_backlight  -   \n\nNotes\nfirst note\nsecond note\n\nCandidate config snippet\n# No viable brightness-capable devices were discovered.";
    const EXPECTED_WINDOWS: &str = "Discovery summary\nviable targets: 0\nexternal monitors: 0\nbacklight devices: 0\n\nBackend status\nbackend        status       detail       \n-------------  -----------  -------------\nddcutil        ok           ddcutil 2.1.4\nbrightnessctl  missing                   \nsysfs          unavailable  no devices   \n\nDDC monitors\nNo external monitors reported.\n\nBacklight devices\nNo internal backlight devices reported.\n\nWindows display topology\n  [1] Dell U2720Q (Primary)\n      Kind:              external_monitor (DisplayPort)\n      Status:            Active / Connected\n      GDI Device:        \\\\.\\DISPLAY1 (0,0 2560x1440)\n      CCD Path:          Adapter 0x1 / Target 4 (Connector #2)\n      Physical Monitors: 1 (Handles transient & destroyed via RAII)\n      Container ID:      {container}\n      Device Instance:   DISPLAY\\DEL\\1\n      Identity Quality:  strong\n      W3 Mutation Tier:  eligible\n      Canonical ID:      canon-1\n      Display Label:     Desk left\n      Brightness Backend: Capable (DXVA2 High-Level, Native: 0..100, Current: 30 [30%])\n      Note:              primary note\n\n  [2] Display\n      Kind:              external_monitor (DisplayPort)\n      Status:            Active / Temporarily Unavailable\n      GDI Device:        \\\\.\\DISPLAY2 (coordinates unknown)\n      CCD Path:          Adapter 0x1 / Target 4 (Connector #2)\n      Physical Monitors: 1 (Handles transient & destroyed via RAII)\n      Identity Quality:  strong\n      W3 Mutation Tier:  eligible\n      Brightness Backend: DXVA2 unsupported\n\n  [3] Display\n      Kind:              external_monitor (DisplayPort)\n      Status:            Inactive\n      CCD Path:          Adapter 0x1 / Target 4 (Connector #2)\n      Physical Monitors: 1 (Handles transient & destroyed via RAII)\n      Identity Quality:  strong\n      W3 Mutation Tier:  eligible\n      Brightness Backend: no handle\n\nCandidate config snippet\n# No viable brightness-capable devices were discovered.";
}
