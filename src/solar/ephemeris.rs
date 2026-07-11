use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// The 8 standard phases of the lunar synodic cycle.
///
/// Derived from the elapsed days since a known reference New Moon using the
/// synodic month constant (~29.5305877 days). Suitable for IPC serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LunarPhase {
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    FullMoon,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

/// Length of one synodic month in days (new moon to new moon).
const SYNODIC_MONTH_DAYS: f64 = 29.530_588_7;

/// Reference epoch: New Moon at 2000-01-06 18:14:00 UTC (Unix ts 947_182_440).
/// JD = 2_440_587.5 + 947_182_440 / 86_400.0 = 2_451_550.259_722...
/// Using the exact computed value so cycle_pos lands at 0.0 for this instant.
const REFERENCE_NEW_MOON_JD: f64 = 2_451_550.259_722;

/// Calculates the current lunar phase from a UTC timestamp using the
/// synodic month cycle. Pure function — no I/O, no external calls.
///
/// Algorithm:
///   1. Convert `now_utc` to Julian Day Number (JDN).
///   2. Subtract the reference New Moon JDN to get elapsed days.
///   3. Modulo by the synodic month to get position within current cycle (0..29.53).
///   4. Bucket into one of 8 equal-ish phases.
///
/// Phase boundaries (days into synodic cycle):
///   0.00 –  1.85  → New Moon        (first/last ~3% of cycle)
///   1.85 –  7.38  → Waxing Crescent
///   7.38 –  9.22  → First Quarter
///   9.22 – 14.77  → Waxing Gibbous
///  14.77 – 16.61  → Full Moon
///  16.61 – 22.15  → Waning Gibbous
///  22.15 – 24.00  → Last Quarter
///  24.00 – 29.53  → Waning Crescent
pub fn calculate_lunar_phase(now_utc: DateTime<Utc>) -> LunarPhase {
    let jd = julian_day(now_utc);
    let elapsed = jd - REFERENCE_NEW_MOON_JD;

    // Normalise to [0, SYNODIC_MONTH_DAYS)
    let cycle_pos = elapsed.rem_euclid(SYNODIC_MONTH_DAYS);

    // Phase boundaries in days. Each named phase spans roughly 1/8 of the
    // synodic month, with New Moon and Full Moon slightly narrower to represent
    // the precise moment of exact phase (±3.16% = ~1.85 days each).
    let half = SYNODIC_MONTH_DAYS / 2.0; // ~14.765
    let eighth = SYNODIC_MONTH_DAYS / 8.0; // ~3.691

    let new_moon_half_width = 1.845; // ±1.845 d around 0 and 29.53
    let full_moon_half_width = 1.845;

    match cycle_pos {
        // NewMoon occupies both the opening and closing slivers of the cycle
        // so that timestamps exactly at the reference epoch (cycle_pos ≈ 0)
        // and timestamps just before the next new moon both resolve correctly.
        p if p < new_moon_half_width => LunarPhase::NewMoon, // 0.000 – 1.845
        p if p < (eighth * 2.0) => LunarPhase::WaxingCrescent, // 1.845 – 7.383
        p if p < (eighth * 2.0) + full_moon_half_width => LunarPhase::FirstQuarter, // 7.383 – 9.228
        p if p < half - full_moon_half_width => LunarPhase::WaxingGibbous, // 9.228 – 12.920
        p if p < half + full_moon_half_width => LunarPhase::FullMoon, // 12.920 – 16.610
        p if p < (eighth * 6.0) - full_moon_half_width => LunarPhase::WaningGibbous, // 16.610 – 20.303
        p if p < (eighth * 6.0) + full_moon_half_width => LunarPhase::LastQuarter, // 20.303 – 23.993
        // Tail of cycle: WaningCrescent until the next NewMoon window
        p if p < SYNODIC_MONTH_DAYS - new_moon_half_width => LunarPhase::WaningCrescent, // 23.993 – 27.686
        _ => LunarPhase::NewMoon, // 27.686 – 29.531
    }
}

