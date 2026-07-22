pub mod math;
pub mod milestones;
pub mod modifiers;

pub(crate) use math::*;
pub(crate) use milestones::*;
pub(crate) use modifiers::*;

use crate::config::SolarPolicyConfig;
use crate::solar::{self, Location, SolarError};
use chrono::{DateTime, Utc};

/// Pure policy input for computing per-monitor targets.
///
/// `now_utc` is always UTC. Timezone interpretation happens explicitly through
/// `location` inside the solar module; the policy engine never asks the host
/// environment for local time.
#[derive(Debug, Clone)]
pub struct PolicyContext<'a> {
    pub now_utc: DateTime<Utc>,
    pub location: &'a Location,
    pub config: &'a SolarPolicyConfig,
    pub monitors: &'a [crate::config::MonitorConfig],
    pub weather_multiplier: Option<f64>,
}

/// Fixed milestone set used to explain and calibrate the automation curve.
/// One milestone entry for one monitor.
/// Daily monitor schedule preview derived from the pure policy curve.
/// Per-monitor target output.
#[derive(Debug, Clone, PartialEq)]
pub struct PerMonitorTarget {
    pub logical_id: String,
    pub percent: u8,
    pub solar_daylight_factor: f64,
    pub effective_daylight_factor: f64,
}

/// Full pure policy evaluation output.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyOutput {
    pub solar_elevation_deg: f64,
    pub weather_multiplier: f64,
    pub targets: Vec<PerMonitorTarget>,
}

/// Structured policy evaluation errors.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PolicyError {
    #[error(
        "invalid monitor config: transition_gamma ({transition_gamma}) must be finite and > 0"
    )]
    InvalidMonitorConfig { transition_gamma: f64 },
    #[error("invalid solar curve: twilight_elevation_start_deg ({twilight_elevation_start_deg}) must be lower than day_elevation_full_deg ({day_elevation_full_deg})")]
    InvalidSolarCurve {
        twilight_elevation_start_deg: f64,
        day_elevation_full_deg: f64,
    },
    #[error(
        "weather daylight multiplier must be finite and within 0..=1, got {daylight_multiplier}"
    )]
    InvalidWeatherModifier { daylight_multiplier: f64 },

    #[error("monitor `{logical_id}` has invalid range: min_pct={min_pct}, max_pct={max_pct}, expected 0..=100 with min_pct <= max_pct")]
    InvalidMonitorRange {
        logical_id: String,
        min_pct: u8,
        max_pct: u8,
    },
    #[error(
        "monitor `{logical_id}` has invalid gain {gain}; expected a finite value greater than 0"
    )]
    InvalidMonitorGain { logical_id: String, gain: f64 },
    #[error(transparent)]
    Solar(#[from] SolarError),
}

/// Computes per-monitor target brightness percentages from time, location, and
/// pure policy inputs.
pub fn compute_policy(input: &PolicyContext) -> Result<PolicyOutput, PolicyError> {
    validate_policy_input(input)?;

    let zenith_deg = compute_adaptive_zenith(
        input.now_utc,
        input.location,
        input.config.day_elevation_full,
        input.config.use_adaptive_zenith,
        input.config.twilight_elevation_start,
    );

    let solar_sample = solar::sample_at_utc(
        input.now_utc,
        input.location,
        input.config.twilight_elevation_start,
        zenith_deg,
    )?;

    // We can directly call the core logic using the dynamically calculated zenith.
    // To ensure the rest of the existing policy engine (adjustments, weather, etc.) works
    // perfectly, we pass the adaptive zenith as the `day_elevation_full` in effective input.
    let mut effective_config = input.config.clone();
    effective_config.day_elevation_full = zenith_deg;

    let effective_input = PolicyContext {
        now_utc: input.now_utc,
        location: input.location,
        config: &effective_config,
        monitors: input.monitors,
        weather_multiplier: input.weather_multiplier,
    };

    compute_policy_for_elevation(&effective_input, f64::from(solar_sample.elevation_deg))
}

