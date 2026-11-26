use std::collections::BTreeMap;

const FADE_THRESHOLD_PCT: u8 = 10;
/// Write economy: every fade step is a DDC `setvcp`. One step per this many
/// percentage points (rounded up) keeps a fade visibly gradual while writing
/// as little as possible; the old fixed 25/40 steps rewrote the same integer
/// value several times for a 10-point fade.
const FADE_PCT_PER_STEP: i32 = 4;
const FADE_MIN_STEPS: i32 = 3;
/// Upper bound on writes of one fade, however large the swing (100 points
/// still finishes in 12 writes instead of 40).
const FADE_MAX_STEPS: i32 = 12;

#[derive(Debug, Clone)]
pub struct ActiveFade {
    pub start_percent: u8,
    pub target_percent: u8,
    pub steps_total: i32,
    pub current_step: i32,
    /// Last value handed out, so a step that rounds to it is not rewritten.
    pub last_emitted: u8,
    /// Explicit user actions are exempt from the rolling write budget.
    pub budget_exempt: bool,
    /// While set and in the future the fade is parked (write budget spent):
    /// it neither steps nor keeps the fast fade cadence alive.
    pub resume_at_epoch_s: Option<u64>,
}

/// One fade write handed out by [`FadeEngine::process_tick_steps`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FadeStep {
    pub monitor_id: String,
    pub percent: u8,
    /// The last step of the fade: `percent` is the final target.
    pub is_final: bool,
    pub budget_exempt: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FadeEngine {
    pub active_fades: BTreeMap<String, ActiveFade>,
}

