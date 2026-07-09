use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use std::sync::Arc;
use tz::TimeZone;

/// A geographic location and explicit resolved IANA timezone.
///
/// The solar module never guesses the local timezone from the host
/// environment. Callers resolve the timezone once and then reuse this typed
/// location for solar and policy calculations.
#[derive(Debug, Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: Arc<TimeZone>,
    pub timezone_name: String,
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.latitude == other.latitude
            && self.longitude == other.longitude
            && self.timezone_name == other.timezone_name
    }
}

impl Location {
    pub fn from_timezone_name(
        latitude: f64,
        longitude: f64,
        timezone_name: &str,
    ) -> Result<Self, SolarError> {
        let timezone_name = timezone_name.trim();
        let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone_name);

        // Use dynamically parsed OS timezone data to prevent static tzdata drift, fallback to POSIX
        let tz = if let Ok(data) = std::fs::read(&path) {
            TimeZone::from_tz_data(&data).map_err(|_| SolarError::InvalidTimezone {
                timezone: timezone_name.to_owned(),
            })?
        } else {
            TimeZone::from_posix_tz(timezone_name).map_err(|_| SolarError::InvalidTimezone {
                timezone: timezone_name.to_owned(),
            })?
        };

        Ok(Self {
            latitude,
            longitude,
            timezone: Arc::new(tz),
            timezone_name: timezone_name.to_owned(),
        })
    }
}
/// Daily solar events localized to `Location::timezone`.
///
/// The returned datetimes carry a fixed UTC offset so callers can use them
/// without re-resolving the timezone. Around DST transitions, different events
/// on the same local date may legitimately carry different offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SunEvents {
    pub dawn: DateTime<FixedOffset>,
    pub sunrise: DateTime<FixedOffset>,
    pub noon: DateTime<FixedOffset>,
    pub sunset: DateTime<FixedOffset>,
    pub dusk: DateTime<FixedOffset>,
}
/// Named solar events used in structured event-search errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunEventKind {
    Dawn,
    Sunrise,
    SolarNoon,
    Sunset,
    Dusk,
}
/// A coarse solar phase used by the brightness policy engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolarPhase {
    Day,
    Transition,
    Night,
}
/// A solar sample suitable for policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarSample {
    pub phase: SolarPhase,
    pub elevation_deg: f32,
}

impl Default for SolarSample {
    fn default() -> Self {
        Self {
            phase: SolarPhase::Day,
            elevation_deg: 0.0,
        }
    }
}
/// Structured solar calculation errors.
///
/// `get_solar_elevation()` can fail on invalid coordinates, invalid timezones,
/// or ambiguous/nonexistent local datetimes around timezone transitions.
///
/// `get_sun_events()` can additionally fail at extreme latitudes when the sun
/// never crosses the event threshold on the requested local date. This is how
/// sunreactor surfaces polar day/night edge cases instead of guessing.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SolarError {
    #[error("latitude must be within -90..=90, got {latitude}")]
    InvalidLatitude { latitude: f64 },
    #[error("longitude must be within -180..=180, got {longitude}")]
    InvalidLongitude { longitude: f64 },
    #[error("date is out of range")]
    DateOutOfRange,
    #[error("invalid IANA timezone `{timezone}`")]
    InvalidTimezone { timezone: String },
    #[error("local datetime {datetime} is ambiguous in timezone {timezone}")]
    AmbiguousLocalTime {
        datetime: NaiveDateTime,
        timezone: String,
    },
    #[error("local datetime {datetime} does not exist in timezone {timezone}")]
    NonexistentLocalTime {
        datetime: NaiveDateTime,
        timezone: String,
    },
    #[error("{event} is unavailable on {date} because the sun never crosses {threshold_deg:.3} deg (daily range {min_elevation_deg:.3}..={max_elevation_deg:.3} deg)")]
    SunNeverCrossesThreshold {
        date: NaiveDate,
        event: SunEventKind,
        threshold_deg: f64,
        min_elevation_deg: f64,
        max_elevation_deg: f64,
    },
}
impl std::fmt::Display for SunEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dawn => write!(f, "dawn"),
            Self::Sunrise => write!(f, "sunrise"),
            Self::SolarNoon => write!(f, "solar noon"),
            Self::Sunset => write!(f, "sunset"),
            Self::Dusk => write!(f, "dusk"),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossingDirection {
    Rising,
    Falling,
}

impl CrossingDirection {
    pub(crate) fn crossed(self, previous: f64, next: f64, threshold: f64) -> bool {
        match self {
            Self::Rising => {
                previous <= threshold
                    && next >= threshold
                    && (previous < threshold || next > threshold)
            }
            Self::Falling => {
                previous >= threshold
                    && next <= threshold
                    && (previous > threshold || next < threshold)
            }
        }
    }
}
/// A polar-safe representation of solar events where sunrise/sunset might not occur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeSunEvents {
    pub dawn: Option<DateTime<FixedOffset>>,
    pub sunrise: Option<DateTime<FixedOffset>>,
    pub noon: DateTime<FixedOffset>,
    pub sunset: Option<DateTime<FixedOffset>>,
    pub dusk: Option<DateTime<FixedOffset>>,
}
