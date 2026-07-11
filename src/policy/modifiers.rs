use crate::policy::math::{daylight_factor_from_elevation, round_and_clamp_percent};
use crate::policy::milestones::{
    adjusted_milestones, relevant_evening_events, remap_adjusted_time_to_base,
    resolve_auto_minimum_brightness_start, BaseMilestone, BaseMilestoneContext,
};
use crate::policy::{PolicyContext, PolicyError};
use crate::solar;
use chrono::{DateTime, Utc};

pub(crate) fn compute_monitor_target_percent(
    monitor: &crate::config::MonitorConfig,
    daylight_factor: f64,
) -> u8 {
    let min_pct = f64::from(monitor.min_pct);
    let max_pct = f64::from(monitor.max_pct);
    let base_percent = min_pct + daylight_factor * (max_pct - min_pct);
    // Gain scales the monitor-local target after the solar/weather curve has
    // been mapped into that monitor's range, then clamps back into min/max.
    let gained_percent = base_percent * monitor.gain;
    round_and_clamp_percent(gained_percent, monitor.min_pct, monitor.max_pct)
}
pub(crate) fn daylight_factor_with_adjustments(
    input: &PolicyContext,
    monitor: &crate::config::MonitorConfig,
    now_local: DateTime<chrono::FixedOffset>,
    context: &BaseMilestoneContext,
    base_milestones: &[BaseMilestone],
) -> Result<f64, PolicyError> {
    let adjusted = adjusted_milestones(monitor, context, base_milestones);
    let first = adjusted
        .first()
        .expect("milestone list should not be empty");
    let last = adjusted.last().expect("milestone list should not be empty");

    if now_local <= first.adjusted_time_local || now_local >= last.adjusted_time_local {
        return Ok(0.0);
    }

    for window in adjusted.windows(2) {
        let [current, next] = window else {
            continue;
        };

        if now_local < current.adjusted_time_local || now_local > next.adjusted_time_local {
            continue;
        }

        let mapped_local = remap_adjusted_time_to_base(now_local, current, next);
        let linear_factor = crate::policy::base_linear_effective_daylight_factor_at_local(
            input,
            context,
            mapped_local,
        )?;
        return Ok(crate::policy::math::apply_gamma(
            linear_factor,
            monitor.transition_gamma,
        ));
    }

    Ok(0.0)
}
pub(crate) fn validate_policy_input(input: &PolicyContext) -> Result<(), PolicyError> {
    validate_solar_curve(
        input.config.twilight_elevation_start,
        input.config.day_elevation_full,
    )?;
    let _ = weather_multiplier(input.weather_multiplier)?;
    for monitor in input.monitors {
        validate_monitor(monitor)?;
    }
    Ok(())
}
pub(crate) fn validate_solar_curve(
    twilight_elevation_start_deg: f64,
    day_elevation_full_deg: f64,
) -> Result<(), PolicyError> {
    if !twilight_elevation_start_deg.is_finite()
        || !day_elevation_full_deg.is_finite()
        || twilight_elevation_start_deg >= day_elevation_full_deg
    {
        return Err(PolicyError::InvalidSolarCurve {
            twilight_elevation_start_deg,
            day_elevation_full_deg,
        });
    }

    Ok(())
}
pub(crate) fn weather_multiplier(modifier: Option<f64>) -> Result<f64, PolicyError> {
    match modifier {
        Some(multiplier) if !multiplier.is_finite() || !(0.0..=1.0).contains(&multiplier) => {
            Err(PolicyError::InvalidWeatherModifier {
                daylight_multiplier: multiplier,
            })
        }
        Some(multiplier) => Ok(multiplier),
        None => Ok(1.0),
    }
}
pub(crate) fn weather_multiplier_for_solar_phase(
    modifier: Option<f64>,
    solar_daylight_factor: f64,
) -> Result<f64, PolicyError> {
    let multiplier = weather_multiplier(modifier)?;
    if solar_daylight_factor <= 0.0 {
        Ok(1.0)
    } else {
        Ok(multiplier)
    }
}
pub(crate) fn adjust_evening_daylight_factor(
    input: &PolicyContext,
    monitor: &crate::config::MonitorConfig,
    solar_daylight_factor: f64,
) -> Result<f64, PolicyError> {
    let now_local = solar::local_datetime_at_utc(input.now_utc, input.location)?;
    let Some(evening_events) = relevant_evening_events(now_local, input.location)? else {
        return Ok(solar_daylight_factor);
    };
    let minimum_brightness_start = resolve_auto_minimum_brightness_start(&evening_events);

    if now_local >= minimum_brightness_start {
        return Ok(0.0);
    }

    let total_seconds = (minimum_brightness_start - evening_events.sunset).num_seconds();
    if total_seconds <= 0 {
        return Ok(0.0);
    }

    let sunset_sample = solar::sample_at_utc(
        evening_events.sunset.with_timezone(&Utc),
        input.location,
        input.config.twilight_elevation_start,
        input.config.day_elevation_full,
    )?;
    let sunset_daylight_factor = daylight_factor_from_elevation(
        f64::from(sunset_sample.elevation_deg),
        input.config.twilight_elevation_start,
        input.config.day_elevation_full,
        monitor.transition_gamma,
    )?;
    let elapsed_seconds = (now_local - evening_events.sunset)
        .num_seconds()
        .clamp(0, total_seconds);
    let remaining_ratio = 1.0 - (elapsed_seconds as f64 / total_seconds as f64);

    Ok((sunset_daylight_factor * remaining_ratio).clamp(0.0, 1.0))
}
pub(crate) fn validate_monitor(monitor: &crate::config::MonitorConfig) -> Result<(), PolicyError> {
    if !monitor.transition_gamma.is_finite() || monitor.transition_gamma <= 0.0 {
        return Err(PolicyError::InvalidMonitorConfig {
            transition_gamma: monitor.transition_gamma,
        });
    }

    if monitor.min_pct > 100 || monitor.max_pct > 100 || monitor.min_pct > monitor.max_pct {
        return Err(PolicyError::InvalidMonitorRange {
            logical_id: monitor.logical_id.clone(),
            min_pct: monitor.min_pct,
            max_pct: monitor.max_pct,
        });
    }

    if !monitor.gain.is_finite() || monitor.gain <= 0.0 {
        return Err(PolicyError::InvalidMonitorGain {
            logical_id: monitor.logical_id.clone(),
            gain: monitor.gain,
        });
    }

    Ok(())
}
