use crate::solar::ephemeris::solar_elevation_utc;
use crate::solar::types::{CrossingDirection, Location, SolarError, SunEventKind};
use crate::solar::{
    local_datetime_at_utc, resolve_date_boundary, validate_location, COARSE_SEARCH_STEP_MINUTES,
    FINE_SEARCH_STEP_MINUTES, NOON_FINAL_WINDOW_SECONDS, NOON_FINE_WINDOW_MINUTES,
};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeZone, Utc};

pub(crate) fn find_solar_noon(
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    location: &Location,
) -> Result<DateTime<Utc>, SolarError> {
    validate_location(location)?;

    let best_coarse = scan_maximum(
        start_utc,
        end_utc,
        Duration::minutes(COARSE_SEARCH_STEP_MINUTES),
        location,
    );
    let best_fine = scan_maximum(
        clamp_utc(
            best_coarse - Duration::minutes(NOON_FINE_WINDOW_MINUTES),
            start_utc,
            end_utc,
        ),
        clamp_utc(
            best_coarse + Duration::minutes(NOON_FINE_WINDOW_MINUTES),
            start_utc,
            end_utc,
        ),
        Duration::minutes(FINE_SEARCH_STEP_MINUTES),
        location,
    );

    Ok(scan_maximum(
        clamp_utc(
            best_fine - Duration::seconds(NOON_FINAL_WINDOW_SECONDS),
            start_utc,
            end_utc,
        ),
        clamp_utc(
            best_fine + Duration::seconds(NOON_FINAL_WINDOW_SECONDS),
            start_utc,
            end_utc,
        ),
        Duration::seconds(1),
        location,
    ))
}

pub(crate) fn scan_maximum(
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    step: Duration,
    location: &Location,
) -> DateTime<Utc> {
    let mut best_time = start_utc;
    let mut best_elevation = solar_elevation_utc(start_utc, location.latitude, location.longitude);
    let mut time = start_utc;

    while time < end_utc {
        let next = clamp_utc(time + step, start_utc, end_utc);
        let elevation = solar_elevation_utc(next, location.latitude, location.longitude);
        if elevation > best_elevation {
            best_elevation = elevation;
            best_time = next;
        }
        time = next;
    }

    best_time
}

pub(crate) fn find_event_crossing(
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    location: &Location,
    threshold_deg: f64,
    direction: CrossingDirection,
    date: NaiveDate,
    event: SunEventKind,
) -> Result<DateTime<Utc>, SolarError> {
    validate_location(location)?;

    let mut min_elevation = f64::INFINITY;
    let mut max_elevation = f64::NEG_INFINITY;
    let mut previous_time = start_utc;
    let mut previous_elevation =
        solar_elevation_utc(previous_time, location.latitude, location.longitude);
    min_elevation = min_elevation.min(previous_elevation);
    max_elevation = max_elevation.max(previous_elevation);

    while previous_time < end_utc {
        let next_time = clamp_utc(
            previous_time + Duration::minutes(COARSE_SEARCH_STEP_MINUTES),
            start_utc,
            end_utc,
        );
        let next_elevation = solar_elevation_utc(next_time, location.latitude, location.longitude);
        min_elevation = min_elevation.min(next_elevation);
        max_elevation = max_elevation.max(next_elevation);

        if direction.crossed(previous_elevation, next_elevation, threshold_deg) {
            return Ok(refine_crossing(
                previous_time,
                next_time,
                threshold_deg,
                direction,
                location,
            ));
        }

        previous_time = next_time;
        previous_elevation = next_elevation;
    }

    Err(SolarError::SunNeverCrossesThreshold {
        date,
        event,
        threshold_deg,
        min_elevation_deg: min_elevation,
        max_elevation_deg: max_elevation,
    })
}

pub(crate) fn refine_crossing(
    low: DateTime<Utc>,
    high: DateTime<Utc>,
    threshold_deg: f64,
    direction: CrossingDirection,
    location: &Location,
) -> DateTime<Utc> {
    let mut low_ts = low.timestamp();
    let mut high_ts = high.timestamp();

    while high_ts - low_ts > 1 {
        let mid_ts = low_ts + (high_ts - low_ts) / 2;
        let mid = Utc
            .timestamp_opt(mid_ts, 0)
            .single()
            .unwrap_or_else(Utc::now); // Infallible inside loop bounding
        let elevation = solar_elevation_utc(mid, location.latitude, location.longitude);

        match direction {
            CrossingDirection::Rising => {
                if elevation >= threshold_deg {
                    high_ts = mid_ts;
                } else {
                    low_ts = mid_ts;
                }
            }
            CrossingDirection::Falling => {
                if elevation <= threshold_deg {
                    high_ts = mid_ts;
                } else {
                    low_ts = mid_ts;
                }
            }
        }
    }

    let low_dt = Utc
        .timestamp_opt(low_ts, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let high_dt = Utc
        .timestamp_opt(high_ts, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let low_error =
        (solar_elevation_utc(low_dt, location.latitude, location.longitude) - threshold_deg).abs();
    let high_error =
        (solar_elevation_utc(high_dt, location.latitude, location.longitude) - threshold_deg).abs();

    if high_error < low_error {
        high_dt
    } else {
        low_dt
    }
}

pub(crate) fn clamp_utc(
    value: DateTime<Utc>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> DateTime<Utc> {
    if value < start {
        start
    } else if value > end {
        end
    } else {
        value
    }
}

/// A safe, polar-aware wrapper for retrieving a specific solar event crossing.
/// Returns `Ok(None)` if the sun never crosses the threshold (e.g., Polar Night or Midnight Sun).
pub(crate) fn safe_find_event_crossing(
    date: NaiveDate,
    location: &Location,
    threshold_deg: f64,
    direction: CrossingDirection,
    event: SunEventKind,
) -> Result<Option<DateTime<FixedOffset>>, SolarError> {
    let timezone_label = location.timezone_name.clone();
    let start_local = resolve_date_boundary(date, &location.timezone, &timezone_label)?;
    let next_date = date.succ_opt().ok_or(SolarError::DateOutOfRange)?;
    let end_local = resolve_date_boundary(next_date, &location.timezone, &timezone_label)?;

    let start_utc = start_local.with_timezone(&Utc);
    let end_utc = end_local.with_timezone(&Utc);

    match find_event_crossing(
        start_utc,
        end_utc,
        location,
        threshold_deg,
        direction,
        date,
        event,
    ) {
        Ok(dt) => Ok(Some(local_datetime_at_utc(dt, location)?)),
        Err(SolarError::SunNeverCrossesThreshold { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solar::get_sun_events;
    use chrono::NaiveDate;
    #[test]
    fn polar_day_night_edge_cases_are_structured_errors() {
        let location =
            Location::from_timezone_name(82.0, 15.0, "UTC").expect("timezone should parse");
        let date = NaiveDate::from_ymd_opt(2024, 12, 21).expect("date should be valid");

        let error = get_sun_events(date, &location).expect_err("polar night should remove events");
        assert!(matches!(error, SolarError::SunNeverCrossesThreshold { .. }));
    }
}
