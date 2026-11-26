//! Fade steps count against the rolling write budget and collapse to the
//! final target when it is spent; user (IPC) fades are exempt.

use super::*;
use crate::apply::types::{MAX_WRITES_PER_WINDOW, WRITE_BUDGET_WINDOW_SECONDS};
use crate::runtime::fade::FadeStep;
use crate::runtime::topology::reconcile;

const NOW: u64 = 1_800_000_000;

fn utc(epoch: u64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(epoch as i64, 0)
        .single()
        .expect("valid time")
}

fn writes(runner: &RecordingRunner) -> Vec<String> {
    runner
        .calls()
        .into_iter()
        .filter(|call| call.contains("|set|"))
        .collect()
}

fn runtime_with_fade(temp: &TempDir, exempt: bool) -> DaemonRuntime {
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    runtime.last_capabilities = Some(test_capability_snapshot());
    runtime.state.record_apply_success("internal", 20, NOW - 1);
    assert!(runtime
        .fade_engine
        .maybe_enqueue_with("internal", 20, 80, exempt));
    runtime
}

fn spend_budget(runtime: &mut DaemonRuntime) {
    for index in 0..MAX_WRITES_PER_WINDOW as u64 {
        runtime.state.record_budget_event(
            "internal",
            NOW - 100 + index,
            WRITE_BUDGET_WINDOW_SECONDS,
        );
    }
}

fn step(percent: u8, is_final: bool, budget_exempt: bool) -> FadeStep {
    FadeStep {
        monitor_id: String::from("internal"),
        percent,
        is_final,
        budget_exempt,
    }
}

fn apply(runtime: &mut DaemonRuntime, runner: &RecordingRunner, step: &FadeStep, now: u64) {
    let caps = runtime.last_capabilities.clone().expect("capabilities");
    let actions = reconcile(&runtime.config.monitors, &caps);
    runtime.apply_fade_step(step, utc(now), runner, &actions);
}

#[test]
fn exhausted_budget_collapses_fade_and_writes_only_the_final_target_later() {
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, false);
    spend_budget(&mut runtime);
    let runner = RecordingRunner::new();

    apply(&mut runtime, &runner, &step(30, false, false), NOW);

    assert!(writes(&runner).is_empty(), "no intermediate write");
    let parked = runtime.fade_engine.active_fades["internal"].clone();
    assert_eq!((parked.target_percent, parked.steps_total), (80, 1));
    let resume_at = parked.resume_at_epoch_s.expect("parked");
    assert_eq!(resume_at, NOW - 100 + WRITE_BUDGET_WINDOW_SECONDS);
    assert!(!runtime.fade_engine.is_stepping(NOW));

    // Still parked: nothing is produced before the budget frees up.
    assert!(runtime
        .fade_engine
        .process_tick_steps(resume_at - 1)
        .is_empty());

    let mut steps = runtime.fade_engine.process_tick_steps(resume_at);
    assert_eq!(steps.len(), 1);
    let final_step = steps.remove(0);
    assert_eq!((final_step.percent, final_step.is_final), (80, true));
    apply(&mut runtime, &runner, &final_step, resume_at);
    let calls = writes(&runner);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].ends_with("80%"), "{calls:?}");
    assert!(!runtime.fade_engine.is_fading());
}

#[test]
fn exhausted_budget_never_drops_a_final_step() {
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, false);
    spend_budget(&mut runtime);
    let runner = RecordingRunner::new();
    runtime.fade_engine.cancel_all();

    apply(&mut runtime, &runner, &step(80, true, false), NOW);

    assert!(writes(&runner).is_empty());
    assert_eq!(
        runtime.fade_engine.active_fades["internal"].target_percent,
        80
    );
}

#[test]
fn every_fade_step_is_counted_in_the_budget_and_ledger() {
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, false);
    let runner = RecordingRunner::new();

    apply(&mut runtime, &runner, &step(35, false, false), NOW);
    apply(&mut runtime, &runner, &step(50, false, false), NOW + 1);

    assert_eq!(writes(&runner).len(), 2);
    assert_eq!(
        runtime
            .state
            .budget_events_in_window("internal", NOW + 1, WRITE_BUDGET_WINDOW_SECONDS),
        2
    );
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .unwrap()
            .ledger
            .writes_total,
        2
    );
}

#[test]
fn user_fades_are_exempt_from_the_write_budget() {
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, true);
    spend_budget(&mut runtime);
    let runner = RecordingRunner::new();

    apply(&mut runtime, &runner, &step(35, false, true), NOW);

    assert_eq!(writes(&runner).len(), 1);
}

#[test]
fn fade_step_for_unknown_or_unsafe_monitor_is_never_written() {
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, false);
    let runner = RecordingRunner::new();

    // Monitor vanished from the config.
    runtime.fade_engine.active_fades.insert(
        String::from("ghost"),
        runtime.fade_engine.active_fades["internal"].clone(),
    );
    apply(
        &mut runtime,
        &runner,
        &FadeStep {
            monitor_id: String::from("ghost"),
            ..step(35, false, false)
        },
        NOW,
    );
    assert!(!runtime.fade_engine.active_fades.contains_key("ghost"));

    // Topology no longer shows the backlight device.
    runtime.last_capabilities = Some(CapabilitySnapshot {
        ddc_present: Vec::new(),
        backlights_present: Vec::new(),
    });
    apply(&mut runtime, &runner, &step(35, false, false), NOW);

    assert!(writes(&runner).is_empty());
    assert!(!runtime.fade_engine.active_fades.contains_key("internal"));
}

#[test]
fn automatic_fade_step_stops_and_drops_when_monitor_is_contested() {
    crate::apply::health::reset_for_test();
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, false);
    let runner = RecordingRunner::new();
    for at in [NOW - 300, NOW - 200, NOW - 100] {
        crate::apply::health::note_contest_event("internal", at + 10, Some(20), Some(at), 80);
    }
    assert!(crate::apply::is_contested("internal", NOW));

    apply(&mut runtime, &runner, &step(35, false, false), NOW);

    assert!(writes(&runner).is_empty());
    assert!(!runtime.fade_engine.active_fades.contains_key("internal"));
    crate::apply::health::reset_for_test();
}

#[test]
fn failed_fade_step_with_zero_attempts_consumes_no_budget() {
    let temp = TempDir::new();
    let mut runtime = runtime_with_fade(&temp, false);
    // Missing sysfs_path causes BackendError::MissingSelector with attempts == 0.
    runtime.config.monitors[0].selector.sysfs_path = None;
    let runner = RecordingRunner::new();

    apply(&mut runtime, &runner, &step(35, false, false), NOW);

    assert_eq!(
        runtime
            .state
            .budget_events_in_window("internal", NOW, WRITE_BUDGET_WINDOW_SECONDS),
        0
    );
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .map_or(0, |monitor| monitor.ledger.writes_total),
        0
    );
    assert!(!runtime.fade_engine.active_fades.contains_key("internal"));
}
