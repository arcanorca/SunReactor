use super::super::probe::ProbeSummary;
use super::super::*;
use super::support::{panel_monitor, probe_panel, test_policy, FakePanel};
use crate::backends::BackendError;
use crate::backends::BackendKind;
use crate::config::MonitorConfig;
use crate::policy::PerMonitorTarget;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[test]
fn wake_probe_rewrites_only_monitors_that_answer_with_the_wrong_value() {
    use crate::backends::{BackendObservation, BackendWrite};
    use std::cell::RefCell;

    let monitor = |id: &str| MonitorConfig {
        logical_id: String::from(id),
        ..MonitorConfig::default()
    };
    let monitors = vec![monitor("right"), monitor("wrong"), monitor("asleep")];
    let target = |id: &str, percent| PerMonitorTarget {
        logical_id: String::from(id),
        percent,
        solar_daylight_factor: 0.0,
        effective_daylight_factor: 0.0,
    };
    let policy = PolicyOutput {
        solar_elevation_deg: 10.0,
        weather_multiplier: 1.0,
        targets: vec![
            target("right", 40),
            target("wrong", 40),
            target("asleep", 40),
        ],
    };
    let mut state = RuntimeState::default();
    let writes = RefCell::new(Vec::new());
    let wrong_now = std::cell::Cell::new(3);
    let summary = probe_and_correct_with(
        &monitors,
        &policy,
        &mut state,
        1_000,
        false,
        |monitor| match monitor.logical_id.as_str() {
            "right" => Ok(Some(BackendObservation { percent: 40 })),
            "wrong" => Ok(Some(BackendObservation {
                percent: wrong_now.get(),
            })),
            _ => Err(BackendError::Io {
                backend: BackendKind::Ddc,
                program: String::from("ddcutil"),
                message: String::from("No monitor detected"),
                attempts: 1,
            }),
        },
        |monitor, _, percent| {
            wrong_now.set(percent);
            writes
                .borrow_mut()
                .push((monitor.logical_id.clone(), percent));
            Ok(BackendWrite {
                backend: BackendKind::Ddc,
                applied_percent: percent,
                attempts: 1,
                detail: String::new(),
            })
        },
    );
    assert_eq!(writes.into_inner(), vec![(String::from("wrong"), 40)]);
    assert_eq!(
        summary,
        ProbeSummary {
            answered: 2,
            corrected: 1,
            failed: 0,
            unreachable: 1,
            reconnected: 0,
        }
    );
    assert_eq!(
        state.monitor("wrong").and_then(|m| m.last_applied_percent),
        Some(40)
    );
    assert!(state.monitor("asleep").is_none_or(|m| m.backoff.is_none()));

    let mut dry_state = RuntimeState::default();
    let dry = probe_and_correct_with(
        &monitors,
        &policy,
        &mut dry_state,
        1_000,
        true,
        |_| Ok(Some(BackendObservation { percent: 3 })),
        |_, _, _| panic!("dry run must not write"),
    );
    assert_eq!(dry.corrected, 0);
    assert_eq!(dry.answered, 3);
}

#[test]
fn zero_readback_after_reset_is_corrected_not_held() {
    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 39)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let panel = FakePanel::new(0);
    let mut holds = BTreeMap::new();
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
    assert_eq!(
        state.monitor("panel").and_then(|m| m.last_applied_percent),
        Some(39)
    );
}

#[test]
fn single_deviating_read_that_settles_is_a_glitch_not_a_hold() {
    let fake = FakePanel::scripted(50, &[20, 50]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 50);

    assert!(holds.is_empty());
    assert_eq!(summary.corrected, 0);
    assert_eq!(fake.writes.get(), 0);
}

#[test]
fn flapping_readbacks_are_never_held_and_are_corrected() {
    let fake = FakePanel::scripted(50, &[20, 80]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 50);

    assert!(holds.is_empty());
    assert_eq!(summary.corrected, 1);
    assert_eq!(fake.percent.get(), 50);
}

#[test]
fn confirmed_deviation_starts_a_hold_without_writing() {
    let fake = FakePanel::scripted(50, &[20, 21]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 50);

    assert_eq!(holds["panel"].observed_percent, 21);
    assert_eq!(summary.corrected, 0);
    assert_eq!(fake.writes.get(), 0);
}

#[test]
fn unreadable_confirmation_skips_without_hold_or_write() {
    let monitor = panel_monitor();
    let reads = std::cell::Cell::new(0);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let mut holds = BTreeMap::new();
    let summary = probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors: std::slice::from_ref(&monitor),
            policy: &test_policy(vec![("panel", 50)]),
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
        |_| {
            reads.set(reads.get() + 1);
            if reads.get() == 1 {
                Ok(Some(crate::backends::BackendObservation { percent: 20 }))
            } else {
                Err(BackendError::Io {
                    backend: BackendKind::Ddc,
                    program: String::from("ddcutil"),
                    message: String::from("busy"),
                    attempts: 1,
                })
            }
        },
        |_, _, _| panic!("must not write on an unconfirmed readback"),
    );

    assert!(holds.is_empty());
    assert_eq!(summary.corrected, 0);
}

#[test]
fn two_percent_off_is_still_corrected_in_recovery() {
    let fake = FakePanel::new(45);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 43, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 43);
    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
}

#[test]
fn startup_without_baseline_never_holds_a_stale_readback() {
    let fake = FakePanel::new(100);
    let mut state = RuntimeState::default();
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);

    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
}

#[test]
fn ignored_correction_is_not_recorded_and_retries_next_probe() {
    let mut fake = FakePanel::new(0);
    fake.ignores_writes = true;
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    let first = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);

    assert_eq!(first.failed, 1);
    assert_eq!(first.corrected, 0);
    assert_eq!(fake.writes.get(), 1, "no retry loop inside one probe");
    assert_eq!(
        state
            .monitor("panel")
            .and_then(|m| m.last_applied_at_epoch_s),
        Some(900)
    );

    fake.ignores_writes = false;
    let second = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);
    assert_eq!(second.corrected, 1);
    assert_eq!(fake.percent.get(), 39);
}

#[test]
fn own_recent_write_with_lagging_readback_is_not_external() {
    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 998);
    let panel = FakePanel::new(20);
    let mut holds = BTreeMap::new();
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

    assert_eq!(summary.corrected, 0);
    assert_eq!(panel.writes.get(), 0);
    assert!(holds.is_empty());
}
