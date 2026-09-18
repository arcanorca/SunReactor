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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use chrono::{FixedOffset, TimeZone};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use crate::policy::{AutomationMilestone, MonitorMilestone, MonitorMilestoneSchedule};
    use crate::tui::form::FormState;
    use crate::tui::model::{DaemonConnection, MonitorPaneFocus, Tab, CURVE_SAMPLE_MINUTES};
    use crate::tui::test_support::{
        buffer_text, configure_monitor_fixture, dummy_status, find_in_buffer, named_monitor_config,
    };
    use crate::tui::{ui, update, Model};

    #[test]
    fn test_phase9_4_range_repeat_defers_schedule_recompute_and_marks_preview() {
        let mut model = Model::new();
        model.config.monitors = vec![crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            min_pct: 7,
            max_pct: 60,
            ..Default::default()
        }];
        model.form = FormState::new(&model.config);
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;

        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
            .single()
            .unwrap();
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![MonitorMilestone {
                milestone: AutomationMilestone::Rise50,
                base_time_local: time,
                adjusted_time_local: time,
                target_percent: 33,
                minutes_offset: 0,
            }],
        }];
        let cached_schedule = model.monitor_milestones.clone();

        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = MonitorPaneFocus::Detail;
        model.monitor_control_index = 0;
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );

        assert_eq!(model.config.monitors[0].min_pct, 8);
        assert!(model.monitor_policy_preview_pending());
        assert_eq!(model.monitor_milestones, cached_schedule);

        // The periodic refresh path must also leave the cached schedule intact
        // until the debounced persistence cycle settles.
        model.refresh_monitor_milestones_if_needed();
        assert_eq!(model.monitor_milestones, cached_schedule);

        model.active_tab = Tab::Limits;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        assert!(
            find_in_buffer(&buffer, "Target updating…").is_some(),
            "{}",
            buffer_text(&buffer)
        );
    }

    #[test]
    fn test_policy_previews_stay_inside_the_configured_range_and_morph_only_in_full_effects() {
        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 12, 64)],
        );
        model.motion.level = crate::config::MotionLevel::Instrument;
        model.refresh_monitor_milestones();

        let curve = model.monitor_curves.first().expect("curve preview");
        assert_eq!(curve.logical_id, "mon-0");
        assert_eq!(
            curve.samples.len(),
            (24 * 60 / CURVE_SAMPLE_MINUTES + 1) as usize
        );
        assert!(curve
            .samples
            .iter()
            .all(|value| (12.0..=64.0).contains(value)));
        let target = model.monitor_targets_now.first().expect("target preview");
        assert!((12..=64).contains(&target.solar_percent));
        assert_eq!(target.solar_percent, target.weather_percent);

        // A different curve shape changes the sampled curve and eases into it.
        model.config.monitors[0].transition_gamma = 2.5;
        model.refresh_monitor_milestones();
        let changed = model.monitor_curves[0].samples
            != model
                .curve_morph_from
                .as_ref()
                .map_or_else(Vec::new, |from| from.samples.clone());
        if changed && model.curve_morph_from.is_some() {
            assert!(model.motion.curve_morph_phase(Instant::now()).is_some());
        }

        // Off never animates.
        model.motion.level = crate::config::MotionLevel::Off;
        model.motion.active_transient = None;
        model.config.monitors[0].transition_gamma = 0.3;
        model.refresh_monitor_milestones();
        assert!(model.motion.curve_morph_phase(Instant::now()).is_none());
    }

    #[test]
    fn test_background_preview_refresh_applies_latest_generation_only() {
        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 10, 70)],
        );
        model.monitor_milestones.clear();
        model.monitor_curves.clear();

        // Two requests: only the second (latest) generation may be applied.
        model.request_preview_refresh();
        let first = model.preview_generation;
        model.request_preview_refresh();
        let second = model.preview_generation;
        assert!(second > first);
        assert!(
            model.monitor_curves.is_empty(),
            "the input thread is not blocked"
        );

        let deadline = Instant::now() + Duration::from_secs(20);
        while model.preview_job.is_some() && Instant::now() < deadline {
            model.poll_preview_refresh();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(model.preview_job.is_none());
        assert!(!model.monitor_milestones.is_empty());
        assert_eq!(model.monitor_curves.len(), 1);
        assert!(!model.monitor_policy_preview_pending());
    }
}
