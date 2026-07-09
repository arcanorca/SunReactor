use std::collections::BTreeMap;

const FADE_THRESHOLD_PCT: u8 = 10;
const FADE_STEPS_DECREASE: i32 = 25;
const FADE_STEPS_INCREASE: i32 = 40;

#[derive(Debug, Clone)]
pub struct ActiveFade {
    pub start_percent: u8,
    pub target_percent: u8,
    pub steps_total: i32,
    pub current_step: i32,
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
        let diff = (i32::from(target_percent) - i32::from(start_percent)).abs();
        if diff >= i32::from(FADE_THRESHOLD_PCT) {
            let steps_total = if target_percent < start_percent {
                FADE_STEPS_DECREASE
            } else {
                FADE_STEPS_INCREASE
            };

            self.active_fades.insert(
                monitor_id.to_string(),
                ActiveFade {
                    start_percent,
                    target_percent,
                    steps_total,
                    current_step: 0,
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

    /// Process one tick of the fade engine.
    /// Returns a list of intermediate targets to apply.
    pub fn process_tick(&mut self) -> Vec<(String, u8)> {
        let mut to_apply = Vec::new();
        let mut completed = Vec::new();

        for (monitor_id, fade) in &mut self.active_fades {
            fade.current_step += 1;

            if fade.current_step >= fade.steps_total {
                // Final step - just set to target
                to_apply.push((monitor_id.clone(), fade.target_percent));
                completed.push(monitor_id.clone());
            } else {
                // Intermediate step
                let intermediate = i32::from(fade.start_percent)
                    + ((i32::from(fade.target_percent) - i32::from(fade.start_percent))
                        * fade.current_step)
                        / fade.steps_total;

                let intermediate = intermediate.clamp(0, 100) as u8;
                to_apply.push((monitor_id.clone(), intermediate));
            }
        }

        // Remove completed fades
        for monitor_id in completed {
            self.active_fades.remove(&monitor_id);
        }

        to_apply
    }

    pub fn cancel_all(&mut self) {
        self.active_fades.clear();
    }
}