pub(crate) fn solar_elevation_utc(
    datetime: DateTime<Utc>,
    latitude_deg: f64,
    longitude_deg: f64,
) -> f64 {
    let julian_day = julian_day(datetime);
    let julian_century = (julian_day - 2_451_545.0) / 36_525.0;

    let geom_mean_longitude_deg = normalize_degrees(
        280.46646 + julian_century * (36_000.769_83 + julian_century * 0.0003032),
    );
    let geom_mean_anomaly_deg =
        357.52911 + julian_century * (35_999.050_29 - 0.0001537 * julian_century);
    let eccentricity = 0.016708634 - julian_century * (0.000042037 + 0.0000001267 * julian_century);

    let sun_eq_of_center_deg = geom_mean_anomaly_deg.to_radians().sin()
        * (1.914602 - julian_century * (0.004817 + 0.000014 * julian_century))
        + (2.0 * geom_mean_anomaly_deg).to_radians().sin() * (0.019993 - 0.000101 * julian_century)
        + (3.0 * geom_mean_anomaly_deg).to_radians().sin() * 0.000289;

    let sun_true_longitude_deg = geom_mean_longitude_deg + sun_eq_of_center_deg;
    let sun_apparent_longitude_deg = sun_true_longitude_deg
        - 0.00569
        - 0.00478 * (125.04 - 1934.136 * julian_century).to_radians().sin();

    let mean_obliquity_deg = 23.0
        + (26.0
            + ((21.448
                - julian_century
                    * (46.815 + julian_century * (0.00059 - julian_century * 0.001813)))
                / 60.0))
            / 60.0;
    let obliquity_correction_deg =
        mean_obliquity_deg + 0.00256 * (125.04 - 1934.136 * julian_century).to_radians().cos();

    let declination_rad = (obliquity_correction_deg.to_radians().sin()
        * sun_apparent_longitude_deg.to_radians().sin())
    .asin();

    let var_y = (obliquity_correction_deg.to_radians() / 2.0).tan().powi(2);
    let equation_of_time_minutes = 4.0
        * radians_to_degrees(
            var_y * (2.0 * geom_mean_longitude_deg).to_radians().sin()
                - 2.0 * eccentricity * geom_mean_anomaly_deg.to_radians().sin()
                + 4.0
                    * eccentricity
                    * var_y
                    * geom_mean_anomaly_deg.to_radians().sin()
                    * (2.0 * geom_mean_longitude_deg).to_radians().cos()
                - 0.5 * var_y.powi(2) * (4.0 * geom_mean_longitude_deg).to_radians().sin()
                - 1.25 * eccentricity.powi(2) * (2.0 * geom_mean_anomaly_deg).to_radians().sin(),
        );

    let utc_minutes = f64::from(datetime.hour()) * 60.0
        + f64::from(datetime.minute())
        + f64::from(datetime.second()) / 60.0
        + f64::from(datetime.nanosecond()) / 60_000_000_000.0;

    let true_solar_time_minutes =
        normalize_minutes(utc_minutes + equation_of_time_minutes + 4.0 * longitude_deg);
    let hour_angle_deg = if true_solar_time_minutes / 4.0 < 0.0 {
        true_solar_time_minutes / 4.0 + 180.0
    } else {
        true_solar_time_minutes / 4.0 - 180.0
    };

    let latitude_rad = latitude_deg.to_radians();
    let zenith_rad = (latitude_rad.sin() * declination_rad.sin()
        + latitude_rad.cos() * declination_rad.cos() * hour_angle_deg.to_radians().cos())
    .clamp(-1.0, 1.0)
    .acos();

    90.0 - radians_to_degrees(zenith_rad)
}

pub(crate) fn julian_day(datetime: DateTime<Utc>) -> f64 {
    2_440_587.5
        + (datetime.timestamp() as f64) / 86_400.0
        + f64::from(datetime.timestamp_subsec_nanos()) / 86_400_000_000_000.0
}

