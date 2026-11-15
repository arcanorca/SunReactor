use std::collections::HashSet;

use super::{
    Config, ConfigError, DaemonConfig, LocationConfig, MonitorConfig, MonitorSelector,
    SolarPolicyConfig, ValidationError, WeatherConfig,
};

const MAX_GAIN: f64 = 4.0;
/// Largest valid exponent for a monitor automation curve.
///
/// Interactive clients use this same limit so they never offer values that
/// persisted configuration validation would reject.
pub const MAX_TRANSITION_GAMMA: f64 = 4.0;
const MIN_TICK_SECONDS: u64 = 5;
/// Below this the daemon would keep the I2C bus needlessly busy.
const MIN_PROBE_SECONDS: u64 = 2;
const MAX_PROBE_SECONDS: u64 = 3600;
const MIN_WEATHER_REFRESH_MINUTES: u32 = 10;
const MAX_OPERATIONAL_MINUTES: u64 = 24 * 60;
const MIN_BACKEND_TIMEOUT_SECONDS: u64 = 1;
const MAX_BACKEND_TIMEOUT_SECONDS: u64 = 60;

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();
        validate_daemon(&self.daemon, &mut errors);
        validate_location(&self.location, &mut errors);
        validate_solar_policy(&self.solar_policy, &mut errors);
        validate_monitors(&self.monitors, &mut errors);
        validate_weather(&self.weather, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Validation(errors))
        }
    }
}

impl DaemonConfig {
    /// Clamps operational values that predate their bounds so configs written
    /// by older releases keep loading (strict validation would otherwise stop
    /// the daemon). Returns one warning per clamped field. Structured saves
    /// still validate without clamping.
    pub(super) fn clamp_legacy_operational_values(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        clamp_legacy(
            &mut warnings,
            "daemon.desktop_idle_timeout_minutes",
            &mut self.desktop_idle_timeout_minutes,
            0,
            MAX_OPERATIONAL_MINUTES,
            "minutes",
        );
        clamp_legacy(
            &mut warnings,
            "daemon.apply_reassert_minutes",
            &mut self.apply_reassert_minutes,
            0,
            MAX_OPERATIONAL_MINUTES,
            "minutes",
        );
        clamp_legacy(
            &mut warnings,
            "daemon.ddc_timeout_seconds",
            &mut self.ddc_timeout_seconds,
            MIN_BACKEND_TIMEOUT_SECONDS,
            MAX_BACKEND_TIMEOUT_SECONDS,
            "seconds",
        );
        clamp_legacy(
            &mut warnings,
            "daemon.backlight_timeout_seconds",
            &mut self.backlight_timeout_seconds,
            MIN_BACKEND_TIMEOUT_SECONDS,
            MAX_BACKEND_TIMEOUT_SECONDS,
            "seconds",
        );
        warnings
    }
}

fn clamp_legacy(
    warnings: &mut Vec<String>,
    field: &str,
    value: &mut u64,
    min: u64,
    max: u64,
    unit: &str,
) {
    let clamped = (*value).clamp(min, max);
    if clamped != *value {
        let warning = format!(
            "{field} = {} is outside {min}..={max} {unit}; clamped to {clamped} (update the config file to silence this warning)",
            *value
        );
        tracing::warn!("{warning}");
        warnings.push(warning);
        *value = clamped;
    }
}

// Each section validator appends to `errors` in a fixed order; the overall
// order (daemon, location, solar policy, monitors, weather) is part of the
// user-visible error report.

