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
    #[error("brightness VCP 0x10 reported an impossible range (current {current}, max {maximum})")]
    OutOfRange { current: u16, maximum: u16 },
}

/// Parses a `getvcp 10` reply. Every form that is not an unambiguous
/// `current <= max, max > 0` reading is an error, never a guessed value.
pub(crate) fn parse_getvcp_brightness(output: &str) -> Result<BrightnessValue, ParseError> {
    for line in output.lines() {
        let trimmed = line.trim();
        let normalized = trimmed.to_ascii_lowercase();

        if normalized.contains("unsupported feature") || normalized.contains("invalid vcp") {
            return Err(ParseError::Unsupported);
        }

        let fields = normalized.split_whitespace().collect::<Vec<_>>();
        if matches!(fields.first(), Some(&"vcp")) && matches!(fields.get(1), Some(&("10" | "0x10")))
        {
            // Terse form: `VCP 10 C <current> <max>`. `ERR`, `CNC` and other
            // result codes carry no reading and fall through to an error.
            if fields.get(2) == Some(&"c") && fields.len() == 5 {
                if let (Ok(current), Ok(maximum)) =
                    (fields[3].parse::<u16>(), fields[4].parse::<u16>())
                {
                    return validated(current, maximum);
                }
            }
            if matches!(fields.get(2), Some(&("err" | "cnc"))) {
                return Err(ParseError::Unsupported);
            }
        }

        if normalized.contains("vcp code 0x10") || normalized.contains("vcp opcode 0x10") {
            let current = value_after(&normalized, "current value =");
            let maximum = value_after(&normalized, "max value =");
            if let (Some(current), Some(maximum)) = (current, maximum) {
                return validated(current, maximum);
            }
        }
    }

    Err(ParseError::Malformed)
}

fn validated(current: u16, maximum: u16) -> Result<BrightnessValue, ParseError> {
    if maximum == 0 || current > maximum {
        return Err(ParseError::OutOfRange { current, maximum });
    }
    Ok(BrightnessValue { current, maximum })
}

/// The unsigned decimal after `marker`, which must be followed by a comma,
/// whitespace or the end of the line (so `35abc` and `-5` are rejected).
fn value_after(line: &str, marker: &str) -> Option<u16> {
    let rest = line.split_once(marker)?.1.trim_start();
    let end = rest
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(rest.len());
    let (digits, tail) = rest.split_at(end);
    if tail
        .chars()
        .next()
        .is_some_and(|next| next != ',' && !next.is_whitespace())
    {
        return None;
    }
    digits.parse().ok()
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

    use super::{parse_getvcp_brightness as getvcp, ParseError};

    #[test]
    fn getvcp_terse_and_verbose_valid_forms() {
        assert_eq!(getvcp("VCP 10 C 35 100\n").unwrap().current, 35);
        assert_eq!(getvcp("VCP 10 C 128 255").unwrap().maximum, 255);
        let verbose =
            "VCP code 0x10 (Brightness                    ): current value =    35, max value =   100\n";
        let value = getvcp(verbose).unwrap();
        assert_eq!((value.current, value.maximum), (35, 100));
        // Warning lines around the reading are ignored.
        let noisy = "Warning: something odd on feature 10\nVCP 10 C 7 50\nTrailing note\n";
        assert_eq!(getvcp(noisy).unwrap().maximum, 50);
    }

    #[test]
    fn getvcp_rejects_every_hostile_form() {
        assert_eq!(getvcp("VCP 10 ERR\n"), Err(ParseError::Unsupported));
        assert_eq!(getvcp("VCP 10 CNC\n"), Err(ParseError::Unsupported));
        assert_eq!(
            getvcp("VCP code 0x10 (Brightness): Unsupported feature code\n"),
            Err(ParseError::Unsupported)
        );
        assert_eq!(getvcp(""), Err(ParseError::Malformed));
        assert_eq!(getvcp("\n  \n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("Display not found\n"), Err(ParseError::Malformed));
        assert!(getvcp("Warning: slow\nVCP 10 ERR extra\n").is_err());
        // Other features are never mistaken for brightness.
        assert_eq!(getvcp("VCP 12 C 50 100\n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("VCP 100 C 50 100\n"), Err(ParseError::Malformed));
        // Negative, huge, hex, trailing-garbage and truncated numbers.
        assert_eq!(getvcp("VCP 10 C -5 100\n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("VCP 10 C 35 99999999\n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("VCP 10 C 70000 100\n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("VCP 10 C 35abc 100\n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("VCP 10 C 35\n"), Err(ParseError::Malformed));
        assert_eq!(getvcp("VCP 10 C 35 100 7\n"), Err(ParseError::Malformed));
        let negative = "VCP code 0x10 (Brightness): current value =   -5, max value =   100\n";
        assert_eq!(getvcp(negative), Err(ParseError::Malformed));
        let garbage = "VCP code 0x10 (Brightness): current value = 35abc, max value = 100\n";
        assert_eq!(getvcp(garbage), Err(ParseError::Malformed));
        let huge = "VCP code 0x10 (Brightness): current value = 35, max value = 1000000\n";
        assert_eq!(getvcp(huge), Err(ParseError::Malformed));
        let truncated = "VCP code 0x10 (Brightness): current value = 35,\n";
        assert_eq!(getvcp(truncated), Err(ParseError::Malformed));
    }

    #[test]
    fn getvcp_rejects_impossible_ranges() {
        assert!(matches!(
            getvcp("VCP 10 C 5 0\n"),
            Err(ParseError::OutOfRange { .. })
        ));
        assert!(matches!(
            getvcp("VCP 10 C 101 100\n"),
            Err(ParseError::OutOfRange { .. })
        ));
        let verbose = "VCP code 0x10 (Brightness): current value = 0, max value = 0\n";
        assert!(matches!(
            getvcp(verbose),
            Err(ParseError::OutOfRange { .. })
        ));
    }
}
