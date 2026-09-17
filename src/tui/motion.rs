use std::time::{Duration, Instant};

pub use crate::config::MotionLevel;

/// The kind of dominant transient effect currently active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransientKind {
    /// A localized commit confirmation on the Operating Range handles.
    RangeCommit { min: bool },
    /// An expanding acquisition ring on the world map centered at (lon, lat).
    LocationAcquisition { lon: f64, lat: f64 },
    /// A bounded Full-effects rotation from one configured globe viewpoint to
    /// another. The final configuration is applied immediately; this only
    /// animates the explanatory Location view.
    GlobeRotation {
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    },
    /// A subtle localized settle cue on an adjusted automation milestone row.
    MilestoneAdjust { index: usize },
    /// Bars trace in when a genuinely newer weather snapshot arrives.
    WeatherUpdate,
    /// The automation curve eases from its previous shape to a recomputed one.
    CurveMorph,
}

/// Duration of the output value count between two applied percentages.
pub const OUTPUT_ROLL_DURATION: Duration = Duration::from_millis(900);

/// The Automation output counting from its previous applied value to a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRoll {
    pub logical_id: String,
    pub from: u8,
    pub to: u8,
    pub started_at: Instant,
}

/// Duration of the tab indicator slide.
pub const TAB_SLIDE_DURATION: Duration = Duration::from_millis(180);

/// An asynchronous ongoing activity whose duration is externally determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveActivity {
    /// Asynchronous weather provider refresh in flight.
    WeatherRefresh { started_at: Instant },
}

impl ActiveActivity {
    #[must_use]
    pub fn started_at(&self) -> Instant {
        match *self {
            Self::WeatherRefresh { started_at } => started_at,
        }
    }
}

/// A time-bounded transient motion effect.
#[derive(Debug, Clone)]
pub struct TransientMotion {
    pub kind: TransientKind,
    pub started_at: Instant,
    pub duration: Duration,
}

impl TransientMotion {
    #[must_use]
    pub fn new(kind: TransientKind, started_at: Instant, duration: Duration) -> Self {
        Self {
            kind,
            started_at,
            duration,
        }
    }

    /// Computes the elapsed phase in `[0.0, 1.0]`. Returns `None` once expired.
    #[must_use]
    pub fn phase(&self, now: Instant) -> Option<f32> {
        if now < self.started_at {
            return Some(0.0);
        }
        let elapsed = now.duration_since(self.started_at);
        if elapsed >= self.duration {
            None
        } else {
            let total = self.duration.as_secs_f32();
            if total <= 0.0 {
                None
            } else {
                Some((elapsed.as_secs_f32() / total).clamp(0.0, 1.0))
            }
        }
    }
}

/// State tracking for all kinetic and transient effects in SunReactor.
#[derive(Debug, Clone)]
pub struct UiMotionState {
    /// Motion accessibility level.
    pub level: MotionLevel,
    /// Timestamp when the TUI session started.
    pub session_started_at: Instant,
    /// Duration of the one-time session startup masthead sweep.
    pub masthead_sweep_duration: Duration,
    /// Flag indicating whether the startup sweep has finished.
    pub masthead_sweep_done: bool,
    /// Timestamp of the last received real daemon status response.
    pub last_heartbeat_at: Option<Instant>,
    /// Duration of the heartbeat emphasis pulse.
    pub heartbeat_pulse_duration: Duration,
    /// The single dominant transient motion allowed under the motion budget.
    pub active_transient: Option<TransientMotion>,
    /// Long-running asynchronous activity currently in progress.
    pub active_activity: Option<ActiveActivity>,
    /// Chrome-only tab indicator slide: `(from_index, to_index, started_at)`.
    pub tab_slide: Option<(usize, usize, Instant)>,
    /// Local, Full-only count animation of the Automation output value. It is
    /// independent of the dominant transient budget because it never moves
    /// layout and lasts under a second.
    pub output_roll: Option<OutputRoll>,
}

impl Default for UiMotionState {
    fn default() -> Self {
        Self::new_at(Instant::now())
    }
}

