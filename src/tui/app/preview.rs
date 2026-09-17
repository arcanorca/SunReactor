use std::time::{Duration, Instant};

use chrono::Utc;

use crate::config::Config;
use crate::policy::{self, MonitorMilestoneSchedule, PolicyContext};
use crate::solar::Location;
use crate::tui::model::{CurvePreview, Model, TargetPreview, CURVE_SAMPLE_MINUTES};
use crate::tui::motion::TransientKind;

pub(crate) struct PreviewInputs {
    pub(crate) generation: u64,
    pub(crate) config: Config,
    pub(crate) now_utc: chrono::DateTime<Utc>,
    pub(crate) midnight_utc: chrono::DateTime<Utc>,
    pub(crate) weather_multiplier: Option<f64>,
}

pub(crate) struct PreviewOutcome {
    pub(crate) generation: u64,
    pub(crate) milestones: Result<Vec<MonitorMilestoneSchedule>, String>,
    pub(crate) previews: Result<PolicyPreviews, String>,
}

pub(crate) type PolicyPreviews = (Vec<TargetPreview>, Vec<CurvePreview>);

impl Model {
    #[must_use]
    pub fn monitor_policy_preview_pending(&self) -> bool {
        self.monitor_milestones_dirty
    }

    /// Recomputes the schedule and previews synchronously. Used at startup and
    /// by tests; interactive paths use [`Self::request_preview_refresh`].
    pub fn refresh_monitor_milestones(&mut self) {
        let job = self.preview_inputs();
        self.preview_job = None;
        let outcome = compute_preview_outcome(&job);
        self.apply_preview_outcome(outcome);
    }

    /// Starts a background recomputation. A newer request supersedes an
    /// older one; stale results are discarded when they arrive.
    pub fn request_preview_refresh(&mut self) {
        let job = self.preview_inputs();
        let generation = job.generation;
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name(String::from("sunreactor-preview"))
            .spawn(move || {
                let _ = tx.send(compute_preview_outcome(&job));
            });
        match spawned {
            Ok(_) => self.preview_job = Some((generation, rx)),
            Err(_) => self.refresh_monitor_milestones(),
        }
    }

