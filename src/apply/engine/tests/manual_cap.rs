#[cfg(target_os = "linux")]
use super::super::*;
#[cfg(target_os = "linux")]
use super::support::{fast_settings, test_monitor, test_policy, GETVCP_PANEL};
use crate::apply::health;
#[cfg(target_os = "linux")]
use crate::apply::types::ApplyStatus;
#[cfg(target_os = "linux")]
use crate::backends::{testutil::FakeRunner, BackendKind};
#[cfg(target_os = "linux")]
use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
fn setvcp_runner(values: &[&str]) -> FakeRunner {
    let mut runner =
        FakeRunner::new().with_success("ddcutil", &["--help"], "--noconfig --noverify");
    for value in values {
        runner = runner.with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_panel",
                "setvcp",
                "10",
                value,
            ],
            "",
        );
    }
    runner
}

#[cfg(target_os = "linux")]
fn forced_apply(
    percent: u8,
    now: u64,
    state: &mut RuntimeState,
    runner: &FakeRunner,
) -> ApplySummary {
    apply_policy_with_runner_monitors_external(
        &[test_monitor("panel", BackendKind::Ddc)],
        fast_settings(),
        &test_policy(vec![("panel", percent)]),
        state,
        None,
        runner,
        now,
        Some(fast_settings()),
        &mut BTreeMap::new(),
        None,
    )
}

#[test]
#[cfg(target_os = "linux")]
fn forced_writes_are_rate_capped_and_the_latest_value_applies_later() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 100);
    let runner = setvcp_runner(&["60", "70"]);

    let first = forced_apply(60, 1_000, &mut state, &runner);
    assert_eq!(first.succeeded, 1);
    let second = forced_apply(65, 1_000, &mut state, &runner);
    let third = forced_apply(70, 1_000, &mut state, &runner);
    for deferred in [&second, &third] {
        assert_eq!(deferred.succeeded, 0);
        assert_eq!(deferred.records[0].status, ApplyStatus::SkippedManualCap);
        assert_eq!(deferred.records[0].backoff_until_epoch_s, Some(1_001));
    }
    assert_eq!(health::next_deferred_manual_at(1_000), Some(1_001));
    let setvcps = |r: &FakeRunner| r.calls().iter().filter(|c| c.contains("|setvcp|")).count();
    assert_eq!(setvcps(&runner), 1);

    let later = forced_apply(70, 1_001, &mut state, &runner);
    assert_eq!(later.succeeded, 1);
    assert_eq!(setvcps(&runner), 2);
    assert_eq!(health::next_deferred_manual_at(1_001), None);
    health::reset_for_test();
}

#[test]
fn forced_writes_stop_at_sixty_per_window() {
    health::reset_for_test();
    for second in 0..60_u64 {
        assert!(health::manual_slot("panel", 1_000 + second * 2).is_ok());
        health::record_manual_write("panel", 1_000 + second * 2);
    }
    assert_eq!(health::manual_slot("panel", 1_200), Err(1_600));
    assert!(health::manual_slot("panel", 1_600).is_ok());
    health::reset_for_test();
}

#[test]
#[cfg(target_os = "linux")]
fn forced_apply_of_the_value_already_written_costs_one_read_and_no_write() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 100);
    let runner = FakeRunner::new().with_success("ddcutil", &GETVCP_PANEL, "VCP 10 C 50 100\n");
    let mut settings = fast_settings();
    settings.min_write_delta_pct = 0;
    let summary = apply_policy_with_runner_monitors_external(
        &[test_monitor("panel", BackendKind::Ddc)],
        settings,
        &test_policy(vec![("panel", 50)]),
        &mut state,
        None,
        &runner,
        1_000,
        Some(settings),
        &mut BTreeMap::new(),
        None,
    );
    assert_eq!(summary.succeeded, 0);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    health::reset_for_test();
}
