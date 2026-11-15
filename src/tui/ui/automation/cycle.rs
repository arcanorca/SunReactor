use std::time::Instant;

use chrono::{DateTime, Duration, FixedOffset, Timelike};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::kit;
use super::schedule::format_time;
use crate::{
    policy::{AutomationMilestone, MonitorMilestoneSchedule},
    solar::{self, Location},
    tui::{
        model::{CurvePreview, CURVE_SAMPLE_MINUTES},
        theme::SemanticStyles,
        Model,
    },
};

#[derive(Debug, Clone, Default)]
pub(crate) struct CycleAnchors {
    pub(crate) dawn: Option<DateTime<FixedOffset>>,
    pub(crate) sunrise: Option<DateTime<FixedOffset>>,
    pub(crate) noon: Option<DateTime<FixedOffset>>,
    pub(crate) sunset: Option<DateTime<FixedOffset>>,
    pub(crate) dusk: Option<DateTime<FixedOffset>>,
}

pub(crate) fn render_small_cycle(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    schedule: Option<&MonitorMilestoneSchedule>,
    anchors: &CycleAnchors,
    area: Rect,
    styles: &SemanticStyles,
) {
    let noon = anchors
        .noon
        .as_ref()
        .map_or_else(|| String::from("—"), |time| format_time(time, false));
    let peak = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::Peak))
        .map_or_else(
            || String::from("—"),
            |entry| format!("{}%", entry.target_percent),
        );
    let target = target_preview(app, logical_id).map_or_else(
        || String::from("—"),
        |target| format!("{}%", target.weather_percent),
    );
    let text = format!("Solar noon {noon} · peak {peak}");
    let now = format_time(&app.current_local_time(), app.config.tui.use_12h_time);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                super::super::truncate(&text, usize::from(area.width)),
                styles.text_primary.add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(format!("{} NOW ", kit::DOT), styles.current_marker),
                Span::styled(now, styles.text_primary),
                Span::styled(kit::SEPARATOR, styles.border_normal),
                Span::styled("Target ", styles.data_label),
                Span::styled(target, styles.data_value),
            ]),
        ]),
        area,
    );
}

pub(crate) fn cycle_anchors(
    app: &Model,
    schedule: Option<&MonitorMilestoneSchedule>,
) -> CycleAnchors {
    let status_sunrise = app
        .status
        .as_ref()
        .and_then(|status| status.sunrise_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let status_sunset = app
        .status
        .as_ref()
        .and_then(|status| status.sunset_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let date = app.current_local_time().date_naive();
    let solar_events = Location::from_timezone_name(
        app.config.location.latitude,
        app.config.location.longitude,
        &app.config.location.timezone,
    )
    .ok()
    .and_then(|location| solar::get_sun_events(date, &location).ok());

    let fallback_sunrise = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::RiseStart))
        .map(|entry| entry.base_time_local);
    let fallback_noon = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::Peak))
        .map(|entry| entry.base_time_local);
    let fallback_sunset = schedule
        .and_then(|schedule| milestone(schedule, AutomationMilestone::NightFloor))
        .map(|entry| entry.base_time_local - Duration::minutes(90));

    CycleAnchors {
        dawn: solar_events.as_ref().map(|events| events.dawn),
        sunrise: status_sunrise
            .or_else(|| solar_events.as_ref().map(|events| events.sunrise))
            .or(fallback_sunrise),
        noon: solar_events
            .as_ref()
            .map(|events| events.noon)
            .or(fallback_noon),
        sunset: status_sunset
            .or_else(|| solar_events.as_ref().map(|events| events.sunset))
            .or(fallback_sunset),
        dusk: solar_events.as_ref().map(|events| events.dusk),
    }
}

pub(crate) fn cycle_bounds(anchors: &CycleAnchors) -> [f64; 7] {
    let mut dawn = anchors.dawn.as_ref().map_or(300.0, minute_of_day);
    let mut sunrise = anchors.sunrise.as_ref().map_or(360.0, minute_of_day);
    let mut noon = anchors.noon.as_ref().map_or(720.0, minute_of_day);
    let mut sunset = anchors.sunset.as_ref().map_or(1080.0, minute_of_day);
    let mut dusk = anchors.dusk.as_ref().map_or(1140.0, minute_of_day);
    dawn = dawn.clamp(0.0, 1435.0);
    sunrise = sunrise.clamp(dawn + 1.0, 1436.0);
    noon = noon.clamp(sunrise + 1.0, 1437.0);
    sunset = sunset.clamp(noon + 1.0, 1438.0);
    dusk = dusk.clamp(sunset + 1.0, 1439.0);
    [0.0, dawn, sunrise, noon, sunset, dusk, 1440.0]
}

pub(crate) fn milestone(
    schedule: &MonitorMilestoneSchedule,
    kind: AutomationMilestone,
) -> Option<&crate::policy::MonitorMilestone> {
    schedule
        .milestones
        .iter()
        .find(|entry| entry.milestone == kind)
}

pub(crate) fn target_preview<'a>(
    app: &'a Model,
    logical_id: &str,
) -> Option<&'a crate::tui::model::TargetPreview> {
    app.monitor_targets_now
        .iter()
        .find(|target| target.logical_id == logical_id)
}

pub(crate) fn minute_of_day(time: &DateTime<FixedOffset>) -> f64 {
    f64::from(time.hour() * 60 + time.minute())
}

pub(crate) fn curve_value_at(app: &Model, curve: &CurvePreview, minute: f64, now: Instant) -> f64 {
    let current = curve_sample_value(curve, minute);
    let Some(phase) = app.motion.curve_morph_phase(now) else {
        return current;
    };
    let Some(previous) = app.curve_morph_from.as_ref() else {
        return current;
    };
    if previous.logical_id != curve.logical_id || previous.samples.len() != curve.samples.len() {
        return current;
    }
    let eased = f64::from(1.0 - (1.0 - phase).powi(3));
    let old = curve_sample_value(previous, minute);
    old + (current - old) * eased
}

fn curve_sample_value(curve: &CurvePreview, minute: f64) -> f64 {
    let minute = minute.clamp(0.0, 1440.0);
    let sample = minute / f64::from(CURVE_SAMPLE_MINUTES);
    let index = sample.floor() as usize;
    let fraction = sample - index as f64;
    match (curve.samples.get(index), curve.samples.get(index + 1)) {
        (Some(a), Some(b)) => f64::from(*a) + (f64::from(*b) - f64::from(*a)) * fraction,
        (Some(a), None) => f64::from(*a),
        _ => curve.samples.last().copied().map_or(0.0, f64::from),
    }
}