    /// Applies a finished background recomputation, if any.
    pub fn poll_preview_refresh(&mut self) {
        let Some((generation, rx)) = &self.preview_job else {
            return;
        };
        match rx.try_recv() {
            Ok(outcome) if outcome.generation == *generation => {
                self.preview_job = None;
                self.apply_preview_outcome(outcome);
            }
            Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => self.preview_job = None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn preview_inputs(&mut self) -> PreviewInputs {
        self.preview_generation = self.preview_generation.wrapping_add(1);
        let now_utc = Utc::now();
        self.last_milestone_refresh_minute = Some(now_utc.timestamp() / 60);
        let local = self.local_time_at(now_utc);
        let midnight = local
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is a valid time");
        let midnight_utc = chrono::DateTime::<Utc>::from_naive_utc_and_offset(
            midnight - chrono::Duration::seconds(i64::from(local.offset().local_minus_utc())),
            Utc,
        );
        PreviewInputs {
            generation: self.preview_generation,
            config: self.config.clone(),
            now_utc,
            midnight_utc,
            weather_multiplier: self
                .status
                .as_ref()
                .and_then(|s| s.weather.as_ref())
                .and_then(|w| w.multiplier),
        }
    }

    fn apply_preview_outcome(&mut self, outcome: PreviewOutcome) {
        let curve_changed = self.apply_policy_previews(outcome.previews);
        match outcome.milestones {
            Ok(milestones) => {
                self.monitor_milestone_error = None;
                self.monitor_milestones = milestones;
                self.monitor_milestones_dirty = false;
                if let Some(schedule) = self.selected_monitor_schedule() {
                    let max_index = schedule.milestones.len().saturating_sub(1);
                    if self.selected_monitor_milestone > max_index {
                        self.selected_monitor_milestone = max_index;
                    }
                } else {
                    self.selected_monitor_milestone = 0;
                }
            }
            Err(error) => {
                self.monitor_milestones.clear();
                self.monitor_milestone_error = Some(error);
                self.monitor_milestones_dirty = false;
                self.selected_monitor_milestone = 0;
            }
        }
        // Start the morph only once the new data is in place, so its whole
        // duration is visible.
        if curve_changed {
            self.motion.trigger(
                TransientKind::CurveMorph,
                Instant::now(),
                Duration::from_millis(320),
            );
        }
    }

    /// Installs new target previews and curves. Returns whether the selected
    /// monitor's curve visibly changed, keeping the previous curve for a morph.
    fn apply_policy_previews(&mut self, previews: Result<PolicyPreviews, String>) -> bool {
        let Ok((targets, curves)) = previews else {
            self.monitor_targets_now.clear();
            self.monitor_curves.clear();
            return false;
        };

        let selected = self.selected_monitor_logical_id().map(str::to_owned);
        let previous = selected.as_ref().and_then(|id| {
            self.monitor_curves
                .iter()
                .find(|curve| &curve.logical_id == id)
                .cloned()
        });
        let mut changed_curve = false;
        if let (Some(previous), Some(id)) = (previous, selected) {
            let changed = curves
                .iter()
                .find(|curve| curve.logical_id == id)
                .is_some_and(|next| {
                    next.samples.len() == previous.samples.len()
                        && next
                            .samples
                            .iter()
                            .zip(&previous.samples)
                            .any(|(a, b)| (a - b).abs() > 0.5)
                });
            if changed {
                self.curve_morph_from = Some(previous);
                changed_curve = true;
            }
        }
        self.monitor_targets_now = targets;
        self.monitor_curves = curves;
        changed_curve
    }
}

fn compute_preview_outcome(inputs: &PreviewInputs) -> PreviewOutcome {
    PreviewOutcome {
        generation: inputs.generation,
        previews: build_policy_previews(
            &inputs.config,
            inputs.now_utc,
            inputs.midnight_utc,
            inputs.weather_multiplier,
        ),
        milestones: super::milestones::build_monitor_milestones(
            &inputs.config,
            inputs.now_utc,
            inputs.weather_multiplier,
        ),
    }
}

fn build_policy_previews(
    config: &Config,
    now_utc: chrono::DateTime<Utc>,
    midnight_utc: chrono::DateTime<Utc>,
    weather_multiplier: Option<f64>,
) -> Result<PolicyPreviews, String> {
    if config.monitors.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    config.validate().map_err(|error| error.to_string())?;
    let location = Location::from_timezone_name(
        config.location.latitude,
        config.location.longitude,
        &config.location.timezone,
    )
    .map_err(|error| error.to_string())?;
    let evaluate = |at: chrono::DateTime<Utc>, multiplier: Option<f64>| {
        policy::compute_policy(&PolicyContext {
            now_utc: at,
            location: &location,
            config: &config.solar_policy,
            weather_multiplier: multiplier,
            monitors: &config.monitors,
        })
        .map_err(|error| error.to_string())
    };

    let solar = evaluate(now_utc, None)?;
    let weather = match weather_multiplier {
        Some(multiplier) => evaluate(now_utc, Some(multiplier))?,
        None => solar.clone(),
    };
    let targets = solar
        .targets
        .iter()
        .zip(&weather.targets)
        .map(|(solar, weather)| TargetPreview {
            logical_id: solar.logical_id.clone(),
            solar_percent: solar.percent,
            weather_percent: weather.percent,
        })
        .collect();

    let sample_count = (24 * 60 / CURVE_SAMPLE_MINUTES + 1) as usize;
    let mut curves: Vec<CurvePreview> = config
        .monitors
        .iter()
        .map(|monitor| CurvePreview {
            logical_id: monitor.logical_id.clone(),
            samples: Vec::with_capacity(sample_count),
        })
        .collect();
    for index in 0..sample_count {
        let at = midnight_utc
            + chrono::Duration::minutes(i64::from(CURVE_SAMPLE_MINUTES) * index as i64);
        let output = evaluate(at, None)?;
        for (curve, target) in curves.iter_mut().zip(&output.targets) {
            curve.samples.push(f32::from(target.percent));
        }
    }
    Ok((targets, curves))
}
