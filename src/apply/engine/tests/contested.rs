use super::super::*;
use super::support::{fast_settings, probe_at, test_monitor, test_policy, FakePanel};
use crate::apply::health;
use crate::backends::{testutil::FakeRunner, BackendKind};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[test]
fn another_controller_undoing_our_writes_makes_the_monitor_contested() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    let mut writes = 0;
    for round in 0..4_u64 {
        let wrote_at = 1_000 + round * 100;
        state.record_apply_success("panel", 39, wrote_at);
        let fight = FakePanel::new(80);
        // Hold detection off (0 minutes): the legacy correct-every-probe path.
        probe_at(
            Some(&fight),
            &mut state,
            &mut holds,
            &mut down,
            39,
            wrote_at + 25,
            None,
            Duration::ZERO,
        );
        writes += fight.writes.get();
    }
    assert_eq!(writes, 2, "the third undone write stops the corrections");
    assert!(health::is_contested("panel", 1_400));
    let listed = health::contested_monitors(1_400);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].logical_id, "panel");
    assert!(!health::is_contested("panel", 1_400 + 7 * 3600));
    health::clear_contested("panel");
    assert!(health::contested_monitors(1_400).is_empty());
}

#[test]
fn one_undone_write_seen_by_many_probes_counts_once() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    state.record_apply_success("panel", 39, 1_000);
    for probe in 0..5_u64 {
        let fight = FakePanel::new(80);
        probe_at(
            Some(&fight),
            &mut state,
            &mut holds,
            &mut down,
            39,
            1_010 + probe,
            None,
            Duration::ZERO,
        );
    }
    assert!(!health::is_contested("panel", 1_020));
}

#[test]
fn contested_monitor_is_skipped_by_automatic_apply_but_not_by_the_user() {
    health::reset_for_test();
    for at in [1_000, 1_100, 1_200] {
        health::note_contest_event("panel", at + 10, Some(39), Some(at), 80);
    }
    assert!(health::is_contested("panel", 1_220));
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 55)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 100);
    let mut holds = BTreeMap::new();
    let runner = FakeRunner::new();
    let auto = apply_policy_with_runner_monitors_external(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        None,
        &runner,
        1_220,
        None,
        &mut holds,
        None,
    );
    assert_eq!(auto.succeeded + auto.failed, 0);
    assert!(auto.records[0].detail.contains("contested"));
    assert!(runner.calls().is_empty(), "hands off: not even a read");
    health::reset_for_test();
}