fn validate_daemon(daemon: &DaemonConfig, errors: &mut Vec<ValidationError>) {
    if daemon.tick_seconds < MIN_TICK_SECONDS {
        errors.push(ValidationError::new(
            "daemon.tick_seconds",
            format!("must be at least {MIN_TICK_SECONDS} to avoid busy polling"),
        ));
    }

    if daemon.probe_seconds != 0
        && !(MIN_PROBE_SECONDS..=MAX_PROBE_SECONDS).contains(&daemon.probe_seconds)
    {
        errors.push(ValidationError::new(
            "daemon.probe_seconds",
            format!("must be 0 (disabled) or within {MIN_PROBE_SECONDS}..={MAX_PROBE_SECONDS}"),
        ));
    }

    if daemon.desktop_idle_timeout_minutes > MAX_OPERATIONAL_MINUTES {
        errors.push(ValidationError::new(
            "daemon.desktop_idle_timeout_minutes",
            format!("must be 0 (disabled) or at most {MAX_OPERATIONAL_MINUTES} minutes"),
        ));
    }

    if daemon.apply_reassert_minutes > MAX_OPERATIONAL_MINUTES {
        errors.push(ValidationError::new(
            "daemon.apply_reassert_minutes",
            format!("must be at most {MAX_OPERATIONAL_MINUTES} minutes"),
        ));
    }

    if daemon.external_change_hold_minutes > 1_440 {
        errors.push(ValidationError::new(
            "daemon.external_change_hold_minutes",
            "must be within 0..=1440 minutes",
        ));
    }

    if !(MIN_BACKEND_TIMEOUT_SECONDS..=MAX_BACKEND_TIMEOUT_SECONDS)
        .contains(&daemon.ddc_timeout_seconds)
    {
        errors.push(ValidationError::new(
            "daemon.ddc_timeout_seconds",
            format!(
                "must be within {MIN_BACKEND_TIMEOUT_SECONDS}..={MAX_BACKEND_TIMEOUT_SECONDS} seconds"
            ),
        ));
    }

    if !(MIN_BACKEND_TIMEOUT_SECONDS..=MAX_BACKEND_TIMEOUT_SECONDS)
        .contains(&daemon.backlight_timeout_seconds)
    {
        errors.push(ValidationError::new(
            "daemon.backlight_timeout_seconds",
            format!(
                "must be within {MIN_BACKEND_TIMEOUT_SECONDS}..={MAX_BACKEND_TIMEOUT_SECONDS} seconds"
            ),
        ));
    }
}

fn validate_location(location: &LocationConfig, errors: &mut Vec<ValidationError>) {
    if !(-90.0..=90.0).contains(&location.latitude) {
        errors.push(ValidationError::new(
            "location.latitude",
            "must be within -90..=90",
        ));
    }

    if !(-180.0..=180.0).contains(&location.longitude) {
        errors.push(ValidationError::new(
            "location.longitude",
            "must be within -180..=180",
        ));
    }

    if let Err(e) = validate_timezone(&location.timezone) {
        errors.push(ValidationError::new("location.timezone", e));
    }
}

fn validate_solar_policy(solar: &SolarPolicyConfig, errors: &mut Vec<ValidationError>) {
    validate_pct(
        errors,
        "solar_policy.max_step_pct_per_tick",
        solar.max_step_pct_per_tick,
    );
    validate_pct(
        errors,
        "solar_policy.min_write_delta_pct",
        solar.min_write_delta_pct,
    );

    if solar.twilight_elevation_start >= solar.day_elevation_full {
        errors.push(ValidationError::new(
            "solar_policy",
            "twilight_elevation_start must be lower than day_elevation_full",
        ));
    }

    if solar.max_step_pct_per_tick == 0 {
        errors.push(ValidationError::new(
            "solar_policy.max_step_pct_per_tick",
            "must be greater than 0",
        ));
    }
}

/// Validates each monitor in order, then the pairwise disjointness of all
/// enabled DDC selectors.
fn validate_monitors(monitors: &[MonitorConfig], errors: &mut Vec<ValidationError>) {
    let mut seen_ids = HashSet::new();
    let mut enabled_ddc = Vec::new();
    for (index, monitor) in monitors.iter().enumerate() {
        validate_monitor(index, monitor, &mut seen_ids, &mut enabled_ddc, errors);
    }
    validate_ddc_disjointness(&enabled_ddc, errors);
}

/// An enabled DDC monitor: its index, selector and effective ddcutil args.
type EnabledDdc = (usize, MonitorSelector, Vec<String>);

