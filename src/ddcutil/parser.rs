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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessValue {
    pub current: u16,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("Malformed getvcp output: {0}")]
    Malformed(String),
    #[error("Unsupported feature")]
    Unsupported,
}

pub(crate) fn parse_getvcp_brightness(output: &str) -> Result<BrightnessValue, ParseError> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized = trimmed.to_ascii_lowercase();

        // Terse format: "VCP 10 C 50 100"
        if normalized.starts_with("vcp 10 c ") || normalized.starts_with("vcp 0x10 c ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5 {
                if let (Ok(current), Ok(maximum)) =
                    (parts[3].parse::<u16>(), parts[4].parse::<u16>())
                {
                    return Ok(BrightnessValue { current, maximum });
                }
            }
        }

        // Standard format: "VCP code 0x10 (Brightness): current value = 50, max value = 100"
        if normalized.contains("vcp opcode 0x10")
            || normalized.contains("feature 10")
            || normalized.contains("vcp code 0x10")
        {
            if let (Some(cur_idx), Some(max_idx)) = (
                normalized.find("current value ="),
                normalized.find("max value ="),
            ) {
                let cur_str = normalized[cur_idx + 15..]
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim();
                let max_str = normalized[max_idx + 11..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim();
                if let (Ok(current), Ok(maximum)) = (cur_str.parse::<u16>(), max_str.parse::<u16>())
                {
                    return Ok(BrightnessValue { current, maximum });
                }
            }
        }

        // Check for unsupported feature error
        if normalized.contains("unsupported feature") || normalized.contains("invalid vcp") {
            return Err(ParseError::Unsupported);
        }
    }

    Err(ParseError::Malformed(
        "Could not extract brightness values from output".to_string(),
    ))
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
    use super::*;

    #[test]
    fn test_parse_terse_brightness() {
        let output = "VCP 10 C 50 100";
        assert_eq!(
            parse_getvcp_brightness(output),
            Ok(BrightnessValue {
                current: 50,
                maximum: 100
            })
        );
        let output2 = "VCP 0x10 C 25 100";
        assert_eq!(
            parse_getvcp_brightness(output2),
            Ok(BrightnessValue {
                current: 25,
                maximum: 100
            })
        );
    }

    #[test]
    fn test_parse_standard_brightness() {
        let output = "VCP code 0x10 (Brightness                    ): current value =    50, max value =   100";
        assert_eq!(
            parse_getvcp_brightness(output),
            Ok(BrightnessValue {
                current: 50,
                maximum: 100
            })
        );
    }

    #[test]
    fn test_parse_unsupported() {
        let output = "VCP code 0x10 (Brightness                    ): unsupported feature";
        assert_eq!(
            parse_getvcp_brightness(output),
            Err(ParseError::Unsupported)
        );
    }

    #[test]
    fn test_parse_malformed() {
        let output = "Random output";
        assert_eq!(
            parse_getvcp_brightness(output),
            Err(ParseError::Malformed(
                "Could not extract brightness values from output".to_string()
            ))
        );
    }
}
