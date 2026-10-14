//! Pure preview sampling of the brightness policy: current targets, a sampled
//! curve over a time window, the daily milestone schedule, and the full
//! local-day curve served by the daemon's `day_curve` IPC request.
//!
//! The TUI and the daemon share these functions so the widget, the TUI and
//! the applied brightness never disagree about the policy. Everything here
//! is side-effect free (no I/O, no clock reads).

use chrono::{DateTime, Duration, NaiveDate, Utc};

use crate::config::{MonitorConfig, SolarPolicyConfig};
use crate::policy::{
    compute_monitor_milestones, compute_policy, MonitorMilestone, MonitorMilestoneSchedule,
    PerMonitorTarget, PolicyContext, PolicyError,
};
use crate::solar::{self, Location, SolarError};

/// Sampling step of [`day_curve`] as served over IPC.
pub const DAY_CURVE_STEP_MINUTES: u32 = 10;
/// Upper bound on samples in one curve (25 h day at 1 min); keeps requests bounded.
pub const MAX_CURVE_SAMPLES: usize = 25 * 60 + 1;

/// Policy inputs shared by every preview function.
#[derive(Debug, Clone, Copy)]
pub struct PreviewInputs<'a> {
    pub location: &'a Location,
    pub config: &'a SolarPolicyConfig,
    pub monitors: &'a [MonitorConfig],
}

impl<'a> PreviewInputs<'a> {
    fn context(&self, now_utc: DateTime<Utc>, weather: Option<f64>) -> PolicyContext<'a> {
        PolicyContext {
            now_utc,
            location: self.location,
            config: self.config,
            monitors: self.monitors,
            weather_multiplier: weather,
        }
    }

    /// Per-monitor targets at one instant.
    pub fn targets_at(
        &self,
        at_utc: DateTime<Utc>,
        weather_multiplier: Option<f64>,
    ) -> Result<Vec<PerMonitorTarget>, PolicyError> {
        Ok(compute_policy(&self.context(at_utc, weather_multiplier))?.targets)
    }

    /// Samples the policy every `step_minutes` starting at `start_utc`.
    /// Returns one `Vec` of percentages per monitor, in monitor order.
    pub fn sample_curves(
        &self,
        start_utc: DateTime<Utc>,
        step_minutes: u32,
        sample_count: usize,
        weather_multiplier: Option<f64>,
    ) -> Result<Vec<Vec<u8>>, PolicyError> {
        let sample_count = sample_count.min(MAX_CURVE_SAMPLES);
        let mut curves = vec![Vec::with_capacity(sample_count); self.monitors.len()];
        for index in 0..sample_count {
            let at = start_utc + Duration::minutes(i64::from(step_minutes) * index as i64);
            for (curve, target) in curves
                .iter_mut()
                .zip(self.targets_at(at, weather_multiplier)?)
            {
                curve.push(target.percent);
            }
        }
        Ok(curves)
    }

    /// Daily milestone schedule for the automation cycle containing `now_utc`
    /// (weather excluded, as the schedule is a stable preview).
    pub fn milestones(
        &self,
        now_utc: DateTime<Utc>,
    ) -> Result<Vec<MonitorMilestoneSchedule>, PolicyError> {
        compute_monitor_milestones(&self.context(now_utc, None))
    }
}

/// Start/end instants of one local calendar day in the configured timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDay {
    pub date: NaiveDate,
    /// Local midnight starting `date`.
    pub start_utc: DateTime<Utc>,
    /// Local midnight starting the next day (23 h / 25 h away on DST days).
    pub end_utc: DateTime<Utc>,
}