fn validate_monitor(
    index: usize,
    monitor: &MonitorConfig,
    seen_ids: &mut HashSet<String>,
    enabled_ddc: &mut Vec<EnabledDdc>,
    errors: &mut Vec<ValidationError>,
) {
    let field_prefix = format!("monitors[{index}]");
    let logical_id = monitor.logical_id.trim();

    if !monitor.transition_gamma.is_finite()
        || monitor.transition_gamma <= 0.0
        || monitor.transition_gamma > MAX_TRANSITION_GAMMA
    {
        errors.push(ValidationError::new(
            format!("{field_prefix}.transition_gamma"),
            format!("must be finite and within 0 < gamma <= {MAX_TRANSITION_GAMMA}"),
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

    validate_pct(errors, format!("{field_prefix}.min_pct"), monitor.min_pct);
    validate_pct(errors, format!("{field_prefix}.max_pct"), monitor.max_pct);

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

    if monitor.enabled && monitor.backend == crate::backends::BackendKind::Ddc {
        match crate::backends::ddc::effective_selector_args(&monitor.selector) {
            Ok(args) => enabled_ddc.push((index, monitor.selector.clone(), args)),
            Err(error) => errors.push(ValidationError::new(
                format!("{field_prefix}.selector"),
                error.to_string(),
            )),
        }
    }

    validate_milestone_adjustments(&field_prefix, monitor, errors);
}

fn validate_milestone_adjustments(
    field_prefix: &str,
    monitor: &MonitorConfig,
    errors: &mut Vec<ValidationError>,
) {
    let mut seen_milestones = HashSet::new();
    for (adjustment_index, adjustment) in monitor.milestone_adjustments.iter().enumerate() {
        if !seen_milestones.insert(adjustment.milestone) {
            errors.push(ValidationError::new(
                format!("{field_prefix}.milestone_adjustments[{adjustment_index}].milestone"),
                format!("duplicate milestone `{}`", adjustment.milestone.as_str()),
            ));
        }

        if !(-720..=720).contains(&adjustment.minutes_offset) {
            errors.push(ValidationError::new(
                format!("{field_prefix}.milestone_adjustments[{adjustment_index}].minutes_offset"),
                "must stay within -720..=720 minutes",
            ));
        }
    }
}

fn validate_ddc_disjointness(enabled_ddc: &[EnabledDdc], errors: &mut Vec<ValidationError>) {
    for (position, (left_index, left, left_args)) in enabled_ddc.iter().enumerate() {
        for (right_index, right, right_args) in enabled_ddc.iter().skip(position + 1) {
            match crate::backends::ddc::selector_relation(left, right) {
                Ok(crate::backends::ddc::DdcSelectorRelation::ProvablyDisjoint) => {}
                Ok(relation) => errors.push(ValidationError::new(
                    format!("monitors[{right_index}].selector"),
                    format!("monitors[{left_index}] and monitors[{right_index}] use DDC selectors that may address the same display ({relation:?}): `{left_args:?}` overlaps `{right_args:?}`; use pairwise disjoint selectors"),
                )),
                Err(error) => errors.push(ValidationError::new(
                    format!("monitors[{right_index}].selector"),
                    error.to_string(),
                )),
            }
        }
    }
}

fn validate_weather(weather: &WeatherConfig, errors: &mut Vec<ValidationError>) {
    if weather.refresh_minutes < MIN_WEATHER_REFRESH_MINUTES {
        errors.push(ValidationError::new(
            "weather.refresh_minutes",
            format!(
                "must be at least {MIN_WEATHER_REFRESH_MINUTES} minutes to keep refresh bounded"
            ),
        ));
    }

    if !weather.min_multiplier.is_finite()
        || weather.min_multiplier <= 0.0
        || weather.min_multiplier > 1.0
    {
        errors.push(ValidationError::new(
            "weather.min_multiplier",
            "must be finite and within 0 < min_multiplier <= 1",
        ));
    }

    if weather.enabled && weather.provider.is_none() {
        errors.push(ValidationError::new(
            "weather.provider",
            "must be set when weather is enabled",
        ));
    }

    if let Some(api_key_env) = weather.api_key_env.as_deref() {
        if api_key_env.trim().is_empty() {
            errors.push(ValidationError::new(
                "weather.api_key_env",
                "must not be empty when set",
            ));
        }
    }

    if let Some(api_key) = weather.api_key.as_deref() {
        if api_key.trim().is_empty() {
            errors.push(ValidationError::new(
                "weather.api_key",
                "must not be empty when set",
            ));
        }
    }

    if weather.enabled && weather.api_key_env.is_none() && weather.api_key.is_none() {
        errors.push(ValidationError::new(
            "weather",
            "enabled weather requires api_key_env or api_key",
        ));
    }
}

fn validate_pct(errors: &mut Vec<ValidationError>, field: impl Into<String>, value: u8) {
    if value > 100 {
        errors.push(ValidationError::new(field, "must be within 0..=100"));
    }
}

pub fn validate_timezone(timezone: &str) -> Result<(), String> {
    let timezone = timezone.trim();
    if crate::timezone::resolve_timezone(timezone).is_some() {
        return Ok(());
    }

    Err(format!(
        "invalid timezone `{timezone}` (must be a valid IANA zone or POSIX string)"
    ))
}