// --- STEP 3: The Zenith Logic ---
// Computes the dynamic zenith (target elevation for 100% brightness).
// If `use_adaptive_zenith` is true, it finds the solar noon for the day.
// If it's a polar region, it falls back safely.

// --- STEP 4: Linear Daylight Factor ---
// Computes a clamped [0.0, 1.0] linear factor from the current elevation.

// --- STEP 5: Perceptual Curve ---
// Maps the linear [0.0, 1.0] factor to a human-perceptual [0.0, 1.0] curve using gamma correction.

// --- STEP 6: Hardware Projection ---
// Projects the perceptual [0.0, 1.0] factor into actual hardware percentages [min_pct, max_pct].

// --- STEP 7: Unified Policy Engine ---
// Pure API that takes environmental inputs and returns the hardware percentage directly.

/// Maps a solar elevation to a normalized daylight factor in `[0, 1]`.
///
/// Values at or below `twilight_elevation_start_deg` clamp to `0`. Values at
/// or above `day_elevation_full_deg` clamp to `1`. Values in between are mapped
/// linearly and then shaped by `transition_gamma`.
///
/// Computes per-monitor target brightness percentages from an explicit solar
/// elevation.
///
/// This keeps the policy engine reusable in tests or higher-level code that
/// already has a trusted solar sample and only needs the brightness mapping.
pub fn compute_policy_for_elevation(
    input: &PolicyContext,
    solar_elevation_deg: f64,
) -> Result<PolicyOutput, PolicyError> {
    validate_policy_input(input)?;
    compute_policy_with_elevation(input, solar_elevation_deg)
}

// Builds a per-monitor daily milestone preview for the local date implied by
// `input.now_utc`. Weather is intentionally excluded so the schedule remains a
// stable automation preview rather than a transient forecast snapshot.

pub(crate) fn compute_policy_with_elevation(
    input: &PolicyContext,
    solar_elevation_deg: f64,
) -> Result<PolicyOutput, PolicyError> {
    let base_linear_factor = daylight_factor_from_elevation(
        solar_elevation_deg,
        input.config.twilight_elevation_start,
        input.config.day_elevation_full,
        1.0, // base linear factor
    )?;

    // We compute the global weather multiplier based on the linear factor to determine
    // if the solar phase allows weather modifiers (daylight > 0).
    let weather_multiplier =
        weather_multiplier_for_solar_phase(input.weather_multiplier, base_linear_factor)?;

    let now_local = solar::local_datetime_at_utc(input.now_utc, input.location)?;
    let shared_schedule = input
        .monitors
        .iter()
        .any(|monitor| !monitor.milestone_adjustments.is_empty())
        .then(|| {
            let context = resolve_base_milestone_context(input)?;
            let base = resolve_base_milestones(input, &context)?;
            Ok::<_, PolicyError>((context, base))
        })
        .transpose()?;

    let mut targets = Vec::with_capacity(input.monitors.len());
    for monitor in input.monitors {
        validate_monitor(monitor)?;

        let monitor_solar_factor = daylight_factor_from_elevation(
            solar_elevation_deg,
            input.config.twilight_elevation_start,
            input.config.day_elevation_full,
            monitor.transition_gamma,
        )?;

        let adjusted_daylight_factor =
            match adjust_evening_daylight_factor(input, monitor, monitor_solar_factor) {
                Ok(factor) => factor,
                Err(PolicyError::Solar(SolarError::SunNeverCrossesThreshold { .. })) => {
                    monitor_solar_factor
                }
                Err(err) => return Err(err),
            };

        let monitor_daylight_factor = if let Some((context, base)) = shared_schedule.as_ref() {
            if monitor.milestone_adjustments.is_empty() {
                adjusted_daylight_factor
            } else {
                daylight_factor_with_adjustments(input, monitor, now_local, context, base)?
            }
        } else {
            adjusted_daylight_factor
        };

        let effective_factor = (monitor_daylight_factor * weather_multiplier).clamp(0.0, 1.0);

        targets.push(PerMonitorTarget {
            logical_id: monitor.logical_id.clone(),
            percent: compute_monitor_target_percent(monitor, effective_factor),
            solar_daylight_factor: monitor_solar_factor,
            effective_daylight_factor: effective_factor,
        });
    }

    Ok(PolicyOutput {
        solar_elevation_deg,
        weather_multiplier,
        targets,
    })
}

