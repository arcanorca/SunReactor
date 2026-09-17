use chrono::Utc;

use crate::config::Config;
use crate::policy::{self, MonitorMilestoneSchedule, PolicyContext};
use crate::solar::Location;
use crate::tui::model::{Model, NextMilestoneInfo};

impl Model {
    #[must_use]
    pub fn selected_monitor_schedule(&self) -> Option<&MonitorMilestoneSchedule> {
        let logical_id = self.selected_monitor_logical_id()?;
        self.monitor_milestones
            .iter()
            .find(|schedule| schedule.logical_id == logical_id)
    }

    #[must_use]
    pub fn next_automation_milestone(
        &self,
        now_local: &chrono::DateTime<chrono::FixedOffset>,
    ) -> Option<NextMilestoneInfo> {
        let schedule = self
            .selected_monitor_schedule()
            .or_else(|| self.monitor_milestones.first())?;

        let (next, is_tomorrow) = if let Some(future) = schedule
            .milestones
            .iter()
            .find(|milestone| milestone.adjusted_time_local > *now_local)
        {
            (future, false)
        } else {
            (schedule.milestones.first()?, true)
        };

        let time_str = if self.config.tui.use_12h_time {
            next.adjusted_time_local.format("%I:%M %p").to_string()
        } else {
            next.adjusted_time_local.format("%H:%M").to_string()
        };

        Some(NextMilestoneInfo {
            milestone_label: next.milestone.label(),
            target_percent: next.target_percent,
            time_str,
            minutes_offset: next.minutes_offset,
            is_tomorrow,
        })
    }

    pub fn refresh_monitor_milestones_if_needed(&mut self) {
        // Keyboard range/curve adjustments are coalesced. Rebuilding the daily
        // solar schedule here would put expensive work back on a key-repeat path.
        if self.monitor_milestones_dirty {
            return;
        }
        let minute = Utc::now().timestamp() / 60;
        if self.last_milestone_refresh_minute == Some(minute) || self.preview_job.is_some() {
            return;
        }
        self.request_preview_refresh();
    }

    pub(super) fn refresh_monitor_milestones_if_empty(&mut self) {
        if self.monitor_milestones.is_empty() && !self.monitor_milestones_dirty {
            self.refresh_monitor_milestones();
        }
    }

    /// Lightweight offset-only update: skips the expensive solar simulation
    /// and only recalculates adjusted times from existing base times + config offsets.
    /// Use after `adjust_selected_monitor_milestone` or `reset_selected_monitor_milestone`.
    pub fn reapply_milestone_offsets(&mut self) {
        for schedule in &mut self.monitor_milestones {
            let Some(monitor) = self
                .config
                .monitors
                .iter()
                .find(|monitor| monitor.logical_id == schedule.logical_id)
            else {
                continue;
            };
            for milestone_entry in &mut schedule.milestones {
                let offset = monitor
                    .milestone_adjustments
                    .iter()
                    .find(|adjustment| adjustment.milestone == milestone_entry.milestone)
                    .map_or(0i16, |adjustment| adjustment.minutes_offset);
                milestone_entry.minutes_offset = offset;
                milestone_entry.adjusted_time_local =
                    milestone_entry.base_time_local + chrono::Duration::minutes(i64::from(offset));
            }
            // Re-enforce monotonicity: each adjusted time must be >= previous + 1 min
            for index in 1..schedule.milestones.len() {
                let minimum = schedule.milestones[index - 1].adjusted_time_local
                    + chrono::Duration::minutes(1);
                if schedule.milestones[index].adjusted_time_local < minimum {
                    schedule.milestones[index].adjusted_time_local = minimum;
                }
            }
            // Backward pass: each adjusted time must be <= next - 1 min
            for index in (0..schedule.milestones.len().saturating_sub(1)).rev() {
                let maximum = schedule.milestones[index + 1].adjusted_time_local
                    - chrono::Duration::minutes(1);
                if schedule.milestones[index].adjusted_time_local > maximum {
                    schedule.milestones[index].adjusted_time_local = maximum;
                }
            }
        }
    }

    /// Determines whether a milestone's effective scheduled time has been constrained
    /// by monotonicity or day boundaries, differing from the requested solar+offset time.
    #[must_use]
    pub fn is_milestone_constrained(
        &self,
        logical_id: &str,
        milestone: &crate::policy::MonitorMilestone,
    ) -> bool {
        let requested_offset = self
            .config
            .monitors
            .iter()
            .find(|monitor| monitor.logical_id == logical_id)
            .and_then(|monitor| {
                monitor
                    .milestone_adjustments
                    .iter()
                    .find(|adjustment| adjustment.milestone == milestone.milestone)
            })
            .map_or(0, |adjustment| adjustment.minutes_offset);

        milestone.is_constrained(requested_offset)
    }
}

pub(crate) fn build_monitor_milestones(
    config: &Config,
    now_utc: chrono::DateTime<Utc>,
    weather_multiplier: Option<f64>,
) -> Result<Vec<MonitorMilestoneSchedule>, String> {
    config.validate().map_err(|error| error.to_string())?;
    let location = Location::from_timezone_name(
        config.location.latitude,
        config.location.longitude,
        &config.location.timezone,
    )
    .map_err(|error| error.to_string())?;

    policy::compute_monitor_milestones(&PolicyContext {
        now_utc,
        location: &location,
        config: &config.solar_policy,
        weather_multiplier,
        monitors: &config.monitors,
    })
    .map_err(|error| error.to_string())
}