impl FadeEngine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues a fade if the difference exceeds the threshold.
    /// Returns true if queued, false if no fade is needed.
    pub fn maybe_enqueue(
        &mut self,
        monitor_id: &str,
        start_percent: u8,
        target_percent: u8,
    ) -> bool {
        self.maybe_enqueue_with(monitor_id, start_percent, target_percent, false)
    }

    /// Like [`Self::maybe_enqueue`]; `budget_exempt` marks a fade caused by an
    /// explicit user action, whose steps never wait for the write budget.
    /// A newer fade (or a cancel) fully replaces the old one, so steps of a
    /// superseded fade are never produced again.
    pub fn maybe_enqueue_with(
        &mut self,
        monitor_id: &str,
        start_percent: u8,
        target_percent: u8,
        budget_exempt: bool,
    ) -> bool {
        let diff = (i32::from(target_percent) - i32::from(start_percent)).abs();
        if diff >= i32::from(FADE_THRESHOLD_PCT) {
            let steps_total = ((diff + FADE_PCT_PER_STEP - 1) / FADE_PCT_PER_STEP)
                .clamp(FADE_MIN_STEPS, FADE_MAX_STEPS);

            self.active_fades.insert(
                monitor_id.to_string(),
                ActiveFade {
                    start_percent,
                    target_percent,
                    steps_total,
                    current_step: 0,
                    last_emitted: start_percent,
                    budget_exempt,
                    resume_at_epoch_s: None,
                },
            );
            true
        } else {
            // Remove any existing fade for this monitor since we're overriding it
            self.active_fades.remove(monitor_id);
            false
        }
    }

    #[must_use]
    pub fn is_fading(&self) -> bool {
        !self.active_fades.is_empty()
    }

    /// True when some fade is due to write now (not parked on the budget).
    #[must_use]
    pub fn is_stepping(&self, now_epoch_s: u64) -> bool {
        self.active_fades
            .values()
            .any(|fade| fade.resume_at_epoch_s.is_none_or(|at| at <= now_epoch_s))
    }

    /// Process one tick of the fade engine.
    /// Returns a list of intermediate targets to apply.
    pub fn process_tick(&mut self) -> Vec<(String, u8)> {
        self.process_tick_steps(u64::MAX)
            .into_iter()
            .map(|step| (step.monitor_id, step.percent))
            .collect()
    }

    /// Advances every fade that is due at `now_epoch_s` by one step.
    pub fn process_tick_steps(&mut self, now_epoch_s: u64) -> Vec<FadeStep> {
        let mut to_apply = Vec::new();
        let mut completed = Vec::new();

        for (monitor_id, fade) in &mut self.active_fades {
            if fade.resume_at_epoch_s.is_some_and(|at| at > now_epoch_s) {
                continue;
            }
            fade.resume_at_epoch_s = None;
            fade.current_step += 1;

            if fade.current_step >= fade.steps_total {
                // Final step - just set to target
                if fade.target_percent != fade.last_emitted {
                    to_apply.push(FadeStep {
                        monitor_id: monitor_id.clone(),
                        percent: fade.target_percent,
                        is_final: true,
                        budget_exempt: fade.budget_exempt,
                    });
                }
                completed.push(monitor_id.clone());
            } else {
                // Intermediate step
                let intermediate = i32::from(fade.start_percent)
                    + ((i32::from(fade.target_percent) - i32::from(fade.start_percent))
                        * fade.current_step)
                        / fade.steps_total;

                let intermediate = intermediate.clamp(0, 100) as u8;
                if intermediate != fade.last_emitted {
                    fade.last_emitted = intermediate;
                    to_apply.push(FadeStep {
                        monitor_id: monitor_id.clone(),
                        percent: intermediate,
                        is_final: false,
                        budget_exempt: fade.budget_exempt,
                    });
                }
            }
        }

        // Remove completed fades
        for monitor_id in completed {
            self.active_fades.remove(&monitor_id);
        }

        to_apply
    }

    /// Collapses a fade whose write budget is spent: every intermediate step
    /// is skipped and only the final `target_percent` remains, parked until
    /// `resume_at_epoch_s`. `last_written` is the value on the hardware. The
    /// final value is never dropped.
    pub fn park_final(
        &mut self,
        monitor_id: &str,
        last_written: u8,
        target_percent: u8,
        resume_at_epoch_s: u64,
    ) {
        self.active_fades.insert(
            monitor_id.to_string(),
            ActiveFade {
                start_percent: last_written,
                target_percent,
                steps_total: 1,
                current_step: 0,
                last_emitted: last_written,
                budget_exempt: false,
                resume_at_epoch_s: Some(resume_at_epoch_s),
            },
        );
    }

    pub fn cancel_all(&mut self) {
        self.active_fades.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(engine: &mut FadeEngine) -> Vec<u8> {
        let mut writes = Vec::new();
        while engine.is_fading() {
            writes.extend(engine.process_tick().into_iter().map(|(_, p)| p));
        }
        writes
    }

    #[test]
    fn fade_step_count_is_bounded_and_values_are_distinct() {
        for (start, target) in [(50, 60), (50, 100), (0, 100), (100, 0), (30, 41)] {
            let mut engine = FadeEngine::new();
            assert!(engine.maybe_enqueue("m", start, target));
            let writes = drain(&mut engine);
            assert!(writes.len() <= FADE_MAX_STEPS as usize, "{start}->{target}");
            assert_eq!(writes.last(), Some(&target));
            assert!(
                writes.windows(2).all(|pair| pair[0] != pair[1]),
                "duplicate consecutive value {start}->{target}: {writes:?}"
            );
        }
    }

    #[test]
    fn ten_point_fade_needs_only_three_writes() {
        let mut engine = FadeEngine::new();
        assert!(engine.maybe_enqueue("m", 50, 60));
        assert_eq!(drain(&mut engine).len(), 3);
    }

    #[test]
    fn newer_target_supersedes_the_running_fade() {
        let mut engine = FadeEngine::new();
        assert!(engine.maybe_enqueue("m", 50, 100));
        let _ = engine.process_tick();
        assert!(engine.maybe_enqueue("m", 58, 20));
        assert_eq!(engine.active_fades.len(), 1);
        assert_eq!(drain(&mut engine).last(), Some(&20));
    }

    #[test]
    fn superseded_fade_never_emits_its_old_steps() {
        let mut engine = FadeEngine::new();
        assert!(engine.maybe_enqueue("m", 50, 100));
        let first = engine.process_tick();
        assert_eq!(first.len(), 1);
        // Replacement fade heads the other way; nothing of the old one may
        // appear afterwards.
        assert!(engine.maybe_enqueue("m", first[0].1, 10));
        let rest = drain(&mut engine);
        assert!(rest.iter().all(|&p| p < first[0].1), "{rest:?}");
        assert_eq!(rest.last(), Some(&10));
    }

    #[test]
    fn parked_final_waits_then_emits_only_the_target() {
        let mut engine = FadeEngine::new();
        engine.park_final("m", 40, 80, 1_000);
        assert!(!engine.is_stepping(999));
        assert!(engine.process_tick_steps(999).is_empty());
        assert!(engine.is_fading());
        assert!(engine.is_stepping(1_000));
        let steps = engine.process_tick_steps(1_000);
        assert_eq!(steps.len(), 1);
        assert_eq!((steps[0].percent, steps[0].is_final), (80, true));
        assert!(!engine.is_fading());
    }
}
