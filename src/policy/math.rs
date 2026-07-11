use crate::policy::{compute_adaptive_zenith, validate_solar_curve, PolicyError};
use crate::solar;
use chrono::{DateTime, Utc};

// Removed compute_linear_daylight_factor and apply_perceptual_curve as they are now dead code.

pub fn compute_smoothstep_daylight_factor(
    current_elevation: f64,
    plateau_elevation: f64,
    night_elevation: f64,
    transition_gamma: f64,
) -> f64 {
    let linear = compute_linear_smoothstep(current_elevation, plateau_elevation, night_elevation);
    apply_gamma(linear, transition_gamma)
}

pub fn compute_linear_smoothstep(
    current_elevation: f64,
    plateau_elevation: f64,
    night_elevation: f64,
) -> f64 {
    if current_elevation >= plateau_elevation {
        return 1.0;
    }
    if current_elevation <= night_elevation {
        return 0.0;
    }

    let t = (current_elevation - night_elevation) / (plateau_elevation - night_elevation);
    t * t * (3.0 - 2.0 * t)
}

pub fn apply_gamma(factor: f64, transition_gamma: f64) -> f64 {
    factor.powf(transition_gamma)
}
pub fn project_to_hardware(
    perceptual_factor: f64,
    min_pct: u8,
    max_pct: u8,
    gain: f64, // The per-monitor gain modifier
) -> u8 {
    let clamped_min = min_pct.min(100);
    let clamped_max = max_pct.max(clamped_min).min(100);

    // Apply gain before projecting to range, clamping the intermediate factor again
    let gained_factor = (perceptual_factor * gain).clamp(0.0, 1.0);

    let range = f64::from(clamped_max - clamped_min);
    let target = f64::from(clamped_min) + (gained_factor * range);
    target.round() as u8
}
pub fn compute_brightness_target(
    now_utc: DateTime<Utc>,
    location: &solar::Location,
    twilight_start_deg: f64,
    config_day_full_deg: f64,
    use_adaptive_zenith: bool,
    transition_gamma: f64,
    min_pct: u8,
    max_pct: u8,
    gain: f64,
) -> Result<u8, PolicyError> {
    let zenith_deg = compute_adaptive_zenith(
        now_utc,
        location,
        config_day_full_deg,
        use_adaptive_zenith,
        twilight_start_deg,
    );

    let current_sample = solar::sample_at_utc(now_utc, location, twilight_start_deg, zenith_deg)?;

    let perceptual = daylight_factor_from_elevation(
        f64::from(current_sample.elevation_deg),
        twilight_start_deg,
        zenith_deg, // this is already effective_plateau
        transition_gamma,
    )?;
    Ok(project_to_hardware(perceptual, min_pct, max_pct, gain))
}
pub fn daylight_factor_from_elevation(
    elevation_deg: f64,
    twilight_elevation_start_deg: f64,
    day_elevation_full_deg: f64,
    transition_gamma: f64,
) -> Result<f64, PolicyError> {
    validate_solar_curve(twilight_elevation_start_deg, day_elevation_full_deg)?;

    // day_elevation_full_deg is already the effective_plateau calculated in compute_adaptive_zenith
    // But as a failsafe, ensure plateau > twilight_elevation_start_deg
    let plateau = day_elevation_full_deg.max(twilight_elevation_start_deg + 0.1);

    Ok(compute_smoothstep_daylight_factor(
        elevation_deg,
        plateau,
        twilight_elevation_start_deg,
        transition_gamma,
    ))
}
pub(crate) fn round_and_clamp_percent(value: f64, min_pct: u8, max_pct: u8) -> u8 {
    value
        .round()
        .clamp(f64::from(min_pct), f64::from(max_pct))
        .clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summer_plateau_is_respected() {
        let zenith = 73.0; // The adaptive zenith ensures effective plateau is 15.0.
                           // Here we test daylight_factor_from_elevation assuming the plateau has already been resolved to 15.0 by compute_adaptive_zenith.
        let plateau = 15.0;
        let twilight = -6.0;
        let gamma = 1.0;
        let current_elevation = 40.0;

        let factor = daylight_factor_from_elevation(current_elevation, twilight, plateau, gamma)
            .expect("valid params");
        assert!(
            (factor - 1.0).abs() < f64::EPSILON,
            "Factor should be exactly 1.0 when elevation is above plateau. Got: {}",
            factor
        );
    }

    #[test]
    fn test_smoothstep_midpoint_is_half() {
        let plateau = 15.0;
        let twilight = -6.0;
        let gamma = 1.0;
        let current_elevation = 4.5; // Exactly halfway between 15.0 and -6.0

        let factor = daylight_factor_from_elevation(current_elevation, twilight, plateau, gamma)
            .expect("valid params");
        assert!(
            (factor - 0.5).abs() < f64::EPSILON,
            "Factor should be exactly 0.5 at the mathematical midpoint. Got: {}",
            factor
        );
    }

    #[test]
    fn test_winter_adaptation_lowers_plateau() {
        let zenith: f64 = 10.0; // Winter solstice simulation
        let config_plateau: f64 = 15.0;
        let twilight: f64 = -6.0;
        let gamma: f64 = 1.0;
        let current_elevation: f64 = 10.0;

        let effective_plateau = config_plateau.min(zenith).max(twilight + 0.1);

        let factor =
            daylight_factor_from_elevation(current_elevation, twilight, effective_plateau, gamma)
                .expect("valid params");
        assert!(
            (factor - 1.0).abs() < f64::EPSILON,
            "Factor should be exactly 1.0 when sun reaches its winter peak of 10.0. Got: {}",
            factor
        );
    }
}
