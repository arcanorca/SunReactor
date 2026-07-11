use toml::Value;

use super::{ConfigError, ValidationError};

pub(super) const LEGACY_MINIMUM_BRIGHTNESS_ALIAS_REMOVAL_DATE: &str = "June 1, 2026";
const REMOVED_SOLAR_POLICY_FIELDS_DATE: &str = "March 4, 2026";

pub(super) fn reject_removed_legacy_fields(raw: &str) -> Result<(), ConfigError> {
    let Some(solar_policy) = solar_policy_table(raw) else {
        return Ok(());
    };

    let mut errors = Vec::new();

    if solar_policy.contains_key("sleep_end_local") {
        errors.push(ValidationError::new(
            "solar_policy.sleep_end_local",
            format!(
                "was removed on {REMOVED_SOLAR_POLICY_FIELDS_DATE}; use solar_policy.minimum_brightness_start_local for the evening minimum-brightness time and delete sleep_end_local"
            ),
        ));
    }

    if solar_policy.contains_key("night_min_pct") {
        errors.push(ValidationError::new(
            "solar_policy.night_min_pct",
            format!(
                "was removed on {REMOVED_SOLAR_POLICY_FIELDS_DATE}; use per-monitor min_pct instead"
            ),
        ));
    }

    if solar_policy.contains_key("day_max_pct") {
        errors.push(ValidationError::new(
            "solar_policy.day_max_pct",
            format!(
                "was removed on {REMOVED_SOLAR_POLICY_FIELDS_DATE}; use per-monitor max_pct instead"
            ),
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ConfigError::Validation(errors))
    }
}

pub(super) fn compatibility_warnings(raw: &str) -> Vec<String> {
    let Some(solar_policy) = solar_policy_table(raw) else {
        return Vec::new();
    };

    let mut warnings = Vec::new();
    if solar_policy.contains_key("sleep_start_local") {
        warnings.push(format!(
            "solar_policy.sleep_start_local is deprecated and will be removed after {LEGACY_MINIMUM_BRIGHTNESS_ALIAS_REMOVAL_DATE}; rename it to solar_policy.minimum_brightness_start_local"
        ));
    }

    warnings
}

fn solar_policy_table(raw: &str) -> Option<toml::map::Map<String, Value>> {
    toml::from_str::<Value>(raw)
        .ok()?
        .get("solar_policy")?
        .as_table()
        .cloned()
}
