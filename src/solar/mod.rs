pub(crate) mod ephemeris;
pub(crate) mod search;
pub mod types;

pub use ephemeris::LunarPhase;
pub use ephemeris::{calculate_lunar_phase, lunar_age_fraction};
pub use types::*;

use crate::solar::ephemeris::solar_elevation_utc;
use crate::solar::search::{find_event_crossing, find_solar_noon, safe_find_event_crossing};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use std::cell::RefCell;
use tz::datetime::{DateTime as TzDateTime, FoundDateTimeKind};

const CIVIL_TWILIGHT_ELEVATION_DEG: f64 = -6.0;
// Conventional sunrise/sunset center used by NOAA-style civil calculations:
// solar-disc center at approximately -0.833 degrees, combining the apparent
// solar radius and a standard near-horizon refraction allowance. This is an
// estimate, not an observation-time guarantee under arbitrary atmosphere.
const SUNRISE_SUNSET_ELEVATION_DEG: f64 = -0.833;
const COARSE_SEARCH_STEP_MINUTES: i64 = 10;
const FINE_SEARCH_STEP_MINUTES: i64 = 1;
const NOON_FINE_WINDOW_MINUTES: i64 = 10;
const NOON_FINAL_WINDOW_SECONDS: i64 = 60;

thread_local! {
    static LAST_SUN_EVENTS: RefCell<Option<(NaiveDate, Location, SunEvents)>> = const { RefCell::new(None) };
}

/// Returns dawn, sunrise, solar noon, sunset, and dusk for a local date.
///
/// The input `date` is interpreted in `location.timezone`.
///
/// At extreme latitudes, some events may not exist on a given date. In that
/// case this function returns `SolarError::SunNeverCrossesThreshold` for the
/// first missing event rather than inventing a placeholder time.
///
/// Caches the most recently calculated result per thread to avoid redundant
pub fn get_sun_events(date: NaiveDate, location: &Location) -> Result<SunEvents, SolarError> {
    validate_location(location)?;

    if let Some((cached_date, cached_location, cached_events)) =
        LAST_SUN_EVENTS.with(|cache| cache.borrow().clone())
    {
        if cached_date == date && cached_location == *location {
            return Ok(cached_events);
        }
    }

    let timezone_label = location.timezone_name.clone();
    let start_local = resolve_date_boundary(date, &location.timezone, &timezone_label)?;
    let next_date = date.succ_opt().ok_or(SolarError::DateOutOfRange)?;
    let end_local = resolve_date_boundary(next_date, &location.timezone, &timezone_label)?;
    let start_utc = start_local.with_timezone(&Utc);
    let end_utc = end_local.with_timezone(&Utc);

    let dawn = find_event_crossing(
        start_utc,
        end_utc,
        location,
        CIVIL_TWILIGHT_ELEVATION_DEG,
        CrossingDirection::Rising,
        date,
        SunEventKind::Dawn,
    )?;
    let sunrise = find_event_crossing(
        start_utc,
        end_utc,
        location,
        SUNRISE_SUNSET_ELEVATION_DEG,
        CrossingDirection::Rising,
        date,
        SunEventKind::Sunrise,
    )?;
    let noon = find_solar_noon(start_utc, end_utc, location)?;
    let sunset = find_event_crossing(
        start_utc,
        end_utc,
        location,
        SUNRISE_SUNSET_ELEVATION_DEG,
        CrossingDirection::Falling,
        date,
        SunEventKind::Sunset,
    )?;
    let dusk = find_event_crossing(
        start_utc,
        end_utc,
        location,
        CIVIL_TWILIGHT_ELEVATION_DEG,
        CrossingDirection::Falling,
        date,
        SunEventKind::Dusk,
    )?;

    let events = SunEvents {
        dawn: local_datetime_at_utc(dawn, location)?,
        sunrise: local_datetime_at_utc(sunrise, location)?,
        noon: local_datetime_at_utc(noon, location)?,
        sunset: local_datetime_at_utc(sunset, location)?,
        dusk: local_datetime_at_utc(dusk, location)?,
    };

    LAST_SUN_EVENTS.with(|cache| {
        *cache.borrow_mut() = Some((date, location.clone(), events.clone()));
    });

    Ok(events)
}

