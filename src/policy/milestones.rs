use crate::policy::{
    compute_monitor_target_percent, linear_daylight_factor_at_local, solar_elevation_at_utc,
    PolicyContext, PolicyError,
};
use crate::solar::{self, Location};
use chrono::{DateTime, Duration, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

pub(crate) const AUTO_MINIMUM_BRIGHTNESS_AFTER_SUNSET_MINUTES: i64 = 90;
pub(crate) const AUTO_MINIMUM_BRIGHTNESS_AFTER_DUSK_MINUTES: i64 = 30;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AutomationMilestone {
    #[serde(rename = "rise_start")]
    RiseStart,
    #[serde(rename = "rise_25")]
    Rise25,
    #[serde(rename = "rise_50")]
    Rise50,
    #[serde(rename = "rise_75")]
    Rise75,
    #[serde(rename = "peak")]
    Peak,
    #[serde(rename = "fall_75")]
    Fall75,
    #[serde(rename = "fall_50")]
    Fall50,
    #[serde(rename = "fall_25")]
    Fall25,
    #[serde(rename = "night_floor")]
    NightFloor,
}
impl AutomationMilestone {
    pub const ALL: [Self; 9] = [
        Self::RiseStart,
        Self::Rise25,
        Self::Rise50,
        Self::Rise75,
        Self::Peak,
        Self::Fall75,
        Self::Fall50,
        Self::Fall25,
        Self::NightFloor,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RiseStart => "rise_start",
            Self::Rise25 => "rise_25",
            Self::Rise50 => "rise_50",
            Self::Rise75 => "rise_75",
            Self::Peak => "peak",
            Self::Fall75 => "fall_75",
            Self::Fall50 => "fall_50",
            Self::Fall25 => "fall_25",
            Self::NightFloor => "night_floor",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::RiseStart => "Rise Start",
            Self::Rise25 => "Rise 25%",
            Self::Rise50 => "Rise 50%",
            Self::Rise75 => "Rise 75%",
            Self::Peak => "Peak",
            Self::Fall75 => "Fall 75%",
            Self::Fall50 => "Fall 50%",
            Self::Fall25 => "Fall 25%",
            Self::NightFloor => "Night Floor",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorMilestone {
    pub milestone: AutomationMilestone,
    pub base_time_local: DateTime<chrono::FixedOffset>,
    pub adjusted_time_local: DateTime<chrono::FixedOffset>,
    pub target_percent: u8,
    pub minutes_offset: i16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorMilestoneSchedule {
    pub logical_id: String,
    pub milestones: Vec<MonitorMilestone>,
}
pub(crate) struct BaseMilestoneContext {
    pub(crate) peak_linear_factor: f64,
    pub(crate) day_start_local: DateTime<chrono::FixedOffset>,
    pub(crate) day_end_local: DateTime<chrono::FixedOffset>,
    pub(crate) peak_local: DateTime<chrono::FixedOffset>,
    pub(crate) sunset_local: DateTime<chrono::FixedOffset>,
    pub(crate) minimum_brightness_start_local: DateTime<chrono::FixedOffset>,
    pub(crate) sunset_linear_factor: f64,
}
pub(crate) struct BaseMilestone {
    pub(crate) milestone: AutomationMilestone,
    local_time: DateTime<chrono::FixedOffset>,
}
pub(crate) struct AdjustedMilestone {
    pub(crate) milestone: AutomationMilestone,
    base_time_local: DateTime<chrono::FixedOffset>,
    pub(crate) adjusted_time_local: DateTime<chrono::FixedOffset>,
    minutes_offset: i16,
}
pub fn compute_adaptive_zenith(
    now_utc: DateTime<Utc>,
    location: &solar::Location,
    config_day_full_deg: f64,
    _use_adaptive_zenith: bool,
    twilight_start_deg: f64,
) -> f64 {
    // `day_elevation_full` and `use_adaptive_zenith` are kept in the config
    // schema for backward compatibility but are no longer used in the
    // computation. The plateau is ALWAYS the day's actual solar noon
    // elevation, producing a natural bell-curve brightness profile that
    // peaks only at solar noon and tapers symmetrically throughout the day.

    let fallback = config_day_full_deg.max(twilight_start_deg + 0.001);
    let now_local = match solar::local_datetime_at_utc(now_utc, location) {
        Ok(dt) => dt,
        Err(_) => return fallback,
    };

    let events = match solar::safe_get_sun_events(now_local.date_naive(), location) {
        Ok(e) => e,
        Err(_) => return fallback,
    };

    let noon_sample = match solar::sample_at_utc(
        events.noon.with_timezone(&Utc),
        location,
        twilight_start_deg,
        config_day_full_deg,
    ) {
        Ok(s) => s,
        Err(_) => return fallback,
    };

    let noon_elevation_deg = f64::from(noon_sample.elevation_deg);
    if !noon_elevation_deg.is_finite() {
        return fallback;
    }

    noon_elevation_deg.max(twilight_start_deg + 0.1)
}
pub(crate) fn compute_monitor_milestones(
    input: &PolicyContext,
) -> Result<Vec<MonitorMilestoneSchedule>, PolicyError> {
    let context = resolve_base_milestone_context(input)?;
    let base_milestones = resolve_base_milestones(input, &context)?;

    Ok(input
        .monitors
        .iter()
        .map(|monitor| {
            let adjusted = adjusted_milestones(monitor, &context, &base_milestones);
            MonitorMilestoneSchedule {
                logical_id: monitor.logical_id.clone(),
                milestones: adjusted
                    .into_iter()
                    .map(|entry| {
                        let linear_factor =
                            milestone_factor(entry.milestone, context.peak_linear_factor);
                        let gamma_factor = crate::policy::math::apply_gamma(
                            linear_factor,
                            monitor.transition_gamma,
                        );
                        MonitorMilestone {
                            milestone: entry.milestone,
                            base_time_local: entry.base_time_local,
                            adjusted_time_local: entry.adjusted_time_local,
                            target_percent: compute_monitor_target_percent(monitor, gamma_factor),
                            minutes_offset: entry.minutes_offset,
                        }
                    })
                    .collect(),
            }
        })
        .collect())
}
pub(crate) fn remap_adjusted_time_to_base(
    now_local: DateTime<chrono::FixedOffset>,
    current: &AdjustedMilestone,
    next: &AdjustedMilestone,
) -> DateTime<chrono::FixedOffset> {
    let adjusted_span = (next.adjusted_time_local - current.adjusted_time_local).num_seconds();
    if adjusted_span <= 0 {
        return next.base_time_local;
    }

    let elapsed = (now_local - current.adjusted_time_local)
        .num_seconds()
        .clamp(0, adjusted_span);
    let base_span = (next.base_time_local - current.base_time_local).num_seconds();
    let mapped_seconds =
        (base_span as f64 * (elapsed as f64 / adjusted_span as f64)).round() as i64;

    current.base_time_local + Duration::seconds(mapped_seconds)
}
pub(crate) fn adjusted_milestones(
    monitor: &crate::config::MonitorConfig,
    context: &BaseMilestoneContext,
    base_milestones: &[BaseMilestone],
) -> Vec<AdjustedMilestone> {
    let mut adjusted = base_milestones
        .iter()
        .map(|base| AdjustedMilestone {
            milestone: base.milestone,
            base_time_local: base.local_time,
            adjusted_time_local: base.local_time,
            minutes_offset: 0,
        })
        .collect::<Vec<_>>();

    for config in &monitor.milestone_adjustments {
        if let Some(entry) = adjusted
            .iter_mut()
            .find(|m| m.milestone == config.milestone)
        {
            entry.minutes_offset = config.minutes_offset;
            entry.adjusted_time_local =
                entry.base_time_local + Duration::minutes(i64::from(config.minutes_offset));
        }
    }

    for i in 0..adjusted.len() {
        let prev_limit = if i == 0 {
            context.day_start_local
        } else {
            adjusted[i - 1].adjusted_time_local
        };

        if adjusted[i].adjusted_time_local < prev_limit {
            adjusted[i].adjusted_time_local = prev_limit;
            adjusted[i].minutes_offset = (adjusted[i].adjusted_time_local
                - adjusted[i].base_time_local)
                .num_minutes() as i16;
        }
    }

    for i in (0..adjusted.len()).rev() {
        let next_limit = if i == adjusted.len() - 1 {
            context.day_end_local
        } else {
            adjusted[i + 1].adjusted_time_local
        };

        if adjusted[i].adjusted_time_local > next_limit {
            adjusted[i].adjusted_time_local = next_limit;
            adjusted[i].minutes_offset = (adjusted[i].adjusted_time_local
                - adjusted[i].base_time_local)
                .num_minutes() as i16;
        }
    }

    adjusted
}
pub(crate) fn resolve_base_milestone_context_for_date(
    input: &PolicyContext,
    date: chrono::NaiveDate,
) -> Result<BaseMilestoneContext, PolicyError> {
    let start_of_day = resolve_local_minute_of_day(date, 0, input.location)?;
    let end_of_day = resolve_local_minute_of_day(
        date.succ_opt()
            .expect("policy date should remain within chrono's supported range"),
        0,
        input.location,
    )?;
    let events = solar::get_sun_events(date, input.location)?;
    let minimum_brightness_start = resolve_auto_minimum_brightness_start(&events);
    let peak_local = events.noon;
    let peak_linear_factor = linear_daylight_factor_at_local(input, peak_local)?;
    let sunset_linear_factor = linear_daylight_factor_at_local(input, events.sunset)?;

    Ok(BaseMilestoneContext {
        peak_linear_factor,
        day_start_local: start_of_day,
        day_end_local: end_of_day.max(minimum_brightness_start + Duration::minutes(1)),
        peak_local,
        sunset_local: events.sunset,
        minimum_brightness_start_local: minimum_brightness_start,
        sunset_linear_factor,
    })
}

pub(crate) fn find_rise_start(
    input: &PolicyContext,
    context: &BaseMilestoneContext,
) -> Result<DateTime<chrono::FixedOffset>, PolicyError> {
    if context.peak_linear_factor <= 0.0 {
        return Ok(context.day_start_local);
    }

    let start = context.day_start_local;
    let end = context.peak_local;
    let threshold = input.config.twilight_elevation_start;

    let mut current = start;
    let mut rise_start = start;

    while current <= end {
        let current_utc = current.with_timezone(&Utc);
        let elevation = solar_elevation_at_utc(input, current_utc).unwrap_or(-90.0);
        if elevation >= threshold {
            rise_start = current;
            break;
        }
        current += Duration::minutes(1);
    }

    Ok(rise_start)
}
pub(crate) fn resolve_base_milestone_context(
    input: &PolicyContext,
) -> Result<BaseMilestoneContext, PolicyError> {
    let now_local = solar::local_datetime_at_utc(input.now_utc, input.location)?;
    let mut date = now_local.date_naive();

    let mut context = resolve_base_milestone_context_for_date(input, date)?;
    let rise_start = find_rise_start(input, &context)?;

    if now_local < rise_start {
        // We are before today's sunrise, so we actually belong to yesterday's cycle.
        date = date
            .pred_opt()
            .expect("policy date should remain within chrono's supported range");
        context = resolve_base_milestone_context_for_date(input, date)?;

        // Yesterday's NightFloor can be pushed up to 1 minute before today's sunrise.
        context.day_end_local = rise_start - Duration::minutes(1);
    } else {
        // We are within today's cycle.
        let tomorrow = date
            .succ_opt()
            .expect("policy date should remain within chrono's supported range");
        let tomorrow_context = resolve_base_milestone_context_for_date(input, tomorrow)?;
        let tomorrow_rise = find_rise_start(input, &tomorrow_context)?;

        // Today's NightFloor can be pushed up to 1 minute before tomorrow's sunrise.
        context.day_end_local = tomorrow_rise - Duration::minutes(1);
    }

    Ok(context)
}
pub(crate) fn resolve_base_milestones(
    input: &PolicyContext,
    context: &BaseMilestoneContext,
) -> Result<Vec<BaseMilestone>, PolicyError> {
    let target_rise25 = milestone_factor(AutomationMilestone::Rise25, context.peak_linear_factor);
    let target_rise50 = milestone_factor(AutomationMilestone::Rise50, context.peak_linear_factor);
    let target_rise75 = milestone_factor(AutomationMilestone::Rise75, context.peak_linear_factor);
    let target_fall75 = milestone_factor(AutomationMilestone::Fall75, context.peak_linear_factor);
    let target_fall50 = milestone_factor(AutomationMilestone::Fall50, context.peak_linear_factor);
    let target_fall25 = milestone_factor(AutomationMilestone::Fall25, context.peak_linear_factor);

    let threshold_rise_start = input.config.twilight_elevation_start;

    let mut rise_start = context.day_start_local;
    let mut rise25 = context.peak_local;
    let mut rise50 = context.peak_local;
    let mut rise75 = context.peak_local;
    let mut fall75 = context.minimum_brightness_start_local;
    let mut fall50 = context.minimum_brightness_start_local;
    let mut fall25 = context.minimum_brightness_start_local;

    // Daily Simulation (Iterative Scanning)
    // We loop minute-by-minute to find the exact crossings to perfectly sync with the Daemon.

    // Rising Phase (Start of Day to Peak)
    let mut current = context.day_start_local;
    let mut found_rise_start = false;
    let mut found_rise25 = false;
    let mut found_rise50 = false;
    let mut found_rise75 = false;

    while current <= context.peak_local {
        let current_utc = current.with_timezone(&Utc);

        if !found_rise_start {
            let elevation = solar_elevation_at_utc(input, current_utc).unwrap_or(-90.0);
            if elevation >= threshold_rise_start {
                rise_start = current;
                found_rise_start = true;
            }
        }

        let factor = crate::policy::base_linear_effective_daylight_factor_at_utc(
            input,
            context,
            current_utc,
        )
        .unwrap_or(0.0);

        if !found_rise25 && factor >= target_rise25 {
            rise25 = current;
            found_rise25 = true;
        }
        if !found_rise50 && factor >= target_rise50 {
            rise50 = current;
            found_rise50 = true;
        }
        if !found_rise75 && factor >= target_rise75 {
            rise75 = current;
            found_rise75 = true;
        }

        current += Duration::minutes(1);
    }

    // Falling Phase (Peak to NightFloor)
    let mut current = context.peak_local;
    let mut found_fall75 = false;
    let mut found_fall50 = false;
    let mut found_fall25 = false;

    while current <= context.minimum_brightness_start_local {
        let current_utc = current.with_timezone(&Utc);
        let factor = crate::policy::base_linear_effective_daylight_factor_at_utc(
            input,
            context,
            current_utc,
        )
        .unwrap_or(0.0);

        if !found_fall75 && factor <= target_fall75 {
            fall75 = current;
            found_fall75 = true;
        }
        if !found_fall50 && factor <= target_fall50 {
            fall50 = current;
            found_fall50 = true;
        }
        if !found_fall25 && factor <= target_fall25 {
            fall25 = current;
            found_fall25 = true;
        }

        current += Duration::minutes(1);
    }

    Ok(vec![
        BaseMilestone {
            milestone: AutomationMilestone::RiseStart,
            local_time: rise_start,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Rise25,
            local_time: rise25,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Rise50,
            local_time: rise50,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Rise75,
            local_time: rise75,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Peak,
            local_time: context.peak_local,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Fall75,
            local_time: fall75,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Fall50,
            local_time: fall50,
        },
        BaseMilestone {
            milestone: AutomationMilestone::Fall25,
            local_time: fall25,
        },
        BaseMilestone {
            milestone: AutomationMilestone::NightFloor,
            local_time: context.minimum_brightness_start_local,
        },
    ])
}

pub(crate) fn milestone_factor(milestone: AutomationMilestone, peak_factor: f64) -> f64 {
    match milestone {
        AutomationMilestone::RiseStart | AutomationMilestone::NightFloor => 0.0,
        AutomationMilestone::Rise25 | AutomationMilestone::Fall25 => peak_factor * 0.25,
        AutomationMilestone::Rise50 | AutomationMilestone::Fall50 => peak_factor * 0.50,
        AutomationMilestone::Rise75 | AutomationMilestone::Fall75 => peak_factor * 0.75,
        AutomationMilestone::Peak => peak_factor,
    }
}
pub(crate) fn relevant_evening_events(
    now_local: DateTime<chrono::FixedOffset>,
    location: &Location,
) -> Result<Option<solar::SunEvents>, PolicyError> {
    let today_events = solar::get_sun_events(now_local.date_naive(), location)?;
    if now_local >= today_events.sunrise && now_local < today_events.sunset {
        return Ok(None);
    }
    if now_local >= today_events.sunset {
        return Ok(Some(today_events));
    }

    let yesterday = now_local
        .date_naive()
        .pred_opt()
        .expect("policy date should remain within chrono's supported range");
    Ok(Some(solar::get_sun_events(yesterday, location)?))
}
pub(crate) fn resolve_auto_minimum_brightness_start(
    evening_events: &solar::SunEvents,
) -> DateTime<chrono::FixedOffset> {
    let after_sunset =
        evening_events.sunset + Duration::minutes(AUTO_MINIMUM_BRIGHTNESS_AFTER_SUNSET_MINUTES);
    let after_dusk =
        evening_events.dusk + Duration::minutes(AUTO_MINIMUM_BRIGHTNESS_AFTER_DUSK_MINUTES);
    after_sunset.max(after_dusk)
}
pub(crate) fn resolve_local_minute_of_day(
    date: chrono::NaiveDate,
    minute_of_day: u16,
    location: &Location,
) -> Result<DateTime<chrono::FixedOffset>, PolicyError> {
    let time = local_time_from_minute_of_day(minute_of_day)
        .expect("validated minute-of-day should produce a local time");
    let datetime = date.and_time(time);
    solar::local_datetime(datetime, location).map_err(Into::into)
}
pub(crate) fn local_time_from_minute_of_day(minute_of_day: u16) -> Option<NaiveTime> {
    if minute_of_day >= 24 * 60 {
        return None;
    }

    NaiveTime::from_hms_opt(
        u32::from(minute_of_day / 60),
        u32::from(minute_of_day % 60),
        0,
    )
}