impl UiMotionState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn new_at(now: Instant) -> Self {
        Self {
            level: MotionLevel::Instrument,
            session_started_at: now,
            masthead_sweep_duration: Duration::from_millis(650),
            masthead_sweep_done: false,
            last_heartbeat_at: None,
            heartbeat_pulse_duration: Duration::from_millis(300),
            active_transient: None,
            active_activity: None,
            tab_slide: None,
            output_roll: None,
        }
    }

    /// Starts an asynchronous ongoing activity.
    pub fn start_activity(&mut self, activity: ActiveActivity) {
        if self.level != MotionLevel::Off {
            self.active_activity = Some(activity);
        }
    }

    /// Ends any currently active asynchronous activity.
    pub fn stop_activity(&mut self) {
        self.active_activity = None;
    }

    /// Returns the normalized periodic phase `[0.0, 1.0)` of an ongoing activity shimmer,
    /// or `None` if inactive, suppressed by motion policy, or in the past.
    #[must_use]
    pub fn activity_shimmer_phase(&self, now: Instant, period: Duration) -> Option<f32> {
        if self.level != MotionLevel::Instrument || period.is_zero() {
            return None;
        }
        let activity = self.active_activity?;
        let started = activity.started_at();
        if now < started {
            return Some(0.0);
        }
        let elapsed = now.duration_since(started);
        let period_secs = period.as_secs_f32();
        let elapsed_secs = (elapsed.as_secs_f64() % f64::from(period_secs)) as f32;
        Some((elapsed_secs / period_secs).clamp(0.0, 1.0))
    }

    /// Records an observable, healthy daemon status response.
    pub fn record_heartbeat(&mut self, now: Instant) {
        if self.level != MotionLevel::Off {
            self.last_heartbeat_at = Some(now);
        }
    }

    /// Triggers a dominant transient effect, adhering to the single-dominant-motion budget.
    pub fn trigger(&mut self, kind: TransientKind, now: Instant, duration: Duration) {
        if self.level == MotionLevel::Off {
            return;
        }
        if self.level == MotionLevel::Reduced {
            // Reduced keeps local confirmations but drops travelling motion.
            if matches!(
                kind,
                TransientKind::LocationAcquisition { .. }
                    | TransientKind::GlobeRotation { .. }
                    | TransientKind::CurveMorph
                    | TransientKind::WeatherUpdate
            ) {
                return;
            }
        }
        self.active_transient = Some(TransientMotion::new(kind, now, duration));
    }

    /// Returns the normalized phase `[0.0, 1.0]` of the startup masthead sweep, or `None` if completed/disabled.
    #[must_use]
    pub fn masthead_sweep_phase(&self, now: Instant) -> Option<f32> {
        if self.masthead_sweep_done || self.level != MotionLevel::Instrument {
            return None;
        }
        if now < self.session_started_at {
            return Some(0.0);
        }
        let elapsed = now.duration_since(self.session_started_at);
        if elapsed >= self.masthead_sweep_duration {
            None
        } else {
            Some(
                (elapsed.as_secs_f32() / self.masthead_sweep_duration.as_secs_f32())
                    .clamp(0.0, 1.0),
            )
        }
    }

    /// Returns the normalized phase of the live status heartbeat pulse, or `None` if inactive.
    #[must_use]
    pub fn heartbeat_pulse_phase(&self, now: Instant) -> Option<f32> {
        if self.level == MotionLevel::Off {
            return None;
        }
        let last = self.last_heartbeat_at?;
        if now < last {
            return Some(0.0);
        }
        let elapsed = now.duration_since(last);
        if elapsed >= self.heartbeat_pulse_duration {
            None
        } else {
            Some(
                (elapsed.as_secs_f32() / self.heartbeat_pulse_duration.as_secs_f32())
                    .clamp(0.0, 1.0),
            )
        }
    }

    /// Returns `Some((phase, lon, lat))` if a location acquisition ping is currently active.
    #[must_use]
    pub fn location_acquisition_phase(&self, now: Instant) -> Option<(f32, f64, f64)> {
        if let Some(transient) = &self.active_transient {
            if let TransientKind::LocationAcquisition { lon, lat } = transient.kind {
                if let Some(p) = transient.phase(now) {
                    return Some((p, lon, lat));
                }
            }
        }
        None
    }

    /// Returns the active globe rotation phase and its two geographic centers.
    #[must_use]
    pub fn globe_rotation_phase(&self, now: Instant) -> Option<(f32, f64, f64, f64, f64)> {
        if let Some(transient) = &self.active_transient {
            if let TransientKind::GlobeRotation {
                from_lon,
                from_lat,
                to_lon,
                to_lat,
            } = transient.kind
            {
                return transient
                    .phase(now)
                    .map(|phase| (phase, from_lon, from_lat, to_lon, to_lat));
            }
        }
        None
    }

    /// Returns `Some(phase)` if milestone adjustment micro-feedback is active for the given index.
    #[must_use]
    pub fn milestone_adjust_phase(&self, now: Instant, target_index: usize) -> Option<f32> {
        if let Some(transient) = &self.active_transient {
            if let TransientKind::MilestoneAdjust { index } = transient.kind {
                if index == target_index {
                    return transient.phase(now);
                }
            }
        }
        None
    }

    /// Returns `Some(phase)` if a weather update settle confirmation is active.
    #[must_use]
    pub fn weather_update_phase(&self, now: Instant) -> Option<f32> {
        if let Some(transient) = &self.active_transient {
            if matches!(transient.kind, TransientKind::WeatherUpdate) {
                return transient.phase(now);
            }
        }
        None
    }

    /// Starts the tab indicator slide (Full effects only).
    pub fn trigger_tab_slide(&mut self, from: usize, to: usize, now: Instant) {
        if self.level == MotionLevel::Instrument && from != to {
            self.tab_slide = Some((from, to, now));
        }
    }

    /// Interpolated tab indicator position, or `None` once settled.
    #[must_use]
    pub fn tab_slide_position(&self, now: Instant) -> Option<f32> {
        let (from, to, started) = self.tab_slide?;
        let elapsed = now.saturating_duration_since(started);
        if elapsed >= TAB_SLIDE_DURATION {
            return None;
        }
        let t = elapsed.as_secs_f32() / TAB_SLIDE_DURATION.as_secs_f32();
        let eased = 1.0 - (1.0 - t).powi(3);
        Some(from as f32 + (to as f32 - from as f32) * eased)
    }

    /// Phase of the automation curve morph.
    #[must_use]
    pub fn curve_morph_phase(&self, now: Instant) -> Option<f32> {
        let transient = self.active_transient.as_ref()?;
        matches!(transient.kind, TransientKind::CurveMorph)
            .then(|| transient.phase(now))
            .flatten()
    }

    /// Starts the output count for a monitor whose applied value changed.
    pub fn trigger_output_roll(&mut self, logical_id: &str, from: u8, to: u8, now: Instant) {
        if self.level != MotionLevel::Instrument {
            return;
        }
        self.output_roll = Some(OutputRoll {
            logical_id: logical_id.to_owned(),
            from,
            to,
            started_at: now,
        });
    }

    /// The value to show while counting, and the eased phase, for `logical_id`.
    #[must_use]
    pub fn output_roll(&self, logical_id: &str, now: Instant) -> Option<(u8, f32)> {
        let roll = self.output_roll.as_ref()?;
        if roll.logical_id != logical_id || self.level != MotionLevel::Instrument {
            return None;
        }
        let elapsed = now.checked_duration_since(roll.started_at)?;
        if elapsed >= OUTPUT_ROLL_DURATION {
            return None;
        }
        let phase = elapsed.as_secs_f32() / OUTPUT_ROLL_DURATION.as_secs_f32();
        let eased = 1.0 - (1.0 - phase).powi(3);
        let value = f32::from(roll.from) + (f32::from(roll.to) - f32::from(roll.from)) * eased;
        Some((value.round() as u8, phase))
    }

    /// Whether a visual effect is still changing and therefore needs an animation-rate redraw.
    #[must_use]
    pub fn needs_animation_frame(&self, now: Instant) -> bool {
        self.masthead_sweep_phase(now).is_some()
            || self.tab_slide_position(now).is_some()
            || self.heartbeat_pulse_phase(now).is_some()
            || self
                .active_transient
                .as_ref()
                .is_some_and(|transient| transient.phase(now).is_some())
            || (self.level == MotionLevel::Instrument && self.active_activity.is_some())
            || self.output_roll.as_ref().is_some_and(|roll| {
                self.level == MotionLevel::Instrument
                    && now.saturating_duration_since(roll.started_at) < OUTPUT_ROLL_DURATION
            })
    }

    /// Prunes expired transients (called on tick).
    pub fn tick(&mut self, now: Instant) {
        if self.tab_slide_position(now).is_none() {
            self.tab_slide = None;
        }
        if !self.masthead_sweep_done
            && now.duration_since(self.session_started_at) >= self.masthead_sweep_duration
        {
            self.masthead_sweep_done = true;
        }
        if let Some(transient) = &self.active_transient {
            if transient.phase(now).is_none() {
                // The rotation is the dominant transition. Once it has settled
                // at the new center, Full effects get one brief, local
                // acquisition ring; Reduced and Off remain static.
                self.active_transient = match transient.kind {
                    TransientKind::GlobeRotation { to_lon, to_lat, .. }
                        if self.level == MotionLevel::Instrument =>
                    {
                        Some(TransientMotion::new(
                            TransientKind::LocationAcquisition {
                                lon: to_lon,
                                lat: to_lat,
                            },
                            now,
                            Duration::from_millis(220),
                        ))
                    }
                    _ => None,
                };
            }
        }
    }
}