/// Returns the solar elevation in degrees for a local datetime.
///
/// `datetime` is interpreted as a local civil datetime in
/// `location.timezone`. The function returns a structured error if the local
/// time is ambiguous or nonexistent during a timezone transition.
pub fn get_solar_elevation(
    datetime: NaiveDateTime,
    location: &Location,
) -> Result<f64, SolarError> {
    let local = resolve_local_datetime(datetime, location)?;
    Ok(solar_elevation_utc(
        local.with_timezone(&Utc),
        location.latitude,
        location.longitude,
    ))
}

/// Resolves a local civil datetime in `location.timezone`.
///
/// This is the timezone-safe companion to `local_datetime_at_utc()`. It keeps
/// DST ambiguity and nonexistent-local-time handling explicit for higher-level
/// scheduling logic that stores human-readable local times.
pub fn local_datetime(
    datetime: NaiveDateTime,
    location: &Location,
) -> Result<DateTime<FixedOffset>, SolarError> {
    Ok(resolve_local_datetime(datetime, location)?.fixed_offset())
}

/// Returns a policy-friendly solar sample for a UTC instant.
///
/// This is the runtime-oriented helper that sunreactor uses to avoid guessing
/// the host timezone. `day_elevation_full_deg` and
/// `twilight_elevation_start_deg` are the same thresholds used by the config
/// model and policy engine.
pub fn sample_at_utc(
    datetime: DateTime<Utc>,
    location: &Location,
    twilight_elevation_start_deg: f64,
    day_elevation_full_deg: f64,
) -> Result<SolarSample, SolarError> {
    validate_location(location)?;

    let elevation_deg = solar_elevation_utc(datetime, location.latitude, location.longitude);
    Ok(SolarSample {
        phase: classify_elevation(
            elevation_deg,
            twilight_elevation_start_deg,
            day_elevation_full_deg,
        ),
        elevation_deg: elevation_deg as f32,
    })
}

/// Returns the local civil datetime for a UTC instant using the explicit
/// timezone configured on `location`.
pub fn local_datetime_at_utc(
    datetime: DateTime<Utc>,
    location: &Location,
) -> Result<DateTime<FixedOffset>, SolarError> {
    validate_location(location)?;
    let local_type = location
        .timezone
        .find_local_time_type(datetime.timestamp())
        .map_err(|_| SolarError::DateOutOfRange)?;
    Ok(datetime.with_timezone(&utc_offset(local_type)))
}

fn noon_utc(
    datetime: DateTime<Utc>,
    location: &Location,
) -> Result<DateTime<FixedOffset>, SolarError> {
    let local_type = location
        .timezone
        .find_local_time_type(datetime.timestamp())
        .map_err(|_| SolarError::DateOutOfRange)?;
    Ok(datetime.with_timezone(&utc_offset(local_type)))
}

fn utc_offset(local_type: &tz::LocalTimeType) -> FixedOffset {
    FixedOffset::east_opt(local_type.ut_offset())
        .expect("invariant: tz database UTC offsets are within ±24h")
}

pub(crate) fn classify_elevation(
    elevation_deg: f64,
    twilight_elevation_start_deg: f64,
    day_elevation_full_deg: f64,
) -> SolarPhase {
    if elevation_deg >= day_elevation_full_deg {
        SolarPhase::Day
    } else if elevation_deg <= twilight_elevation_start_deg {
        SolarPhase::Night
    } else {
        SolarPhase::Transition
    }
}

pub(crate) fn validate_location(location: &Location) -> Result<(), SolarError> {
    if !(-90.0..=90.0).contains(&location.latitude) {
        return Err(SolarError::InvalidLatitude {
            latitude: location.latitude,
        });
    }

    if !(-180.0..=180.0).contains(&location.longitude) {
        return Err(SolarError::InvalidLongitude {
            longitude: location.longitude,
        });
    }

    Ok(())
}

