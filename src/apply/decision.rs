//! Central write-admission policy for every hardware-mutating path.
//!
//! All non-readback write-suppression decisions (topology, disabled, contested,
//! external hold, failure backoff, minimum apply interval, probe correction
//! interval, rolling write budget, manual write rate cap, and fade duplicate
//! avoidance) flow through [`admit_write`]. Readback and delta logic
//! (hysteresis, integrity verification, baseline adoption, hold confirmation)
//! remain in `engine::plan` and `engine::gate`.
//!
//! | Rule | `Tick` | `Probe` | `FadeStep` | `Forced` | Justification |
//! |---|---|---|---|---|---|
//! | Topology | yes | no | yes | yes | `Tick`/`FadeStep`/`Forced` gate on `CapabilitySnapshot`; `Probe` reads verified bus/sysfs directly to catch wake/reconnect before topology refresh. |
//! | Disabled monitor | yes | yes (caller) | yes | yes | Disabled monitors never receive reads or writes. |
//! | Contested back-off | yes | yes | yes (auto; user fade passes) | passes | Stop fighting external DDC controllers automatically; explicit user overrides and user fades still apply. |
//! | External hold | yes | yes (probe hold gate) | yes | yes (cleared by override/resume/clear) | Respect user OSD adjustments; IPC commands that set brightness clear holds up front. |
//! | Failure backoff | yes | no | no (drops on error) | bypass | Ticks back off failing backends; `Probe` tests recovery; `Forced` tries immediately; `FadeStep` cancels on first failure. |
//! | Min interval (`min_apply_interval`) | yes | no (`CorrectionInterval`) | no | bypass (`0s`) | Ticks respect `tick_seconds`; `FadeStep` steps at ~16 ms; `Forced` applies immediately. |
//! | Correction interval (`20s`) | no | yes (unless `in_recovery` or fresh baseline) | no | no | Prevents back-to-back probe rewrites on waking panels. |
//! | Write budget (`12 / 600s`) | yes (unless fresh baseline) | yes (unless fresh baseline) | yes (auto; user fade exempt; collapses to final) | exempt | Caps automatic EEPROM wear; fresh baseline after startup/reconnect gets one write; user actions are exempt. |
//! | Manual cap (`60 / 600s`, `1s` gap) | no | no | no (admitted when queued) | yes | Prevents slider floods from wearing EEPROM while never dropping the latest user value. |
//! | Already matches `last_applied` | via hysteresis | via readback | yes | via baseline read | `FadeStep` skips duplicate rounded steps without consuming write budget. |

use super::health;
use super::types::{
    MAX_WRITES_PER_WINDOW, MIN_CORRECTION_INTERVAL_SECONDS, WRITE_BUDGET_WINDOW_SECONDS,
};
use crate::backends::{BackendError, BackendKind, BackendWrite, FailureKind};
use crate::state::{self, FailureBackoffState, RuntimeState};
use std::time::Duration;

/// Caller path requesting permission to issue a hardware write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePath {
    Tick,
    Probe,
    FadeStep,
    Forced,
}

/// Reason a hardware write was suppressed by [`admit_write`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Topology,
    Disabled,
    Contested {
        until_epoch_s: u64,
    },
    Hold {
        until_epoch_s: u64,
    },
    Backoff {
        remaining_s: u64,
        suppress_until_epoch_s: u64,
        failure_kind: FailureKind,
        consecutive_failures: u32,
    },
    MinInterval {
        elapsed_s: u64,
        min_interval_s: u64,
    },
    CorrectionInterval,
    Budget {
        used: usize,
        resume_at_epoch_s: Option<u64>,
    },
    ManualCap {
        retry_at_epoch_s: u64,
    },
    AlreadyMatches,
}

