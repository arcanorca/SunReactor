use crate::discovery::model::RawDdcMonitor;

pub(crate) fn parse_ddc_detect(output: &str) -> Vec<RawDdcMonitor> {
    let mut monitors = Vec::new();
    let mut current: Option<RawDdcMonitor> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(display_number) = parse_display_header(trimmed) {
            if let Some(monitor) = current.take() {
                monitors.push(monitor);
            }
            current = Some(RawDdcMonitor::new(display_number));
            continue;
        }

        let Some(monitor) = current.as_mut() else {
            continue;
        };

        if let Some(value) = trimmed.strip_prefix("I2C bus:") {
            monitor.bus_number = parse_bus_number(value.trim());
        } else if let Some(value) = trimmed.strip_prefix("DRM connector:") {
            monitor.connector = normalize_optional(value.trim());
        } else if let Some(value) = trimmed.strip_prefix("Monitor:") {
            let (manufacturer, model, serial) = parse_monitor_identity(value.trim());
            monitor.manufacturer = manufacturer;
            monitor.model = model;
            monitor.serial = serial;
        }
    }

    if let Some(monitor) = current {
        monitors.push(monitor);
    }

    monitors.sort_by_key(|monitor| monitor.display_number);
    monitors
}

pub(crate) fn parse_brightness_vcp_support(output: &str) -> bool {
    for line in output.lines() {
        let normalized = line.trim().to_ascii_lowercase();
        let Some(rest) = normalized.strip_prefix("feature:") else {
            continue;
        };
        let Some(code) = rest.split_whitespace().next() else {
            continue;
        };
        if code == "10" || code == "0x10" {
            return true;
        }
    }
    false
}

pub(crate) fn parse_getvcp_brightness(output: &str) -> bool {
    for line in output.lines() {
        let normalized = line.trim().to_ascii_lowercase();
        if normalized.contains("vcp opcode 0x10") || normalized.contains("feature 10") {
            if normalized.contains("current value") || normalized.contains("max value") {
                return true;
            }
        }
    }
    false
}

fn parse_display_header(line: &str) -> Option<u32> {
    line.strip_prefix("Display ")?.trim().parse::<u32>().ok()
}

fn parse_bus_number(value: &str) -> Option<u32> {
    value.rsplit('-').next()?.trim().parse::<u32>().ok()
}

fn parse_monitor_identity(value: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut parts = value.splitn(3, ':');
    let manufacturer = parts.next().and_then(normalize_optional);
    let model = parts.next().and_then(normalize_optional);
    let serial = parts.next().and_then(normalize_optional);
    (manufacturer, model, serial)
}

fn normalize_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}