pub(crate) fn resolve_local_datetime(
    datetime: NaiveDateTime,
    location: &Location,
) -> Result<DateTime<FixedOffset>, SolarError> {
    validate_location(location)?;

    resolve_local_datetime_in_timezone(datetime, &location.timezone, &location.timezone_name)
}

/// Start of a local calendar day. Where DST skips 00:00 (Cairo, Santiago,
/// Havana, Beirut) the day starts at the earliest valid instant after the
/// missing midnight; where 00:00 happens twice the earlier occurrence wins.
/// Arbitrary wall times keep the strict [`resolve_local_datetime`] contract.
pub(crate) fn resolve_date_boundary(
    date: NaiveDate,
    tz: &tz::TimeZone,
    timezone_label: &str,
) -> Result<DateTime<FixedOffset>, SolarError> {
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(SolarError::DateOutOfRange)?;

    match resolve_local_datetime_in_timezone(midnight, tz, timezone_label) {
        Err(SolarError::NonexistentLocalTime { .. }) => {
            first_valid_instant_after(midnight, tz, timezone_label)
        }
        Err(SolarError::AmbiguousLocalTime { .. }) => earliest_occurrence(midnight, tz),
        other => other,
    }
}

/// Walks forward minute by minute from a skipped wall time to the first wall
/// time that exists (the instant the clock jumped to).
fn first_valid_instant_after(
    skipped: NaiveDateTime,
    tz: &tz::TimeZone,
    timezone_label: &str,
) -> Result<DateTime<FixedOffset>, SolarError> {
    // The longest gap ever recorded is a skipped day (Samoa, 2011).
    const MAX_GAP_MINUTES: i64 = 36 * 60;
    for minutes in 1..=MAX_GAP_MINUTES {
        let candidate = skipped + chrono::Duration::minutes(minutes);
        match resolve_local_datetime_in_timezone(candidate, tz, timezone_label) {
            Err(SolarError::NonexistentLocalTime { .. }) => {}
            Err(SolarError::AmbiguousLocalTime { .. }) => {
                return earliest_occurrence(candidate, tz)
            }
            other => return other,
        }
    }
    Err(SolarError::DateOutOfRange)
}

/// The earliest instant a repeated wall time occurs at.
fn earliest_occurrence(
    datetime: NaiveDateTime,
    tz: &tz::TimeZone,
) -> Result<DateTime<FixedOffset>, SolarError> {
    let found = TzDateTime::find(
        datetime.year(),
        datetime.month() as u8,
        datetime.day() as u8,
        datetime.hour() as u8,
        datetime.minute() as u8,
        datetime.second() as u8,
        datetime.nanosecond(),
        tz.as_ref(),
    )
    .map_err(|_| SolarError::DateOutOfRange)?;
    found
        .into_inner()
        .iter()
        .filter_map(|kind| match kind {
            FoundDateTimeKind::Normal(candidate) => Some(candidate),
            FoundDateTimeKind::Skipped { .. } => None,
        })
        .min_by_key(|candidate| candidate.unix_time())
        .ok_or(SolarError::DateOutOfRange)
        .and_then(|candidate| {
            let offset = FixedOffset::east_opt(candidate.local_time_type().ut_offset())
                .ok_or(SolarError::DateOutOfRange)?;
            Utc.timestamp_opt(candidate.unix_time(), candidate.nanoseconds())
                .single()
                .map(|utc| utc.with_timezone(&offset))
                .ok_or(SolarError::DateOutOfRange)
        })
}

/// Resolve a wall time through tz-rs' canonical local-time inverse.
fn resolve_local_datetime_in_timezone(
    datetime: NaiveDateTime,
    tz: &tz::TimeZone,
    timezone_label: &str,
) -> Result<DateTime<FixedOffset>, SolarError> {
    let found = TzDateTime::find(
        datetime.year(),
        datetime.month() as u8,
        datetime.day() as u8,
        datetime.hour() as u8,
        datetime.minute() as u8,
        datetime.second() as u8,
        datetime.nanosecond(),
        tz.as_ref(),
    )
    .map_err(|_| SolarError::DateOutOfRange)?;

    match found.into_inner().as_slice() {
        [FoundDateTimeKind::Normal(candidate)] => {
            let offset = FixedOffset::east_opt(candidate.local_time_type().ut_offset())
                .ok_or(SolarError::DateOutOfRange)?;
            Utc.timestamp_opt(candidate.unix_time(), candidate.nanoseconds())
                .single()
                .map(|utc| utc.with_timezone(&offset))
                .ok_or(SolarError::DateOutOfRange)
        }
        [FoundDateTimeKind::Skipped { .. }] => Err(SolarError::NonexistentLocalTime {
            datetime,
            timezone: timezone_label.to_owned(),
        }),
        [] => Err(SolarError::DateOutOfRange),
        _ => Err(SolarError::AmbiguousLocalTime {
            datetime,
            timezone: timezone_label.to_owned(),
        }),
    }
}

