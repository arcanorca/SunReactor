use std::collections::HashSet;

use super::{Config, ConfigError, ValidationError};

const MAX_GAIN: f64 = 4.0;
const MAX_GAMMA: f64 = 4.0;
const MIN_TICK_SECONDS: u64 = 5;
const MIN_WEATHER_REFRESH_MINUTES: u32 = 10;

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();

        if self.daemon.tick_seconds < MIN_TICK_SECONDS {
            errors.push(ValidationError::new(
                "daemon.tick_seconds",
                format!("must be at least {MIN_TICK_SECONDS} to avoid busy polling"),
            ));
        }

        if !(-90.0..=90.0).contains(&self.location.latitude) {
            errors.push(ValidationError::new(
                "location.latitude",
                "must be within -90..=90",
            ));
        }

        if !(-180.0..=180.0).contains(&self.location.longitude) {
            errors.push(ValidationError::new(
                "location.longitude",
                "must be within -180..=180",
            ));
        }

        if let Err(e) = validate_timezone(&self.location.timezone) {
            errors.push(ValidationError::new("location.timezone", e));
        }

        validate_pct(
            &mut errors,
            "solar_policy.max_step_pct_per_tick",
            self.solar_policy.max_step_pct_per_tick,
        );
        validate_pct(
            &mut errors,
            "solar_policy.min_write_delta_pct",
            self.solar_policy.min_write_delta_pct,
        );

        if self.solar_policy.twilight_elevation_start >= self.solar_policy.day_elevation_full {
            errors.push(ValidationError::new(
                "solar_policy",
                "twilight_elevation_start must be lower than day_elevation_full",
            ));
        }

        if self.solar_policy.max_step_pct_per_tick == 0 {
            errors.push(ValidationError::new(
                "solar_policy.max_step_pct_per_tick",
                "must be greater than 0",
            ));
        }

        let mut seen_ids = HashSet::new();
        for (index, monitor) in self.monitors.iter().enumerate() {
            let field_prefix = format!("monitors[{index}]");
            let logical_id = monitor.logical_id.trim();
            let mut seen_milestones = HashSet::new();

            if !monitor.transition_gamma.is_finite()
                || monitor.transition_gamma <= 0.0
                || monitor.transition_gamma > MAX_GAMMA
            {
                errors.push(ValidationError::new(
                    format!("{field_prefix}.transition_gamma"),
                    format!("must be finite and within 0 < gamma <= {MAX_GAMMA}"),
                ));
            }

            if logical_id.is_empty() {
                errors.push(ValidationError::new(
                    format!("{field_prefix}.logical_id"),
                    "must not be empty",
                ));
            } else if !seen_ids.insert(logical_id.to_owned()) {
                errors.push(ValidationError::new(
                    format!("{field_prefix}.logical_id"),
                    format!("duplicate logical id `{logical_id}`"),
                ));
            }

            validate_pct(
                &mut errors,
                format!("{field_prefix}.min_pct"),
                monitor.min_pct,
            );
            validate_pct(
                &mut errors,
                format!("{field_prefix}.max_pct"),
                monitor.max_pct,
            );

            if monitor.min_pct > monitor.max_pct {
                errors.push(ValidationError::new(
                    format!("{field_prefix}.min_pct"),
                    "must be less than or equal to max_pct",
                ));
            }

            if !monitor.gain.is_finite() || monitor.gain <= 0.0 || monitor.gain > MAX_GAIN {
                errors.push(ValidationError::new(
                    format!("{field_prefix}.gain"),
                    format!("must be finite and within 0 < gain <= {MAX_GAIN}"),
                ));
            }

            if monitor.enabled && !monitor.selector.has_any() {
                errors.push(ValidationError::new(
                    format!("{field_prefix}.selector"),
                    "enabled monitors must define at least one selector field",
                ));
            }

            if let Some(address) = monitor.selector.ddc_address {
                if address > 127 {
                    errors.push(ValidationError::new(
                        format!("{field_prefix}.ddc_address"),
                        "must fit within the 7-bit I2C address range",
                    ));
                }
            }

            for (adjustment_index, adjustment) in monitor.milestone_adjustments.iter().enumerate() {
                if !seen_milestones.insert(adjustment.milestone) {
                    errors.push(ValidationError::new(
                        format!(
                            "{field_prefix}.milestone_adjustments[{adjustment_index}].milestone"
                        ),
                        format!("duplicate milestone `{}`", adjustment.milestone.as_str()),
                    ));
                }

                if !(-720..=720).contains(&adjustment.minutes_offset) {
                    errors.push(ValidationError::new(
                        format!(
                            "{field_prefix}.milestone_adjustments[{adjustment_index}].minutes_offset"
                        ),
                        "must stay within -720..=720 minutes",
                    ));
                }
            }
        }

        if self.weather.refresh_minutes < MIN_WEATHER_REFRESH_MINUTES {
            errors.push(ValidationError::new(
                "weather.refresh_minutes",
                format!(
                    "must be at least {MIN_WEATHER_REFRESH_MINUTES} minutes to keep refresh bounded"
                ),
            ));
        }

        if !self.weather.min_multiplier.is_finite()
            || self.weather.min_multiplier <= 0.0
            || self.weather.min_multiplier > 1.0
        {
            errors.push(ValidationError::new(
                "weather.min_multiplier",
                "must be finite and within 0 < min_multiplier <= 1",
            ));
        }

        if self.weather.enabled && self.weather.provider.is_none() {
            errors.push(ValidationError::new(
                "weather.provider",
                "must be set when weather is enabled",
            ));
        }

        if let Some(api_key_env) = self.weather.api_key_env.as_deref() {
            if api_key_env.trim().is_empty() {
                errors.push(ValidationError::new(
                    "weather.api_key_env",
                    "must not be empty when set",
                ));
            }
        }

        if let Some(api_key) = self.weather.api_key.as_deref() {
            if api_key.trim().is_empty() {
                errors.push(ValidationError::new(
                    "weather.api_key",
                    "must not be empty when set",
                ));
            }
        }

        if self.weather.enabled
            && self.weather.api_key_env.is_none()
            && self.weather.api_key.is_none()
        {
            errors.push(ValidationError::new(
                "weather",
                "enabled weather requires api_key_env or api_key",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Validation(errors))
        }
    }
}

fn validate_pct(errors: &mut Vec<ValidationError>, field: impl Into<String>, value: u8) {
    if value > 100 {
        errors.push(ValidationError::new(field, "must be within 0..=100"));
    }
}

pub fn validate_timezone(timezone: &str) -> Result<(), String> {
    let timezone = timezone.trim();
    let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone);
    if path.exists() {
        return Ok(());
    }

    if tz::TimeZone::from_posix_tz(timezone).is_ok() {
        return Ok(());
    }

    Err(format!(
        "invalid timezone `{timezone}` (must be a valid IANA zone or POSIX string)"
    ))
}