/// Read-only snapshot of target state and configuration passed to [`admit_write`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct AdmissionFacts<'a> {
    pub logical_id: &'a str,
    pub backend: BackendKind,
    pub enabled: bool,
    pub topology_allowed: bool,
    pub active_hold_until_epoch_s: Option<u64>,
    pub last_applied_percent: Option<u8>,
    pub last_applied_at_epoch_s: Option<u64>,
    pub backoff: Option<&'a FailureBackoffState>,
    pub min_apply_interval: Duration,
    pub in_recovery: bool,
    /// True for a `FadeStep` queued by a user-forced action (`step.budget_exempt`).
    pub budget_exempt: bool,
    /// Target percent for `FadeStep`'s `AlreadyMatches` check.
    pub fade_target_percent: Option<u8>,
    /// Evaluates rate-limit rules (`Backoff`, `MinInterval`, `CorrectionInterval`,
    /// `ManualCap`, `Budget`). Pre-readback checks pass `false` so steady
    /// targets do not trigger budget warnings before hysteresis runs.
    pub check_rate_limits: bool,
}

/// Evaluates whether `path` is permitted to write to `facts.logical_id` at
/// `now_epoch_s`. See the module-level table for per-path rules.
pub(crate) fn admit_write(
    path: WritePath,
    facts: &AdmissionFacts<'_>,
    state: &RuntimeState,
    now_epoch_s: u64,
) -> Result<(), SkipReason> {
    if matches!(
        path,
        WritePath::Tick | WritePath::Forced | WritePath::FadeStep
    ) && !facts.topology_allowed
    {
        return Err(SkipReason::Topology);
    }

    if !facts.enabled {
        return Err(SkipReason::Disabled);
    }

    let bypass_contested = matches!(path, WritePath::Forced)
        || (matches!(path, WritePath::FadeStep) && facts.budget_exempt);
    if !bypass_contested {
        if let Some(until_epoch_s) = health::contested_until(facts.logical_id, now_epoch_s) {
            return Err(SkipReason::Contested { until_epoch_s });
        }
    }

    if let Some(until_epoch_s) = facts
        .active_hold_until_epoch_s
        .filter(|until| *until > now_epoch_s)
    {
        return Err(SkipReason::Hold { until_epoch_s });
    }

    if matches!(path, WritePath::FadeStep)
        && facts
            .fade_target_percent
            .is_some_and(|target| facts.last_applied_percent == Some(target))
    {
        return Err(SkipReason::AlreadyMatches);
    }

    if !facts.check_rate_limits {
        return Ok(());
    }

    if matches!(path, WritePath::Tick) {
        if let Some(backoff) = facts.backoff.filter(|backoff| {
            backoff.backend == facts.backend
                && backoff
                    .suppress_until_epoch_s
                    .is_some_and(|until| until > now_epoch_s)
        }) {
            let suppress_until_epoch_s = backoff.suppress_until_epoch_s.unwrap_or(now_epoch_s);
            let remaining_s = suppress_until_epoch_s.saturating_sub(now_epoch_s);
            return Err(SkipReason::Backoff {
                remaining_s,
                suppress_until_epoch_s,
                failure_kind: backoff.failure_kind,
                consecutive_failures: backoff.consecutive_failures,
            });
        }

        if state::write_interval_active(
            facts.last_applied_at_epoch_s,
            now_epoch_s,
            facts.min_apply_interval,
        ) {
            let elapsed_s =
                now_epoch_s.saturating_sub(facts.last_applied_at_epoch_s.unwrap_or(now_epoch_s));
            return Err(SkipReason::MinInterval {
                elapsed_s,
                min_interval_s: facts.min_apply_interval.as_secs(),
            });
        }
    }

    let fresh_baseline = facts.last_applied_percent.is_none();

    if matches!(path, WritePath::Probe)
        && !fresh_baseline
        && !facts.in_recovery
        && facts
            .last_applied_at_epoch_s
            .is_some_and(|at| now_epoch_s < at.saturating_add(MIN_CORRECTION_INTERVAL_SECONDS))
    {
        return Err(SkipReason::CorrectionInterval);
    }

    if matches!(path, WritePath::Forced) {
        return match health::manual_slot(facts.logical_id, now_epoch_s) {
            Ok(()) => Ok(()),
            Err(retry_at_epoch_s) => Err(SkipReason::ManualCap { retry_at_epoch_s }),
        };
    }

    let exempt_budget = matches!(path, WritePath::FadeStep) && facts.budget_exempt;
    if !exempt_budget && !fresh_baseline {
        let used = state.budget_events_in_window(
            facts.logical_id,
            now_epoch_s,
            WRITE_BUDGET_WINDOW_SECONDS,
        );
        if used >= MAX_WRITES_PER_WINDOW {
            let resume_at_epoch_s = state.budget_resume_at(
                facts.logical_id,
                now_epoch_s,
                WRITE_BUDGET_WINDOW_SECONDS,
                MAX_WRITES_PER_WINDOW,
            );
            return Err(SkipReason::Budget {
                used,
                resume_at_epoch_s,
            });
        }
    }

    Ok(())
}

