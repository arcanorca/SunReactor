use super::super::probe::ProbeSummary;
use super::super::*;
use super::support::{fast_settings, probe_panel, test_monitor, test_policy, FakePanel};
use crate::apply::types::ApplyStatus;
use crate::backends::{testutil::FakeRunner, BackendKind};
use crate::config::MonitorConfig;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[test]
#[cfg(target_os = "linux")]
fn external_change_outside_recovery_window_sets_hold_without_writing() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
            "VCP 10 C 35 100\n",
        )
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
            "VCP 10 C 35 100\n",
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

    assert_eq!(summary.succeeded, 0);
    assert_eq!(summary.records[0].status, ApplyStatus::SkippedHysteresis);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    assert_eq!(holds["panel"].observed_percent, 35);
}

#[test]
#[cfg(target_os = "linux")]
fn external_change_within_recovery_window_is_corrected() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
            "VCP 10 C 35 100\n",
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
        &monitors,
        settings,
        &policy,
        &mut state,
        None,
        &runner,
        1_120,
        None,
        &mut holds,
        Some(1_200),
    );

    assert_eq!(summary.succeeded, 1);
    assert!(holds.is_empty());
    assert!(runner.calls().iter().any(|call| call.contains("|setvcp|")));
}

#[test]
fn external_hold_expiry_starts_fade_from_current_readback() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 80)]);
    let runner = FakeRunner::new().with_success(
        "ddcutil",
        &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
        "VCP 10 C 35 100\n",
    );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 1_100,
            observed_percent: 35,
        },
    )]);
    let mut fade_engine = crate::runtime::fade::FadeEngine::new();
    let mut settings = fast_settings();
    settings.external_change_hold = Duration::from_mins(30);

    let summary = apply_policy_with_runner_monitors_external(
        &monitors,
        settings,
        &policy,
        &mut state,
        Some(&mut fade_engine),
        &runner,
        1_120,
        None,
        &mut holds,
        None,
    );

    assert_eq!(summary.succeeded, 1);
    assert!(holds.is_empty());
    assert_eq!(fade_engine.active_fades["panel"].start_percent, 35);
    assert_eq!(fade_engine.active_fades["panel"].target_percent, 80);
    assert!(runner.calls().iter().any(|call| call.contains("getvcp")));
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
}

#[test]
fn external_hold_readback_change_extends_until_and_updates_observation() {
    use crate::backends::{BackendObservation, BackendWrite};

    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 1_800,
            observed_percent: 35,
        },
    )]);
    let summary = probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors: std::slice::from_ref(&monitor),
            policy: &policy,
            now_epoch_s: 1_000,
            dry_run: false,
            external_change_hold: Duration::from_mins(30),
            recovery_until_epoch_s: None,
        },
        ProbeSinks {
            state: &mut state,
            external_holds: Some(&mut holds),
            fade_engine: None,
            unreachable_monitors: None,
        },
        |_| Ok(Some(BackendObservation { percent: 20 })),
        |_, _, _| {
            Ok(BackendWrite {
                backend: BackendKind::Ddc,
                applied_percent: 50,
                attempts: 1,
                detail: String::new(),
            })
        },
    );

    assert_eq!(summary.corrected, 0);
    assert_eq!(holds["panel"].observed_percent, 20);
    assert_eq!(holds["panel"].until_epoch_s, 2_800);
}

#[test]
fn hold_minutes_zero_keeps_legacy_correction() {
    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let panel = FakePanel::new(20);
    let mut holds = BTreeMap::new();
    let summary = probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors: std::slice::from_ref(&monitor),
            policy: &policy,
            now_epoch_s: 1_000,
            dry_run: false,
            external_change_hold: Duration::ZERO,
            recovery_until_epoch_s: None,
        },
        ProbeSinks {
            state: &mut state,
            external_holds: Some(&mut holds),
            fade_engine: None,
            unreachable_monitors: None,
        },
        |m| panel.read(m),
        |m, t, requested| panel.write(m, t, requested),
    );

    assert_eq!(summary.corrected, 1);
    assert_eq!(panel.writes.get(), 1);
    assert!(holds.is_empty());
}

#[test]
fn zero_readback_under_active_hold_releases_hold_and_restores() {
    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let panel = FakePanel::new(0);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 1_800,
            observed_percent: 70,
        },
    )]);
    let summary = probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors: std::slice::from_ref(&monitor),
            policy: &policy,
            now_epoch_s: 1_000,
            dry_run: false,
            external_change_hold: Duration::from_mins(30),
            recovery_until_epoch_s: None,
        },
        ProbeSinks {
            state: &mut state,
            external_holds: Some(&mut holds),
            fade_engine: None,
            unreachable_monitors: None,
        },
        |m| panel.read(m),
        |m, t, requested| panel.write(m, t, requested),
    );

    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
}

fn probe_with_active_hold(fake: &FakePanel) -> (BTreeMap<String, ExternalHold>, ProbeSummary) {
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 1_800,
            observed_percent: 35,
        },
    )]);
    let mut down = BTreeSet::new();
    let summary = probe_panel(fake, &mut state, &mut holds, &mut down, 50);
    (holds, summary)
}

#[test]
fn flapping_readbacks_never_extend_an_active_hold() {
    let fake = FakePanel::scripted(35, &[20, 80]);
    let (holds, summary) = probe_with_active_hold(&fake);
    assert_eq!(holds["panel"].observed_percent, 35);
    assert_eq!(holds["panel"].until_epoch_s, 1_800);
    assert_eq!(summary.corrected, 0);
    assert_eq!(fake.writes.get(), 0);
}

#[test]
fn single_deviating_read_that_settles_back_leaves_the_hold_untouched() {
    let fake = FakePanel::scripted(35, &[20, 35]);
    let (holds, _) = probe_with_active_hold(&fake);
    assert_eq!(holds["panel"].observed_percent, 35);
    assert_eq!(holds["panel"].until_epoch_s, 1_800);
    assert_eq!(fake.writes.get(), 0);
}

#[test]
fn confirmed_deviation_extends_an_active_hold_to_the_confirmed_value() {
    let fake = FakePanel::scripted(21, &[20]);
    let (holds, _) = probe_with_active_hold(&fake);
    assert_eq!(holds["panel"].observed_percent, 21);
    assert_eq!(holds["panel"].until_epoch_s, 2_800);
    assert_eq!(fake.writes.get(), 0);
}
