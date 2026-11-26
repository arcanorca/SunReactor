use super::super::*;
use super::support::{held, probe_at, probe_panel, FakePanel, HOLD};
use crate::apply::health;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn reconnect_with_factory_value_is_corrected_not_held() {
    let fake = FakePanel::new(100);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let mut holds = BTreeMap::new();
    let mut down = BTreeSet::from([String::from("panel")]);

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);

    assert_eq!(summary.reconnected, 1);
    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
    assert!(down.is_empty());
    assert_eq!(fake.percent.get(), 39);
}

#[test]
fn reconnect_discards_a_stale_hold() {
    let fake = FakePanel::new(100);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 2_700,
            observed_percent: 70,
        },
    )]);
    let mut down = BTreeSet::from([String::from("panel")]);

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);

    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
}

#[test]
fn reconnect_matching_target_reestablishes_baseline_without_write() {
    let fake = FakePanel::new(39);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 70, 900);
    let mut holds = BTreeMap::new();
    let mut down = BTreeSet::from([String::from("panel")]);

    let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 39);

    assert_eq!(summary.corrected, 0);
    assert_eq!(fake.writes.get(), 0);
    assert_eq!(
        state.monitor("panel").and_then(|m| m.last_applied_percent),
        Some(39)
    );
}

#[test]
fn single_failed_read_never_drops_a_hold_or_baseline() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let mut holds = held(70);
    let mut down = BTreeSet::new();

    let failed = probe_at(
        None, &mut state, &mut holds, &mut down, 39, 1_000, None, HOLD,
    );
    assert_eq!(failed.unreachable, 1);
    assert!(
        down.is_empty(),
        "one transient error is not a disconnection"
    );

    let fake = FakePanel::new(30);
    let back = probe_at(
        Some(&fake),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_015,
        None,
        HOLD,
    );
    assert_eq!(back.reconnected, 0);
    assert_eq!(
        fake.writes.get(),
        0,
        "the user's OSD value is not overwritten"
    );
    assert!(
        holds.contains_key("panel"),
        "the hold survives a transient error"
    );
    assert_eq!(
        state.monitor("panel").and_then(|m| m.last_applied_percent),
        Some(39)
    );
}

#[test]
fn failed_reads_arm_reconnect_only_after_count_and_span() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    for now in [1_000, 1_010, 1_020] {
        probe_at(None, &mut state, &mut holds, &mut down, 39, now, None, HOLD);
    }
    assert!(down.is_empty(), "three failures in 20 s are still noise");
    probe_at(
        None, &mut state, &mut holds, &mut down, 39, 1_030, None, HOLD,
    );
    assert!(
        down.contains("panel"),
        "3+ failures over 30 s are a disconnection"
    );

    let fake = FakePanel::new(100);
    let awake = probe_at(
        Some(&fake),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_045,
        None,
        HOLD,
    );
    assert_eq!(awake.reconnected, 1);
    assert_eq!(awake.corrected, 1);
}

#[test]
fn a_success_between_failures_resets_the_streak() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    let fake = FakePanel::new(39);
    probe_at(
        None, &mut state, &mut holds, &mut down, 39, 1_000, None, HOLD,
    );
    probe_at(
        None, &mut state, &mut holds, &mut down, 39, 1_020, None, HOLD,
    );
    probe_at(
        Some(&fake),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_030,
        None,
        HOLD,
    );
    probe_at(
        None, &mut state, &mut holds, &mut down, 39, 1_060, None, HOLD,
    );
    assert!(down.is_empty());
}

#[test]
fn powered_off_connector_arms_reconnect_on_the_first_failed_read() {
    health::reset_for_test();
    health::set_powered_off_for_test(Some(true));
    let mut state = RuntimeState::default();
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    probe_at(
        None, &mut state, &mut holds, &mut down, 39, 1_000, None, HOLD,
    );
    assert!(down.contains("panel"));
    health::reset_for_test();
}

#[test]
fn genuine_reconnect_keeps_the_hold_when_the_value_is_unchanged() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 900);
    let mut holds = held(70);
    let mut down = BTreeSet::from([String::from("panel")]);
    let fake = FakePanel::new(71);

    let summary = probe_at(
        Some(&fake),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        None,
        HOLD,
    );

    assert_eq!(summary.reconnected, 1);
    assert_eq!(fake.writes.get(), 0);
    assert!(holds.contains_key("panel"));
    assert_eq!(
        state.monitor("panel").and_then(|m| m.last_applied_percent),
        Some(39)
    );
}

#[test]
fn factory_value_after_a_power_transition_is_corrected_not_held() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

    health::set_powered_off_for_test(Some(true));
    let standby = FakePanel::new(39);
    probe_at(
        Some(&standby),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        None,
        HOLD,
    );
    health::set_powered_off_for_test(Some(false));
    let woke = FakePanel::new(100);
    let summary = probe_at(
        Some(&woke),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_015,
        None,
        HOLD,
    );

    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty(), "a reset must not become a 30 minute hold");
    assert_eq!(woke.percent.get(), 39);
    health::reset_for_test();
}

#[test]
fn genuine_osd_change_is_still_held_once_the_power_transition_is_consumed() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    health::set_powered_off_for_test(Some(true));
    probe_at(
        Some(&FakePanel::new(39)),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        None,
        HOLD,
    );
    health::set_powered_off_for_test(Some(false));
    let woke = FakePanel::new(100);
    probe_at(
        Some(&woke),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_015,
        None,
        HOLD,
    );
    assert_eq!(woke.percent.get(), 39);

    let user = FakePanel::new(70);
    let summary = probe_at(
        Some(&user),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_500,
        None,
        HOLD,
    );
    assert_eq!(summary.corrected, 0);
    assert_eq!(user.writes.get(), 0);
    assert_eq!(holds.get("panel").map(|h| h.observed_percent), Some(70));
    health::reset_for_test();
}

#[test]
fn reset_shaped_value_shortly_after_a_recovery_window_is_a_reset() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());
    let late_wake = FakePanel::new(50);
    let summary = probe_at(
        Some(&late_wake),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        Some(950),
        HOLD,
    );
    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());

    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    let chosen = FakePanel::new(73);
    probe_at(
        Some(&chosen),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        Some(950),
        HOLD,
    );
    assert_eq!(chosen.writes.get(), 0);
    assert_eq!(holds.get("panel").map(|h| h.observed_percent), Some(73));

    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    let mut holds = BTreeMap::new();
    let fifty = FakePanel::new(50);
    probe_at(
        Some(&fifty),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        Some(700),
        HOLD,
    );
    assert_eq!(fifty.writes.get(), 0);
    assert!(holds.contains_key("panel"));
}

#[test]
fn hold_is_released_when_a_power_transition_wiped_the_held_value() {
    health::reset_for_test();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 39, 100);
    let mut holds = held(70);
    let mut down = BTreeSet::new();
    health::set_powered_off_for_test(Some(true));
    probe_at(
        Some(&FakePanel::new(70)),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_000,
        None,
        HOLD,
    );
    assert!(holds.contains_key("panel"), "value unchanged: hold kept");
    health::set_powered_off_for_test(Some(false));
    let woke = FakePanel::new(100);
    let summary = probe_at(
        Some(&woke),
        &mut state,
        &mut holds,
        &mut down,
        39,
        1_015,
        None,
        HOLD,
    );
    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
    health::reset_for_test();
}