/// Counts actual `setvcp` / sysfs write attempts after a backend write is
/// attempted and records a rolling budget event only when at least one hardware
/// write attempt occurred (`attempts > 0`).
pub(crate) fn record_attempted_hardware_write(
    state: &mut RuntimeState,
    logical_id: &str,
    now_epoch_s: u64,
    result: &Result<BackendWrite, BackendError>,
    log_tag: &'static str,
) -> u8 {
    let attempts = match result {
        Ok(write) => write.attempts,
        Err(error) => error.attempts(),
    };
    if attempts > 0 {
        let writes_total = state.count_hardware_write_attempts(logical_id, attempts);
        state.record_budget_event(logical_id, now_epoch_s, WRITE_BUDGET_WINDOW_SECONDS);
        tracing::debug!(
            logical_id = %logical_id,
            attempts,
            writes_total,
            "{log_tag}"
        );
    }
    attempts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::BackendKind;

    fn base_facts<'a>() -> AdmissionFacts<'a> {
        AdmissionFacts {
            logical_id: "panel",
            backend: BackendKind::Ddc,
            enabled: true,
            topology_allowed: true,
            active_hold_until_epoch_s: None,
            last_applied_percent: Some(50),
            last_applied_at_epoch_s: Some(900),
            backoff: None,
            min_apply_interval: Duration::from_mins(1),
            in_recovery: false,
            budget_exempt: false,
            fade_target_percent: None,
            check_rate_limits: true,
        }
    }

    #[test]
    fn fade_step_stops_when_automatic_and_contested_but_user_fade_passes() {
        health::reset_for_test();
        for at in [1_000, 1_100, 1_200] {
            health::note_contest_event("panel", at + 10, Some(39), Some(at), 80);
        }
        let state = RuntimeState::default();
        let mut facts = base_facts();
        facts.fade_target_percent = Some(70);

        assert!(matches!(
            admit_write(WritePath::FadeStep, &facts, &state, 1_250),
            Err(SkipReason::Contested { .. })
        ));

        facts.budget_exempt = true;
        assert_eq!(
            admit_write(WritePath::FadeStep, &facts, &state, 1_250),
            Ok(())
        );
        health::reset_for_test();
    }

    #[test]
    fn failed_write_with_zero_attempts_does_not_consume_budget() {
        let mut state = RuntimeState::default();
        let err: Result<BackendWrite, BackendError> = Err(BackendError::MissingSelector {
            backend: BackendKind::Ddc,
            expected: "serial",
        });
        let attempts =
            record_attempted_hardware_write(&mut state, "panel", 1_000, &err, "hardware_write");
        assert_eq!(attempts, 0);
        assert_eq!(state.budget_events_in_window("panel", 1_000, 600), 0);
        assert_eq!(
            state
                .monitor("panel")
                .map_or(0, |monitor| monitor.ledger.writes_total),
            0
        );
    }
}