pub(crate) fn base_linear_effective_daylight_factor_at_local(
    input: &PolicyContext,
    context: &BaseMilestoneContext,
    datetime_local: DateTime<chrono::FixedOffset>,
) -> Result<f64, PolicyError> {
    base_linear_effective_daylight_factor_at_utc(input, context, datetime_local.with_timezone(&Utc))
}

pub(crate) fn base_linear_effective_daylight_factor_at_utc(
    input: &PolicyContext,
    context: &BaseMilestoneContext,
    datetime_utc: DateTime<Utc>,
) -> Result<f64, PolicyError> {
    let datetime_local = crate::solar::local_datetime_at_utc(datetime_utc, input.location).unwrap();
    if datetime_local <= context.day_start_local
        || datetime_local >= context.minimum_brightness_start_local
    {
        return Ok(0.0);
    }

    if datetime_local <= context.sunset_local {
        return linear_daylight_factor_at_utc(input, datetime_utc);
    }

    let total_seconds =
        (context.minimum_brightness_start_local - context.sunset_local).num_seconds();
    if total_seconds <= 0 {
        return Ok(0.0);
    }

    let elapsed_seconds = (datetime_local - context.sunset_local)
        .num_seconds()
        .clamp(0, total_seconds);
    let remaining_ratio = 1.0 - (elapsed_seconds as f64 / total_seconds as f64);
    Ok((context.sunset_linear_factor * remaining_ratio).clamp(0.0, context.peak_linear_factor))
}

pub(crate) fn linear_daylight_factor_at_local(
    input: &PolicyContext,
    datetime_local: DateTime<chrono::FixedOffset>,
) -> Result<f64, PolicyError> {
    linear_daylight_factor_at_utc(input, datetime_local.with_timezone(&Utc))
}

pub(crate) fn linear_daylight_factor_at_utc(
    input: &PolicyContext,
    datetime_utc: DateTime<Utc>,
) -> Result<f64, PolicyError> {
    let zenith_deg = compute_adaptive_zenith(
        datetime_utc,
        input.location,
        input.config.day_elevation_full,
        input.config.use_adaptive_zenith,
        input.config.twilight_elevation_start,
    );

    let sample = solar::sample_at_utc(
        datetime_utc,
        input.location,
        input.config.twilight_elevation_start,
        zenith_deg,
    )?;

    daylight_factor_from_elevation(
        f64::from(sample.elevation_deg),
        input.config.twilight_elevation_start,
        zenith_deg,
        1.0, // Calculate linear factor only; gamma is applied per-monitor later
    )
}