/// Computes the terminal cell style for a specific visual column during an asynchronous shimmer wave.
///
/// Ensures strict geometric invariance: string width and layout remain fixed; only foreground emphasis travels.
#[must_use]
pub fn shimmer_style_for_column(
    col: usize,
    total_cols: usize,
    elapsed: Duration,
    period: Duration,
    styles: &crate::tui::theme::SemanticStyles,
) -> ratatui::style::Style {
    if total_cols == 0 || period.is_zero() {
        return styles.text_muted;
    }
    let cycle_secs = period.as_secs_f32();
    let elapsed_secs = (elapsed.as_secs_f64() % f64::from(cycle_secs)) as f32;
    let phase = elapsed_secs / cycle_secs;

    let band_half_width = 3.0_f32;
    let total_travel = total_cols as f32 + band_half_width * 2.0;
    let center = phase * total_travel - band_half_width;

    let dist = (col as f32 - center).abs();
    if dist <= band_half_width {
        styles.text_heading
    } else {
        styles.text_muted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_globe_rotation_hands_off_to_one_acquisition_pulse() {
        let now = Instant::now();
        let mut motion = UiMotionState::new_at(now);
        motion.trigger(
            TransientKind::GlobeRotation {
                from_lon: 28.9784,
                from_lat: 41.0082,
                to_lon: 139.6917,
                to_lat: 35.6895,
            },
            now,
            Duration::from_millis(450),
        );

        assert!(motion
            .globe_rotation_phase(now + Duration::from_millis(200))
            .is_some());
        motion.tick(now + Duration::from_millis(450));
        assert!(motion
            .globe_rotation_phase(now + Duration::from_millis(450))
            .is_none());
        let (_, lon, lat) = motion
            .location_acquisition_phase(now + Duration::from_millis(450))
            .expect("full rotation should settle with one acquisition pulse");
        assert!((lon - 139.6917).abs() < f64::EPSILON);
        assert!((lat - 35.6895).abs() < f64::EPSILON);

        motion.tick(now + Duration::from_millis(670));
        assert!(motion.active_transient.is_none());
    }

    #[test]
    fn reduced_and_off_do_not_start_globe_rotation() {
        let now = Instant::now();
        for level in [MotionLevel::Reduced, MotionLevel::Off] {
            let mut motion = UiMotionState::new_at(now);
            motion.level = level;
            motion.trigger(
                TransientKind::GlobeRotation {
                    from_lon: 0.0,
                    from_lat: 0.0,
                    to_lon: 10.0,
                    to_lat: 10.0,
                },
                now,
                Duration::from_millis(450),
            );
            assert!(motion.active_transient.is_none());
        }
    }
}