/// Retrieves all sun events for a given day safely, handling polar edge cases without erroring out.
pub fn safe_get_sun_events(
    date: NaiveDate,
    location: &Location,
) -> Result<SafeSunEvents, SolarError> {
    validate_location(location)?;

    let timezone_label = location.timezone_name.clone();
    let start_local = resolve_date_boundary(date, &location.timezone, &timezone_label)?;
    let next_date = date.succ_opt().ok_or(SolarError::DateOutOfRange)?;
    let end_local = resolve_date_boundary(next_date, &location.timezone, &timezone_label)?;

    let start_utc = start_local.with_timezone(&Utc);
    let end_utc = end_local.with_timezone(&Utc);

    let dawn = safe_find_event_crossing(
        date,
        location,
        CIVIL_TWILIGHT_ELEVATION_DEG,
        CrossingDirection::Rising,
        SunEventKind::Dawn,
    )?;
    let sunrise = safe_find_event_crossing(
        date,
        location,
        SUNRISE_SUNSET_ELEVATION_DEG,
        CrossingDirection::Rising,
        SunEventKind::Sunrise,
    )?;
    let noon_utc_dt = find_solar_noon(start_utc, end_utc, location)?;
    let noon = noon_utc(noon_utc_dt, location)?;
    let sunset = safe_find_event_crossing(
        date,
        location,
        SUNRISE_SUNSET_ELEVATION_DEG,
        CrossingDirection::Falling,
        SunEventKind::Sunset,
    )?;
    let dusk = safe_find_event_crossing(
        date,
        location,
        CIVIL_TWILIGHT_ELEVATION_DEG,
        CrossingDirection::Falling,
        SunEventKind::Dusk,
    )?;

    Ok(SafeSunEvents {
        dawn,
        sunrise,
        noon,
        sunset,
        dusk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn boundary_utc(zone: &str, y: i32, m: u32, d: u32) -> DateTime<Utc> {
        let location = Location::from_timezone_name(0.0, 0.0, zone).expect("timezone");
        let date = NaiveDate::from_ymd_opt(y, m, d).expect("date");
        resolve_date_boundary(date, &location.timezone, zone)
            .unwrap_or_else(|error| panic!("{zone} {date}: {error}"))
            .with_timezone(&Utc)
    }

    #[test]
    fn day_boundary_survives_dst_that_skips_local_midnight() {
        // Cairo, Santiago and Havana jump 00:00 -> 01:00: the day starts at 01:00.
        let utc = |y, m, d, h| Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap();
        assert_eq!(
            boundary_utc("Africa/Cairo", 2025, 4, 25),
            utc(2025, 4, 24, 22)
        );
        assert_eq!(
            boundary_utc("America/Santiago", 2025, 9, 7),
            utc(2025, 9, 7, 4)
        );
        assert_eq!(
            boundary_utc("America/Havana", 2025, 3, 9),
            utc(2025, 3, 9, 5)
        );
        // Beirut also skips midnight at its spring transition.
        let beirut = boundary_utc("Asia/Beirut", 2025, 3, 30);
        assert_eq!(beirut, utc(2025, 3, 29, 22));
    }

    #[test]
    fn day_boundary_takes_the_earliest_of_a_repeated_midnight() {
        // Havana falls back 01:00 -> 00:00 on 2025-11-02: 00:00 happens twice.
        let first = Utc.with_ymd_and_hms(2025, 11, 2, 4, 0, 0).unwrap();
        assert_eq!(boundary_utc("America/Havana", 2025, 11, 2), first);
    }

    #[test]
    fn strict_wall_time_resolution_still_rejects_nonexistent_times() {
        let cairo = Location::from_timezone_name(30.0, 31.0, "Africa/Cairo").unwrap();
        let skipped = NaiveDate::from_ymd_opt(2025, 4, 25)
            .unwrap()
            .and_hms_opt(0, 30, 0)
            .unwrap();
        assert!(matches!(
            local_datetime(skipped, &cairo),
            Err(SolarError::NonexistentLocalTime { .. })
        ));
    }

    #[test]
    fn sun_events_resolve_on_a_day_whose_midnight_is_skipped() {
        let cairo = Location::from_timezone_name(30.0444, 31.2357, "Africa/Cairo").unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 4, 25).unwrap();
        let events = safe_get_sun_events(date, &cairo).expect("day resolves");
        assert!(events.sunrise.is_some() && events.sunset.is_some());
    }

    #[test]
    fn sun_events_are_ordered_for_mid_latitudes() {
        let location = Location::from_timezone_name(41.0082, 28.9784, "Europe/Istanbul")
            .expect("timezone should parse");
        let date = NaiveDate::from_ymd_opt(2025, 6, 21).expect("date should be valid");

        let events = get_sun_events(date, &location).expect("sun events should be available");

        assert!(events.dawn < events.sunrise);
        assert!(events.sunrise < events.noon);
        assert!(events.noon < events.sunset);
        assert!(events.sunset < events.dusk);
        assert_eq!(events.sunrise.date_naive(), date);
        assert_eq!(events.sunset.date_naive(), date);
    }
    #[test]
    fn solar_elevation_uses_explicit_timezone() {
        let new_york = Location::from_timezone_name(40.7128, -74.0060, "America/New_York")
            .expect("timezone should parse");
        let utc =
            Location::from_timezone_name(40.7128, -74.0060, "UTC").expect("timezone should parse");

        let ny_local_noon = NaiveDate::from_ymd_opt(2024, 6, 21)
            .expect("date should be valid")
            .and_hms_opt(12, 0, 0)
            .expect("time should be valid");
        let same_instant_utc = NaiveDate::from_ymd_opt(2024, 6, 21)
            .expect("date should be valid")
            .and_hms_opt(16, 0, 0)
            .expect("time should be valid");

        let ny_elevation =
            get_solar_elevation(ny_local_noon, &new_york).expect("elevation should resolve");
        let utc_elevation =
            get_solar_elevation(same_instant_utc, &utc).expect("elevation should resolve");

        assert!((ny_elevation - utc_elevation).abs() < 0.000_001);

        let events = get_sun_events(
            NaiveDate::from_ymd_opt(2024, 6, 21).expect("date should be valid"),
            &new_york,
        )
        .expect("sun events should resolve");
        assert_eq!(events.sunrise.offset().local_minus_utc(), -4 * 60 * 60);
    }
    #[test]
    fn sample_at_utc_classifies_phase_from_elevation_thresholds() {
        let location =
            Location::from_timezone_name(0.0, 0.0, "UTC").expect("timezone should parse");
        let noon = Utc
            .with_ymd_and_hms(2024, 3, 20, 12, 0, 0)
            .single()
            .expect("UTC datetime should be valid");

        let sample = sample_at_utc(noon, &location, -6.0, 3.0).expect("sample should resolve");
        assert_eq!(sample.phase, SolarPhase::Day);
        assert!(sample.elevation_deg > 80.0);
    }

    #[test]
    fn local_datetime_rejects_dst_gap_and_fold() {
        let berlin = Location::from_timezone_name(52.52, 13.405, "Europe/Berlin")
            .expect("timezone should parse");
        let gap = NaiveDate::from_ymd_opt(2024, 3, 31)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let fold = NaiveDate::from_ymd_opt(2024, 10, 27)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        assert!(matches!(
            local_datetime(gap, &berlin),
            Err(SolarError::NonexistentLocalTime { .. })
        ));
        assert!(matches!(
            local_datetime(fold, &berlin),
            Err(SolarError::AmbiguousLocalTime { .. })
        ));
    }

    #[test]
    fn local_datetime_resolves_ordinary_dst_wall_time() {
        let new_york = Location::from_timezone_name(40.7128, -74.006, "America/New_York")
            .expect("timezone should parse");
        let ordinary = NaiveDate::from_ymd_opt(2024, 6, 21)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let resolved = local_datetime(ordinary, &new_york).expect("ordinary time should resolve");
        assert_eq!(resolved.offset().local_minus_utc(), -4 * 60 * 60);
    }

    #[test]
    fn local_datetime_rejects_new_york_gap_and_fold_and_utc_is_unique() {
        let new_york = Location::from_timezone_name(40.7128, -74.006, "America/New_York")
            .expect("timezone should parse");
        let gap = NaiveDate::from_ymd_opt(2024, 3, 10)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let fold = NaiveDate::from_ymd_opt(2024, 11, 3)
            .unwrap()
            .and_hms_opt(1, 30, 0)
            .unwrap();
        assert!(matches!(
            local_datetime(gap, &new_york),
            Err(SolarError::NonexistentLocalTime { .. })
        ));
        assert!(matches!(
            local_datetime(fold, &new_york),
            Err(SolarError::AmbiguousLocalTime { .. })
        ));

        let utc = Location::from_timezone_name(0.0, 0.0, "UTC").expect("UTC should parse");
        let ordinary = NaiveDate::from_ymd_opt(2024, 3, 10)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        assert_eq!(
            local_datetime(ordinary, &utc)
                .expect("UTC local time should be unique")
                .offset()
                .local_minus_utc(),
            0
        );
    }

    #[test]
    fn date_boundary_uses_canonical_unique_midnight_resolution() {
        let berlin = Location::from_timezone_name(52.52, 13.405, "Europe/Berlin")
            .expect("timezone should parse");
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let boundary = resolve_date_boundary(date, &berlin.timezone, &berlin.timezone_name)
            .expect("ordinary midnight should resolve");
        assert_eq!(boundary.date_naive(), date);
        assert_eq!(boundary.time().hour(), 0);
    }

    #[test]
    fn lord_howe_half_hour_transition_is_classified_without_hour_assumptions() {
        let lord_howe = Location::from_timezone_name(-31.555, 159.08, "Australia/Lord_Howe")
            .expect("Lord Howe timezone should parse from installed tzdata");
        let spring_gap = NaiveDate::from_ymd_opt(2024, 10, 6)
            .unwrap()
            .and_hms_opt(2, 15, 0)
            .unwrap();
        let autumn_fold = NaiveDate::from_ymd_opt(2024, 4, 7)
            .unwrap()
            .and_hms_opt(1, 45, 0)
            .unwrap();

        assert!(matches!(
            local_datetime(spring_gap, &lord_howe),
            Err(SolarError::NonexistentLocalTime { .. })
        ));
        assert!(matches!(
            local_datetime(autumn_fold, &lord_howe),
            Err(SolarError::AmbiguousLocalTime { .. })
        ));
    }

    #[test]
    fn solar_position_matches_published_nrel_spa_example_with_model_tolerance() {
        // NREL SPA example vector: 2003-10-17 12:30:30 local time,
        // 39.742_476 N, 105.1786 W, UTC-07:00. The published SPA result is
        // apparent zenith 50.111_622 degrees; SunReactor compares geometric
        // elevation and intentionally does not model SPA atmospheric refraction.
        let location = Location::from_timezone_name(39.742_476, -105.1786, "UTC")
            .expect("UTC timezone should parse");
        let datetime = Utc
            .with_ymd_and_hms(2003, 10, 17, 19, 30, 30)
            .single()
            .expect("SPA example timestamp should be valid");
        let elevation = get_solar_elevation(datetime.naive_utc(), &location)
            .expect("solar elevation should resolve");
        let reference_geometric_elevation = 90.0 - 50.111_622;

        assert!(
            (elevation - reference_geometric_elevation).abs() < 0.05,
            "SunReactor elevation {elevation:.6} differs from the published SPA vector"
        );
    }
}