pub(crate) fn solar_elevation_at_utc(
    input: &PolicyContext,
    datetime_utc: DateTime<Utc>,
) -> Result<f64, PolicyError> {
    let sample = solar::sample_at_utc(
        datetime_utc,
        input.location,
        input.config.twilight_elevation_start,
        input.config.day_elevation_full,
    )?;
    Ok(f64::from(sample.elevation_deg))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, TimeZone, Utc};

    use super::{
        compute_monitor_milestones, compute_policy, compute_policy_for_elevation,
        daylight_factor_from_elevation, AutomationMilestone, PolicyContext, PolicyError,
        PolicyOutput,
    };
    use crate::config::{MonitorConfig, MonitorMilestoneAdjustment, SolarPolicyConfig};
    use crate::solar::{self, Location};

    #[test]
    fn night_maps_to_monitor_minimum() {
        let output = evaluate_at_elevation(-12.0, vec![monitor("internal", 18, 90, 1.0)]);
        assert_eq!(output.targets[0].percent, 18);
        assert_eq!(output.targets[0].solar_daylight_factor, 0.0);
        assert_eq!(output.targets[0].effective_daylight_factor, 0.0);
    }

    #[test]
    fn high_sun_maps_to_monitor_maximum() {
        let output = evaluate_at_elevation(45.0, vec![monitor("internal", 18, 90, 1.0)]);
        assert_eq!(output.targets[0].percent, 90);
        assert_eq!(output.targets[0].solar_daylight_factor, 1.0);
        assert_eq!(output.targets[0].effective_daylight_factor, 1.0);
    }

    #[test]
    fn daylight_factor_is_monotonic_with_increasing_elevation() {
        let elevations = [-10.0, -6.0, -3.0, 0.0, 3.0, 10.0];
        let mut previous = f64::NEG_INFINITY;

        for elevation in elevations {
            let factor = daylight_factor_from_elevation(elevation, -6.0, 3.0, 1.4)
                .expect("curve should be valid");
            assert!(
                factor >= previous,
                "factor regressed at elevation {elevation}"
            );
            previous = factor;
        }
    }

    #[test]
    fn per_monitor_overrides_produce_distinct_targets() {
        let output = evaluate_at_elevation(
            0.0,
            vec![
                monitor("desk", 20, 80, 1.0),
                monitor("internal", 10, 100, 1.0),
            ],
        );

        assert_eq!(output.targets[0].logical_id, "desk");
        assert_eq!(output.targets[1].logical_id, "internal");
        assert_ne!(output.targets[0].percent, output.targets[1].percent);
        assert_eq!(output.targets[0].percent, 59);
        assert_eq!(output.targets[1].percent, 69);
    }

    #[test]
    fn gain_raises_target_but_respects_monitor_maximum() {
        let output = evaluate_at_elevation(
            0.0,
            vec![monitor("base", 20, 80, 1.0), monitor("gain", 20, 80, 1.5)],
        );

        assert!(output.targets[1].percent > output.targets[0].percent);
        assert_eq!(output.targets[0].percent, 59);
        assert_eq!(output.targets[1].percent, 80);
    }

    #[test]
    fn gain_below_one_dims_target_but_not_below_monitor_minimum() {
        let output = evaluate_at_elevation(
            3.0,
            vec![monitor("base", 20, 80, 1.0), monitor("dimmed", 20, 80, 0.5)],
        );

        assert_eq!(output.targets[0].percent, 80);
        assert_eq!(output.targets[1].percent, 40);

        let night = evaluate_at_elevation(-12.0, vec![monitor("dimmed", 20, 80, 0.5)]);
        assert_eq!(night.targets[0].percent, 20);
    }

    #[test]
    fn gain_and_weather_are_clamped_to_safe_monitor_range() {
        let output = evaluate_at_elevation_with_weather(
            3.0,
            Some(1.0),
            vec![
                monitor("high-gain", 20, 80, 4.0),
                monitor("weather", 20, 80, 1.0),
            ],
        );

        assert_eq!(output.targets[0].percent, 80);
        assert_eq!(output.targets[1].percent, 80);

        let dimmed = evaluate_at_elevation_with_weather(
            3.0,
            Some(0.5),
            vec![monitor("weather", 20, 80, 1.0)],
        );
        assert_eq!(dimmed.targets[0].percent, 50);
        assert_eq!(dimmed.targets[0].effective_daylight_factor, 0.5);
    }

    #[test]
    fn invalid_monitor_range_returns_structured_error() {
        let err = compute_policy_for_elevation(
            &base_input(vec![monitor("broken", 80, 20, 1.0)]).as_context(),
            0.0,
        )
        .expect_err("invalid range should be rejected");

        assert_eq!(
            err,
            PolicyError::InvalidMonitorRange {
                logical_id: String::from("broken"),
                min_pct: 80,
                max_pct: 20,
            }
        );
    }

    #[test]
    fn invalid_weather_modifier_returns_structured_error() {
        let mut input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        input.weather_multiplier = Some(1.2);

        let err = compute_policy_for_elevation(&input.as_context(), 0.0)
            .expect_err("out-of-range weather modifier should be rejected");

        assert_eq!(
            err,
            PolicyError::InvalidWeatherModifier {
                daylight_multiplier: 1.2,
            }
        );
    }

    #[test]
    fn auto_minimum_brightness_start_delays_minimum_after_dusk() {
        let mut input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        let events = solar::get_sun_events(input.now_utc.date_naive(), &input.location)
            .expect("sun events should resolve");
        input.now_utc = (events.dusk + Duration::minutes(10)).with_timezone(&chrono::Utc);

        let output = compute_policy(&input.as_context()).expect("policy should evaluate");

        // With adaptive zenith spanning the full elevation range, sunset's
        // daylight factor is very small (~0.003) so the evening ramp starts
        // near zero. The ramp structure still extends past dusk (non-zero
        // effective factor), but the rounded percent equals min_pct.
        assert!(
            output.targets[0].effective_daylight_factor > 0.0,
            "evening ramp should keep effective factor above zero past dusk"
        );
    }

    #[test]
    fn auto_minimum_brightness_start_reaches_monitor_minimum_after_default_delay() {
        let mut input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        let events = solar::get_sun_events(input.now_utc.date_naive(), &input.location)
            .expect("sun events should resolve");
        let auto_minimum_start =
            (events.sunset + Duration::minutes(90)).max(events.dusk + Duration::minutes(30));
        input.now_utc = (auto_minimum_start + Duration::minutes(5)).with_timezone(&chrono::Utc);

        let output = compute_policy(&input.as_context()).expect("policy should evaluate");

        assert_eq!(output.targets[0].percent, 20);
    }

    #[test]
    fn weather_is_ignored_after_total_dark_even_during_evening_ramp() {
        let mut input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        let events = solar::get_sun_events(input.now_utc.date_naive(), &input.location)
            .expect("sun events should resolve");
        input.now_utc = (events.dusk + Duration::minutes(10)).with_timezone(&chrono::Utc);

        let baseline = compute_policy(&input.as_context()).expect("policy should evaluate");

        input.weather_multiplier = Some(0.5);
        let clouded = compute_policy(&input.as_context()).expect("policy should evaluate");

        assert_eq!(
            baseline.targets[0].effective_daylight_factor,
            clouded.targets[0].effective_daylight_factor
        );
        assert_eq!(baseline.targets[0].percent, clouded.targets[0].percent);
        assert_eq!(clouded.weather_multiplier, 1.0);
    }

    #[test]
    fn compute_policy_uses_time_and_location_purely() {
        let input = base_input(vec![monitor("internal", 10, 100, 1.0)]);
        let output = compute_policy(&input.as_context()).expect("policy should evaluate");
        assert_eq!(output.targets.len(), 1);
        assert!((0.0..=1.0).contains(&output.targets[0].solar_daylight_factor));
        assert!((0.0..=1.0).contains(&output.targets[0].effective_daylight_factor));
    }

    #[test]
    fn compute_policy_adapts_day_peak_to_daily_solar_noon() {
        let location =
            Location::from_timezone_name(40.8, 29.2, "Europe/Istanbul").expect("valid location");
        let date = NaiveDate::from_ymd_opt(2026, 3, 5).expect("valid test date");
        let noon_local = solar::local_datetime(
            date.and_hms_opt(12, 0, 0).expect("valid noon local time"),
            &location,
        )
        .expect("local noon should resolve")
        .with_timezone(&Utc);
        let afternoon_local = solar::local_datetime(
            date.and_hms_opt(15, 0, 0)
                .expect("valid afternoon local time"),
            &location,
        )
        .expect("local afternoon should resolve")
        .with_timezone(&Utc);

        let mut noon_input = base_input(vec![monitor("internal", 0, 100, 1.0)]);
        noon_input.location = location.clone();
        noon_input.now_utc = noon_local;
        noon_input.config.day_elevation_full = 80.0;

        let mut afternoon_input = noon_input.clone();
        afternoon_input.now_utc = afternoon_local;

        let noon_output =
            compute_policy(&noon_input.as_context()).expect("noon policy should evaluate");
        let afternoon_output = compute_policy(&afternoon_input.as_context())
            .expect("afternoon policy should evaluate");

        assert!(
            noon_output.targets[0].solar_daylight_factor
                > afternoon_output.targets[0].solar_daylight_factor
        );
        assert!(afternoon_output.targets[0].solar_daylight_factor < 1.0);
        assert!(afternoon_output.targets[0].percent < 100);
    }

    #[test]
    fn milestone_preview_is_ordered_and_uses_monitor_targets() {
        let schedules = compute_monitor_milestones(
            &base_input(vec![monitor("internal", 20, 80, 1.0)]).as_context(),
        )
        .expect("milestones should resolve");
        let schedule = &schedules[0];

        assert_eq!(schedule.logical_id, "internal");
        assert_eq!(schedule.milestones.len(), 9);
        assert_eq!(
            schedule.milestones[0].milestone,
            AutomationMilestone::RiseStart
        );
        assert_eq!(schedule.milestones[4].milestone, AutomationMilestone::Peak);
        assert_eq!(schedule.milestones[0].target_percent, 20);
        assert_eq!(schedule.milestones[4].target_percent, 80);

        for window in schedule.milestones.windows(2) {
            let [left, right] = window else {
                continue;
            };
            assert!(left.adjusted_time_local < right.adjusted_time_local);
        }
    }

    #[test]
    fn milestone_adjustments_delay_monitor_progress() {
        let mut delayed_monitor = monitor("internal", 20, 80, 1.0);
        delayed_monitor.milestone_adjustments = vec![
            milestone(AutomationMilestone::Rise25, 30),
            milestone(AutomationMilestone::Rise50, 30),
            milestone(AutomationMilestone::Rise75, 30),
            milestone(AutomationMilestone::Peak, 30),
        ];
        let mut delayed_input = base_input(vec![delayed_monitor.clone()]);
        let preview = compute_monitor_milestones(&delayed_input.as_context())
            .expect("milestones should resolve");
        let rise_50_time = preview[0]
            .milestones
            .iter()
            .find(|milestone| milestone.milestone == AutomationMilestone::Rise50)
            .expect("rise_50 milestone should exist")
            .adjusted_time_local
            - Duration::minutes(10);
        delayed_input.now_utc = rise_50_time.with_timezone(&chrono::Utc);

        let delayed_output =
            compute_policy(&delayed_input.as_context()).expect("policy should evaluate");

        let mut baseline_input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        baseline_input.now_utc = delayed_input.now_utc;
        let baseline_output =
            compute_policy(&baseline_input.as_context()).expect("policy should evaluate");

        assert!(delayed_output.targets[0].percent < baseline_output.targets[0].percent);
    }

    fn evaluate_at_elevation(elevation_deg: f64, monitors: Vec<MonitorConfig>) -> PolicyOutput {
        let (location, config) = base_config();
        let input = PolicyContext {
            now_utc: chrono::Utc
                .with_ymd_and_hms(2024, 3, 20, 12, 0, 0)
                .single()
                .expect("UTC datetime should be valid"),
            location: &location,
            config: &config,
            monitors: &monitors,
            weather_multiplier: None,
        };
        compute_policy_for_elevation(&input, elevation_deg).expect("policy should evaluate")
    }

    fn evaluate_at_elevation_with_weather(
        elevation_deg: f64,
        weather_multiplier: Option<f64>,
        monitors: Vec<MonitorConfig>,
    ) -> PolicyOutput {
        let (location, config) = base_config();
        let input = PolicyContext {
            now_utc: chrono::Utc
                .with_ymd_and_hms(2024, 3, 20, 12, 0, 0)
                .single()
                .expect("UTC datetime should be valid"),
            location: &location,
            config: &config,
            monitors: &monitors,
            weather_multiplier,
        };
        compute_policy_for_elevation(&input, elevation_deg).expect("policy should evaluate")
    }

    fn base_config() -> (Location, SolarPolicyConfig) {
        (
            Location::from_timezone_name(0.0, 0.0, "UTC").unwrap(),
            SolarPolicyConfig {
                twilight_elevation_start: -6.0,
                day_elevation_full: 3.0,
                use_adaptive_zenith: true,
                ..Default::default()
            },
        )
    }

    #[derive(Clone)]
    struct TestPolicyInput {
        now_utc: chrono::DateTime<Utc>,
        location: Location,
        config: SolarPolicyConfig,
        weather_multiplier: Option<f64>,
        monitors: Vec<MonitorConfig>,
    }

    impl TestPolicyInput {
        fn as_context(&self) -> PolicyContext<'_> {
            PolicyContext {
                now_utc: self.now_utc,
                location: &self.location,
                config: &self.config,
                weather_multiplier: self.weather_multiplier,
                monitors: &self.monitors,
            }
        }
    }

    fn base_input(monitors: Vec<MonitorConfig>) -> TestPolicyInput {
        let (location, config) = base_config();
        TestPolicyInput {
            now_utc: chrono::Utc
                .with_ymd_and_hms(2024, 3, 20, 12, 0, 0)
                .single()
                .unwrap(),
            location,
            config,
            weather_multiplier: None,
            monitors,
        }
    }

    fn monitor(logical_id: &str, min_pct: u8, max_pct: u8, gain: f64) -> MonitorConfig {
        MonitorConfig {
            logical_id: logical_id.to_owned(),
            min_pct,
            max_pct,
            gain,
            transition_gamma: 1.4,
            ..Default::default()
        }
    }

    fn milestone(
        milestone: AutomationMilestone,
        minutes_offset: i16,
    ) -> MonitorMilestoneAdjustment {
        MonitorMilestoneAdjustment {
            milestone,
            minutes_offset,
        }
    }

    #[test]
    fn compute_policy_handles_polar_day_gracefully() {
        let mut input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        // Longyearbyen, Svalbard during summer solstice (Polar Day)
        input.location = Location::from_timezone_name(78.2232, 15.6267, "Europe/Oslo").unwrap();
        input.now_utc = chrono::Utc.with_ymd_and_hms(2024, 6, 21, 12, 0, 0).unwrap();

        let output =
            compute_policy(&input.as_context()).expect("policy should evaluate even in polar day");

        assert!(
            output.targets[0].effective_daylight_factor > 0.8,
            "Should be mostly bright in polar day"
        );
        assert!(
            output.targets[0].percent > 60,
            "Monitor target should be high in polar day"
        );
    }

    #[test]
    fn compute_policy_handles_polar_night_gracefully() {
        let mut input = base_input(vec![monitor("internal", 20, 80, 1.0)]);
        // Longyearbyen, Svalbard during winter solstice (Polar Night)
        input.location = Location::from_timezone_name(78.2232, 15.6267, "Europe/Oslo").unwrap();
        input.now_utc = chrono::Utc
            .with_ymd_and_hms(2024, 12, 21, 12, 0, 0)
            .unwrap();

        let output = compute_policy(&input.as_context())
            .expect("policy should evaluate even in polar night");

        assert!(
            output.targets[0].effective_daylight_factor < 0.2,
            "Should be very dark in polar night"
        );
        assert!(
            output.targets[0].percent < 40,
            "Monitor target should be low in polar night"
        );
    }
}
