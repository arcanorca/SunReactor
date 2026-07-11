pub(crate) mod ephemeris;
pub(crate) mod search;
pub mod types;

pub use ephemeris::calculate_lunar_phase;
pub use ephemeris::LunarPhase;
pub use types::*;

use crate::solar::ephemeris::solar_elevation_utc;
use crate::solar::search::{find_event_crossing, find_solar_noon, safe_find_event_crossing};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use std::cell::RefCell;

const CIVIL_TWILIGHT_ELEVATION_DEG: f64 = -6.0;
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
        dawn: local_datetime_at_utc(dawn, location).unwrap(),
        sunrise: local_datetime_at_utc(sunrise, location).unwrap(),
        noon: local_datetime_at_utc(noon, location).unwrap(),
        sunset: local_datetime_at_utc(sunset, location).unwrap(),
        dusk: local_datetime_at_utc(dusk, location).unwrap(),
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
    let offset = FixedOffset::east_opt(local_type.ut_offset()).unwrap();
    Ok(datetime.with_timezone(&offset))
}

fn noon_utc(datetime: DateTime<Utc>, location: &Location) -> DateTime<FixedOffset> {
    let local_type = location
        .timezone
        .find_local_time_type(datetime.timestamp())
        .unwrap();
    let offset = FixedOffset::east_opt(local_type.ut_offset()).unwrap();
    datetime.with_timezone(&offset)
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

    let fake_utc_ts = datetime.and_utc().timestamp();
    let tz = &location.timezone;

    let local_type_1 = tz
        .find_local_time_type(fake_utc_ts)
        .map_err(|_| SolarError::DateOutOfRange)?;
    let approx_utc_ts = fake_utc_ts - local_type_1.ut_offset() as i64;

    let local_type_2 = tz
        .find_local_time_type(approx_utc_ts)
        .map_err(|_| SolarError::DateOutOfRange)?;
    let offset = FixedOffset::east_opt(local_type_2.ut_offset()).unwrap();

    Ok(datetime.and_local_timezone(offset).unwrap())
}

pub(crate) fn resolve_date_boundary(
    date: NaiveDate,
    tz: &tz::TimeZone,
    timezone_label: &str,
) -> Result<DateTime<FixedOffset>, SolarError> {
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(SolarError::DateOutOfRange)?;

    let fake_utc_ts = midnight.and_utc().timestamp();

    let local_type_1 =
        tz.find_local_time_type(fake_utc_ts)
            .map_err(|_| SolarError::NonexistentLocalTime {
                datetime: midnight,
                timezone: timezone_label.to_owned(),
            })?;
    let approx_utc_ts = fake_utc_ts - local_type_1.ut_offset() as i64;

    let local_type_2 =
        tz.find_local_time_type(approx_utc_ts)
            .map_err(|_| SolarError::NonexistentLocalTime {
                datetime: midnight,
                timezone: timezone_label.to_owned(),
            })?;
    let offset = FixedOffset::east_opt(local_type_2.ut_offset()).unwrap();

    Ok(midnight.and_local_timezone(offset).unwrap())
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
    let noon = noon_utc(noon_utc_dt, location);
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
}
