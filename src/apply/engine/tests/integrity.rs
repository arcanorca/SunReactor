#[cfg(target_os = "linux")]
use super::super::*;
#[cfg(target_os = "linux")]
use super::support::{fast_settings, test_monitor, test_policy, GETVCP_PANEL};
#[cfg(target_os = "linux")]
use crate::apply::types::ApplyStatus;
#[cfg(target_os = "linux")]
use crate::backends::{testutil::FakeRunner, BackendKind};
#[cfg(target_os = "linux")]
use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use std::time::Duration;

#[test]
#[cfg(target_os = "linux")]
fn due_reassert_reads_matching_hardware_without_writing() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new().with_success(
        "ddcutil",
        &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
        "VCP 10 C 50 100\n",
    );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);

    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    let summary = apply_policy_with_runner_monitors(
        &monitors, settings, &policy, &mut state, None, &runner, 1_120, None,
    );

    assert_eq!(summary.succeeded, 0);
    assert_eq!(summary.skipped, 1);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    assert_eq!(
        state
            .monitor("panel")
            .and_then(|m| m.last_integrity_check_at_epoch_s),
        Some(1_120)
    );
}

#[test]
#[cfg(target_os = "linux")]
fn before_reassert_deadline_hysteresis_still_avoids_read_and_write() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);

    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    let summary = apply_policy_with_runner_monitors(
        &monitors, settings, &policy, &mut state, None, &runner, 1_119, None,
    );

    assert_eq!(summary.records[0].status, ApplyStatus::SkippedHysteresis);
    assert!(runner.calls().is_empty());
}

#[test]
#[cfg(target_os = "linux")]
fn integrity_zero_readback_is_corrected_not_held() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
            "VCP 10 C 0 100\n",
        )
        .with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_panel",
                "setvcp",
                "10",
                "50",
            ],
            "",
        );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::new();
    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    settings.external_change_hold = Duration::from_mins(30);

    let summary = apply_policy_with_runner_monitors_external(
        &monitors, settings, &policy, &mut state, None, &runner, 1_120, None, &mut holds, None,
    );

    assert_eq!(summary.succeeded, 1);
    assert!(holds.is_empty());
    assert!(runner.calls().iter().any(|call| call.contains("|setvcp|")));
}

#[test]
#[cfg(target_os = "linux")]
fn integrity_off_by_one_readback_is_not_rewritten() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 43)]);
    let runner = FakeRunner::new().with_success("ddcutil", &GETVCP_PANEL, "VCP 10 C 44 100\n");
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 43, 1_000);
    let mut holds = BTreeMap::new();
    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    settings.external_change_hold = Duration::from_mins(30);

    let summary = apply_policy_with_runner_monitors_external(
        &monitors, settings, &policy, &mut state, None, &runner, 1_120, None, &mut holds, None,
    );

    assert_eq!(summary.succeeded, 0);
    assert!(holds.is_empty());
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    assert_eq!(
        state
            .monitor("panel")
            .and_then(|m| m.last_integrity_check_at_epoch_s),
        Some(1_120)
    );
}

#[test]
#[cfg(target_os = "linux")]
fn integrity_single_deviating_read_is_a_glitch_not_a_hold() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new()
        .with_success("ddcutil", &GETVCP_PANEL, "VCP 10 C 35 100\n")
        .with_success("ddcutil", &GETVCP_PANEL, "VCP 10 C 50 100\n");
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::new();
    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    settings.external_change_hold = Duration::from_mins(30);

    let summary = apply_policy_with_runner_monitors_external(
        &monitors, settings, &policy, &mut state, None, &runner, 1_120, None, &mut holds, None,
    );

    assert!(holds.is_empty());
    assert_eq!(summary.succeeded, 0);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
}

#[test]
#[cfg(target_os = "linux")]
fn first_write_after_baseline_reset_is_skipped_when_hardware_already_matches() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new().with_success("ddcutil", &GETVCP_PANEL, "VCP 10 C 51 100\n");
    let mut state = RuntimeState::default();
    let mut holds = BTreeMap::new();

    let summary = apply_policy_with_runner_monitors_external(
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

    assert_eq!(summary.succeeded, 0);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    assert_eq!(
        state.monitor("panel").and_then(|m| m.last_applied_percent),
        Some(50)
    );
}

#[test]
#[cfg(target_os = "linux")]
fn unreadable_monitor_is_reasserted_blind_only_rarely() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::new();
    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);

    let early = apply_policy_with_runner_monitors_external(
        &monitors, settings, &policy, &mut state, None, &runner, 1_300, None, &mut holds, None,
    );
    assert_eq!(early.attempted, 0, "no blind rewrite after only 5 minutes");
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
}
