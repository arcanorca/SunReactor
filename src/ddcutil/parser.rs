use crate::discovery::model::RawDdcMonitor;

#[derive(Debug)]
enum DetectSection {
    None,
    ValidDisplay(RawDdcMonitor),
    InvalidDisplay,
}

pub(crate) fn parse_ddc_detect(output: &str) -> Vec<RawDdcMonitor> {
    let mut monitors = Vec::new();
    let mut section = DetectSection::None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(display_number) = parse_display_header(trimmed) {
            finalize_valid_display(&mut section, &mut monitors);
            section = DetectSection::ValidDisplay(RawDdcMonitor::new(display_number));
            continue;
        }

        if trimmed == "Invalid display" {
            finalize_valid_display(&mut section, &mut monitors);
            section = DetectSection::InvalidDisplay;
            continue;
        }

        let DetectSection::ValidDisplay(monitor) = &mut section else {
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

    finalize_valid_display(&mut section, &mut monitors);

    monitors.sort_by_key(|monitor| monitor.display_number);
    monitors
}

fn finalize_valid_display(section: &mut DetectSection, monitors: &mut Vec<RawDdcMonitor>) {
    if let DetectSection::ValidDisplay(monitor) = std::mem::replace(section, DetectSection::None) {
        monitors.push(monitor);
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrightnessValue {
    pub current: u16,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ParseError {
    #[error("brightness VCP 0x10 is unsupported")]
    Unsupported,
    #[error("could not parse brightness VCP output")]
    Malformed,
}

pub(crate) fn parse_getvcp_brightness(output: &str) -> Result<BrightnessValue, ParseError> {
    for line in output.lines() {
        let trimmed = line.trim();
        let normalized = trimmed.to_ascii_lowercase();

        if normalized.contains("unsupported feature") || normalized.contains("invalid vcp") {
            return Err(ParseError::Unsupported);
        }

        if normalized.starts_with("vcp 10 c ") || normalized.starts_with("vcp 0x10 c ") {
            let fields = trimmed.split_whitespace().collect::<Vec<_>>();
            if fields.len() >= 5 {
                if let (Ok(current), Ok(maximum)) =
                    (fields[3].parse::<u16>(), fields[4].parse::<u16>())
                {
                    return Ok(BrightnessValue { current, maximum });
                }
            }
        }

        if normalized.contains("vcp code 0x10")
            || normalized.contains("vcp opcode 0x10")
            || normalized.contains("feature 10")
        {
            let current = value_after(&normalized, "current value =");
            let maximum = value_after(&normalized, "max value =");
            if let (Some(current), Some(maximum)) = (current, maximum) {
                return Ok(BrightnessValue { current, maximum });
            }
        }
    }

    Err(ParseError::Malformed)
}

fn value_after(line: &str, marker: &str) -> Option<u16> {
    line.split_once(marker)?
        .1
        .trim_start()
        .split(|character: char| !character.is_ascii_digit())
        .next()?
        .parse()
        .ok()
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

#[cfg(test)]
mod tests {
    use super::parse_ddc_detect;

    const MSI_THEN_INVALID_BOE: &str =
        include_str!("../../tests/fixtures/ddcutil/msi_then_invalid_boe.txt");
    const INVALID_THEN_MSI: &str =
        include_str!("../../tests/fixtures/ddcutil/invalid_then_msi.txt");
    const TWO_VALID_THEN_INVALID: &str =
        include_str!("../../tests/fixtures/ddcutil/two_valid_then_invalid.txt");
    const VALID_MISSING_SERIAL: &str =
        include_str!("../../tests/fixtures/ddcutil/valid_missing_serial.txt");
    const INVALID_RESEMBLES_VALID: &str =
        include_str!("../../tests/fixtures/ddcutil/invalid_resembles_valid.txt");
    const TRUNCATED_VALID: &str = include_str!("../../tests/fixtures/ddcutil/truncated_valid.txt");

    #[test]
    fn valid_msi_is_not_contaminated_by_invalid_boe_block() {
        let monitors = parse_ddc_detect(MSI_THEN_INVALID_BOE);

        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].display_number, 1);
        assert_eq!(monitors[0].bus_number, Some(4));
        assert_eq!(monitors[0].connector.as_deref(), Some("card1-DP-1"));
        assert_eq!(monitors[0].manufacturer.as_deref(), Some("MSI"));
        assert_eq!(monitors[0].model.as_deref(), Some("MAG 274QRF QD"));
        assert_eq!(monitors[0].serial.as_deref(), Some("REDACTED-MSI-SERIAL"));
    }

    #[test]
    fn invalid_block_before_valid_display_is_ignored() {
        let monitors = parse_ddc_detect(INVALID_THEN_MSI);

        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].manufacturer.as_deref(), Some("MSI"));
        assert_eq!(monitors[0].bus_number, Some(4));
    }

    #[test]
    fn two_valid_displays_survive_a_trailing_invalid_block() {
        let monitors = parse_ddc_detect(TWO_VALID_THEN_INVALID);

        assert_eq!(monitors.len(), 2);
        assert_eq!(monitors[0].bus_number, Some(4));
        assert_eq!(monitors[1].bus_number, Some(7));
        assert!(monitors
            .iter()
            .all(|monitor| { monitor.manufacturer.as_deref() != Some("BOE") }));
    }

    #[test]
    fn valid_display_with_missing_serial_is_retained() {
        let monitors = parse_ddc_detect(VALID_MISSING_SERIAL);

        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].model.as_deref(), Some("MAG 274QRF QD"));
        assert_eq!(monitors[0].serial, None);
    }

    #[test]
    fn valid_looking_fields_inside_invalid_blocks_never_create_monitors() {
        let monitors = parse_ddc_detect(INVALID_RESEMBLES_VALID);

        assert!(monitors.is_empty());
    }

    #[test]
    fn truncated_final_valid_display_is_retained_with_typed_missing_fields() {
        let monitors = parse_ddc_detect(TRUNCATED_VALID);

        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].display_number, 3);
        assert_eq!(monitors[0].bus_number, Some(11));
        assert_eq!(monitors[0].connector, None);
        assert_eq!(monitors[0].model, None);
    }
}