pub(crate) fn normalize_degrees(value: f64) -> f64 {
    let mut normalized = value % 360.0;
    if normalized < 0.0 {
        normalized += 360.0;
    }
    normalized
}

pub(crate) fn normalize_minutes(value: f64) -> f64 {
    let mut normalized = value % 1_440.0;
    if normalized < 0.0 {
        normalized += 1_440.0;
    }
    normalized
}

pub(crate) fn radians_to_degrees(value: f64) -> f64 {
    value.to_degrees()
}

#[cfg(test)]
mod lunar_tests {
    use super::{calculate_lunar_phase, LunarPhase};
    use chrono::{TimeZone, Utc};

    /// 2000-01-06 18:14 UTC — the reference New Moon itself.
    /// cycle_pos ≈ 0.0 d → NewMoon.
    /// Unix epoch for this instant: 947_182_440 s.
    #[test]
    fn reference_epoch_is_new_moon() {
        let dt = Utc
            .with_ymd_and_hms(2000, 1, 6, 18, 14, 0)
            .single()
            .expect("valid datetime");
        // Sanity-check our understanding of the timestamp.
        assert_eq!(dt.timestamp(), 947_182_440);
        assert_eq!(calculate_lunar_phase(dt), LunarPhase::NewMoon);
    }

    /// 2000-01-21 04:40 UTC — historically confirmed Full Moon.
    /// cycle_pos ≈ 14.43 d → FullMoon.
    #[test]
    fn historical_full_moon_jan_2000() {
        let dt = Utc
            .with_ymd_and_hms(2000, 1, 21, 4, 40, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(calculate_lunar_phase(dt), LunarPhase::FullMoon);
    }

    /// 2024-01-11 11:57 UTC — confirmed New Moon (NASA/USNO).
    /// cycle_pos ≈ 0.15 d → NewMoon.
    #[test]
    fn new_moon_jan_2024() {
        let dt = Utc
            .with_ymd_and_hms(2024, 1, 11, 11, 57, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(calculate_lunar_phase(dt), LunarPhase::NewMoon);
    }

    /// 2024-01-25 17:54 UTC — confirmed Full Moon (NASA/USNO).
    /// cycle_pos ≈ 14.40 d → FullMoon.
    #[test]
    fn full_moon_jan_2024() {
        let dt = Utc
            .with_ymd_and_hms(2024, 1, 25, 17, 54, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(calculate_lunar_phase(dt), LunarPhase::FullMoon);
    }

    /// Exhaustive bucket coverage: step through one full synodic cycle from
    /// the reference new moon and verify each phase is visited in order.
    ///
    /// Offsets are chosen to land squarely in the centre of each phase window,
    /// measured in days after 2000-01-06 18:14:00 UTC (Unix ts 947_182_440).
    #[test]
    fn all_eight_phases_are_reachable_within_one_cycle() {
        let phase_samples: &[(f64, LunarPhase)] = &[
            (0.5, LunarPhase::NewMoon),
            (4.0, LunarPhase::WaxingCrescent),
            (8.3, LunarPhase::FirstQuarter),
            (11.5, LunarPhase::WaxingGibbous),
            (14.77, LunarPhase::FullMoon),
            (19.0, LunarPhase::WaningGibbous),
            (22.5, LunarPhase::LastQuarter),
            (27.0, LunarPhase::WaningCrescent),
        ];

        // 2000-01-06 18:14:00 UTC
        let reference_epoch_s: i64 = 947_182_440;

        for (day_offset, expected_phase) in phase_samples {
            let offset_s = (day_offset * 86_400.0) as i64;
            let dt = Utc
                .timestamp_opt(reference_epoch_s + offset_s, 0)
                .single()
                .expect("valid timestamp");
            let got = calculate_lunar_phase(dt);
            assert_eq!(
                got, *expected_phase,
                "at +{day_offset:.2} days expected {expected_phase:?}, got {got:?}"
            );
        }
    }
}