/// Resolves the local day containing `now_utc`. A nonexistent or ambiguous
/// local midnight is a structured error, never a guessed boundary.
pub fn local_day(now_utc: DateTime<Utc>, location: &Location) -> Result<LocalDay, SolarError> {
    let date = solar::local_datetime_at_utc(now_utc, location)?.date_naive();
    let next = date.succ_opt().ok_or(SolarError::DateOutOfRange)?;
    let label = location.timezone_name.as_str();
    let start = solar::resolve_date_boundary(date, &location.timezone, label)?;
    let end = solar::resolve_date_boundary(next, &location.timezone, label)?;
    Ok(LocalDay {
        date,
        start_utc: start.with_timezone(&Utc),
        end_utc: end.with_timezone(&Utc),
    })
}

/// Sun events of a local day; `None` where the sun never crosses the
/// threshold (polar day/night).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaySun {
    pub dawn: Option<DateTime<Utc>>,
    pub sunrise: Option<DateTime<Utc>>,
    pub solar_noon: Option<DateTime<Utc>>,
    pub sunset: Option<DateTime<Utc>>,
    pub dusk: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayCurvePoint {
    pub at_utc: DateTime<Utc>,
    /// Target with the weather multiplier applied (equals `percent_no_weather`
    /// when no multiplier is active).
    pub percent: u8,
    pub percent_no_weather: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorDayCurve {
    pub logical_id: String,
    pub min_pct: u8,
    pub max_pct: u8,
    pub samples: Vec<DayCurvePoint>,
    pub milestones: Vec<MonitorMilestone>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayCurve {
    pub date: NaiveDate,
    pub step_minutes: u32,
    pub weather_multiplier: Option<f64>,
    pub sun: DaySun,
    pub monitors: Vec<MonitorDayCurve>,
    /// Set when the milestone schedule could not be built (e.g. polar day or
    /// night); the curve samples are still valid.
    pub milestones_error: Option<PolicyError>,
}

/// Today's policy curve (local midnight to local midnight, both included)
/// for every monitor in `inputs`, with and without the weather multiplier.
pub fn day_curve(
    inputs: &PreviewInputs<'_>,
    now_utc: DateTime<Utc>,
    step_minutes: u32,
    weather_multiplier: Option<f64>,
) -> Result<DayCurve, PolicyError> {
    let step_minutes = step_minutes.max(1);
    let day = local_day(now_utc, inputs.location)?;
    let events = solar::safe_get_sun_events(day.date, inputs.location)?;
    let sun = DaySun {
        dawn: events.dawn.map(|t| t.with_timezone(&Utc)),
        sunrise: events.sunrise.map(|t| t.with_timezone(&Utc)),
        solar_noon: Some(events.noon.with_timezone(&Utc)),
        sunset: events.sunset.map(|t| t.with_timezone(&Utc)),
        dusk: events.dusk.map(|t| t.with_timezone(&Utc)),
    };

    let span_minutes = (day.end_utc - day.start_utc).num_minutes().max(0);
    let sample_count = (span_minutes / i64::from(step_minutes)) as usize + 1;
    let no_weather = inputs.sample_curves(day.start_utc, step_minutes, sample_count, None)?;
    let with_weather = match weather_multiplier {
        Some(multiplier) => {
            inputs.sample_curves(day.start_utc, step_minutes, sample_count, Some(multiplier))?
        }
        None => no_weather.clone(),
    };

    // Milestones for the cycle containing solar noon, i.e. today's cycle
    // even when the request arrives before sunrise.
    let reference = sun
        .solar_noon
        .unwrap_or(day.start_utc + Duration::hours(12));
    let (schedules, milestones_error) = match inputs.milestones(reference) {
        Ok(schedules) => (schedules, None),
        Err(error) => (Vec::new(), Some(error)),
    };

    let monitors = inputs
        .monitors
        .iter()
        .zip(no_weather.iter().zip(&with_weather))
        .map(|(monitor, (base, weather))| MonitorDayCurve {
            logical_id: monitor.logical_id.clone(),
            min_pct: monitor.min_pct,
            max_pct: monitor.max_pct,
            samples: base
                .iter()
                .zip(weather)
                .enumerate()
                .map(|(index, (base, weather))| DayCurvePoint {
                    at_utc: day.start_utc
                        + Duration::minutes(i64::from(step_minutes) * index as i64),
                    percent: *weather,
                    percent_no_weather: *base,
                })
                .collect(),
            milestones: schedules
                .iter()
                .find(|schedule| schedule.logical_id == monitor.logical_id)
                .map(|schedule| schedule.milestones.clone())
                .unwrap_or_default(),
        })
        .collect();

    Ok(DayCurve {
        date: day.date,
        step_minutes,
        weather_multiplier,
        sun,
        monitors,
        milestones_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn monitor(id: &str, min_pct: u8, max_pct: u8) -> MonitorConfig {
        MonitorConfig {
            logical_id: id.to_owned(),
            min_pct,
            max_pct,
            ..MonitorConfig::default()
        }
    }

    fn curve_at(
        lat: f64,
        lon: f64,
        tz: &str,
        now: DateTime<Utc>,
        weather: Option<f64>,
    ) -> Result<DayCurve, PolicyError> {
        let location = Location::from_timezone_name(lat, lon, tz).unwrap();
        let config = SolarPolicyConfig::default();
        let monitors = vec![monitor("mi-monitor", 1, 50), monitor("laptop", 10, 90)];
        let inputs = PreviewInputs {
            location: &location,
            config: &config,
            monitors: &monitors,
        };
        day_curve(&inputs, now, DAY_CURVE_STEP_MINUTES, weather)
    }

    #[test]
    fn day_curve_covers_local_midnight_to_midnight_within_bounds() {
        let now = Utc.with_ymd_and_hms(2026, 10, 13, 9, 30, 0).unwrap();
        let curve = curve_at(41.0082, 28.9784, "Europe/Istanbul", now, Some(0.8)).unwrap();
        assert_eq!(curve.date, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap());
        assert_eq!(curve.monitors.len(), 2);
        let istanbul_midnight = Utc.with_ymd_and_hms(2026, 10, 12, 21, 0, 0).unwrap();
        for monitor in &curve.monitors {
            assert_eq!(monitor.samples.len(), 24 * 6 + 1);
            assert_eq!(monitor.samples[0].at_utc, istanbul_midnight);
            assert_eq!(
                monitor.samples.last().unwrap().at_utc,
                istanbul_midnight + Duration::hours(24)
            );
            assert!(monitor
                .samples
                .windows(2)
                .all(|w| (w[1].at_utc - w[0].at_utc).num_minutes() == 10));
            for sample in &monitor.samples {
                assert!((monitor.min_pct..=monitor.max_pct).contains(&sample.percent));
                assert!((monitor.min_pct..=monitor.max_pct).contains(&sample.percent_no_weather));
                assert!(sample.percent <= sample.percent_no_weather);
            }
            assert!(
                monitor
                    .samples
                    .iter()
                    .any(|s| s.percent < s.percent_no_weather),
                "weather must dim some daytime samples"
            );
            assert_eq!(monitor.milestones.len(), 9);
            assert!(monitor
                .milestones
                .windows(2)
                .all(|w| w[0].adjusted_time_local < w[1].adjusted_time_local));
        }
        let sun = curve.sun;
        assert!(sun.dawn < sun.sunrise && sun.sunrise < sun.solar_noon);
        assert!(sun.solar_noon < sun.sunset && sun.sunset < sun.dusk);
        assert!(curve.milestones_error.is_none());
    }

    #[test]
    fn day_curve_without_weather_is_identical_and_milestones_are_todays_before_sunrise() {
        let early = Utc.with_ymd_and_hms(2026, 10, 13, 0, 30, 0).unwrap(); // 03:30 local
        let curve = curve_at(41.0082, 28.9784, "Europe/Istanbul", early, None).unwrap();
        assert_eq!(curve.weather_multiplier, None);
        for monitor in &curve.monitors {
            assert!(monitor
                .samples
                .iter()
                .all(|s| s.percent == s.percent_no_weather));
            let rise = monitor.milestones.first().unwrap();
            assert_eq!(rise.adjusted_time_local.date_naive(), curve.date);
        }
    }

    #[test]
    fn dst_days_have_23_and_25_hours_of_samples() {
        // Europe/Berlin: 2026-03-29 spring forward, 2026-10-25 fall back.
        let spring = Utc.with_ymd_and_hms(2026, 3, 29, 12, 0, 0).unwrap();
        let curve = curve_at(52.52, 13.405, "Europe/Berlin", spring, None).unwrap();
        assert_eq!(curve.monitors[0].samples.len(), 23 * 6 + 1);

        let autumn = Utc.with_ymd_and_hms(2026, 10, 25, 12, 0, 0).unwrap();
        let curve = curve_at(52.52, 13.405, "Europe/Berlin", autumn, None).unwrap();
        assert_eq!(curve.monitors[0].samples.len(), 25 * 6 + 1);
    }

    #[test]
    fn polar_night_yields_null_sun_events_not_invented_times() {
        // Tromsø, mid-December: no sunrise/sunset.
        let now = Utc.with_ymd_and_hms(2026, 12, 15, 11, 0, 0).unwrap();
        let curve = curve_at(69.6492, 18.9553, "Europe/Oslo", now, None).unwrap();
        assert!(curve.sun.sunrise.is_none());
        assert!(curve.sun.sunset.is_none());
        assert!(curve.sun.solar_noon.is_some());
        for monitor in &curve.monitors {
            assert!(monitor
                .samples
                .iter()
                .all(|s| (monitor.min_pct..=monitor.max_pct).contains(&s.percent)));
        }
    }

    #[test]
    fn sample_curves_are_bounded() {
        let location = Location::from_timezone_name(0.0, 0.0, "UTC").unwrap();
        let config = SolarPolicyConfig::default();
        let monitors = vec![monitor("m", 5, 60)];
        let inputs = PreviewInputs {
            location: &location,
            config: &config,
            monitors: &monitors,
        };
        let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let curves = inputs
            .sample_curves(start, 1, usize::MAX, None)
            .unwrap_or_default();
        assert!(curves.iter().all(|c| c.len() <= MAX_CURVE_SAMPLES));
    }

    #[test]
    fn milestone_adjustments_shift_the_curve_and_milestones() {
        use crate::config::MonitorMilestoneAdjustment;
        use crate::policy::AutomationMilestone;

        let location = Location::from_timezone_name(41.0082, 28.9784, "Europe/Istanbul").unwrap();
        let config = SolarPolicyConfig::default();
        let plain = vec![monitor("m", 5, 60)];
        let mut adjusted = plain.clone();
        adjusted[0].milestone_adjustments = vec![MonitorMilestoneAdjustment {
            milestone: AutomationMilestone::Rise50,
            minutes_offset: 45,
        }];
        let now = Utc.with_ymd_and_hms(2026, 10, 13, 9, 30, 0).unwrap();
        let curve = |monitors: &[MonitorConfig]| {
            let inputs = PreviewInputs {
                location: &location,
                config: &config,
                monitors,
            };
            day_curve(&inputs, now, DAY_CURVE_STEP_MINUTES, Some(0.9)).unwrap()
        };
        let base = curve(&plain);
        let shifted = curve(&adjusted);
        let rise50 = |c: &DayCurve| {
            c.monitors[0]
                .milestones
                .iter()
                .find(|m| m.milestone == AutomationMilestone::Rise50)
                .unwrap()
                .adjusted_time_local
        };
        assert_eq!(rise50(&shifted) - rise50(&base), Duration::minutes(45));
        assert_ne!(base.monitors[0].samples, shifted.monitors[0].samples);
    }
}
