use super::super::*;
use super::support::{fast_settings, probe_panel, test_monitor, test_policy, FakePanel};
use crate::apply::types::ApplyStatus;
use crate::backends::{testutil::FakeRunner, BackendKind};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn probe_corrections_stop_at_the_rolling_budget() {
    let fake = FakePanel::new(100);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    for at in 0..crate::apply::types::MAX_WRITES_PER_WINDOW as u64 {
        state.record_budget_event("panel", 900 + at, 600);
    }
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);

    assert_eq!(summary.corrected, 0);
    assert_eq!(fake.writes.get(), 0);
}

#[test]
#[cfg(target_os = "linux")]
fn tick_write_over_budget_is_deferred_but_manual_override_still_writes() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 55)]);
    let setvcp = |value: &str| {
        FakeRunner::new()
            .with_success("ddcutil", &["--help"], "--noconfig --noverify")
            .with_success(
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
            )
    };
    let fresh_state = || {
        let mut state = RuntimeState::default();
        state.record_apply_success("panel", 50, 100);
        for at in 0..crate::apply::types::MAX_WRITES_PER_WINDOW as u64 {
            state.record_budget_event("panel", 990 + at, 600);
        }
        state
    };
    let mut holds = BTreeMap::new();

    let mut state = fresh_state();
    let runner = setvcp("55");
    let deferred = apply_policy_with_runner_monitors_external(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        None,
        &runner,
        1_000,
        None,
        &mut holds,
        None,
    );
    assert_eq!(deferred.succeeded, 0);
    assert_eq!(deferred.records[0].status, ApplyStatus::SkippedBudget);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));

    let mut state = fresh_state();
    let runner = setvcp("55");
    let manual = apply_policy_with_runner_monitors_external(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        None,
        &runner,
        1_000,
        Some(fast_settings()),
        &mut holds,
        None,
    );
    assert_eq!(manual.succeeded, 1);
}
