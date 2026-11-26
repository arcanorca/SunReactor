use super::super::*;
use super::support::{panel_monitor, probe_panel, test_policy, FakePanel};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[test]
fn off_by_one_readback_is_not_rewritten() {
    for observed in [42, 43, 44] {
        let fake = FakePanel::new(observed);
        let mut state = RuntimeState::default();
        state.record_apply_success("panel", 43, 900);
        let (mut holds, mut down) = (BTreeMap::new(), BTreeSet::new());

        let summary = probe_panel(&fake, &mut state, &mut holds, &mut down, 43);

        assert_eq!(summary.corrected, 0, "observed {observed}");
        assert_eq!(fake.writes.get(), 0, "observed {observed}");
        assert!(holds.is_empty());
        assert_eq!(
            state
                .monitor("panel")
                .and_then(|m| m.last_integrity_check_at_epoch_s),
            Some(1_000)
        );
    }
}

#[test]
fn rounding_monitor_is_learned_and_not_rewritten_forever() {
    let mut fake = FakePanel::new(45);
    fake.snap_offset = 2;
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 43, 900);
    let mut holds = BTreeMap::new();
    let mut down = BTreeSet::new();
    let policy = test_policy(vec![("panel", 43)]);
    let monitor = panel_monitor();

    let mut total_corrected = 0;
    for probe in 0..10u64 {
        let summary = probe_and_correct_with_external_state(
            &ProbeInputs {
                monitors: std::slice::from_ref(&monitor),
                policy: &policy,
                now_epoch_s: 1_000 + probe * 30,
                dry_run: false,
                external_change_hold: Duration::from_mins(30),
                recovery_until_epoch_s: None,
            },
            ProbeSinks {
                state: &mut state,
                external_holds: Some(&mut holds),
                fade_engine: None,
                unreachable_monitors: Some(&mut down),
            },
            |m| fake.read(m),
            |m, t, percent| fake.write(m, t, percent),
        );
        total_corrected += summary.corrected + summary.failed;
    }

    assert!(holds.is_empty());
    assert_eq!(
        fake.writes.get(),
        1,
        "one attempt, then the readback is accepted"
    );
    assert_eq!(total_corrected, 1);
    assert!(state.quantization_accepted("panel", 43, 45));
}

#[test]
fn reset_value_is_never_learned_as_rounding() {
    let mut fake = FakePanel::new(100);
    fake.ignores_writes = true;
    let mut state = RuntimeState::default();
    let mut holds = BTreeMap::new();
    let mut down = BTreeSet::new();
    let policy = test_policy(vec![("panel", 39)]);
    let monitor = panel_monitor();
    for probe in 0..3u64 {
        probe_and_correct_with_external_state(
            &ProbeInputs {
                monitors: std::slice::from_ref(&monitor),
                policy: &policy,
                now_epoch_s: 1_000 + probe * 30,
                dry_run: false,
                external_change_hold: Duration::from_mins(30),
                recovery_until_epoch_s: None,
            },
            ProbeSinks {
                state: &mut state,
                external_holds: Some(&mut holds),
                fade_engine: None,
                unreachable_monitors: Some(&mut down),
            },
            |m| fake.read(m),
            |m, t, percent| fake.write(m, t, percent),
        );
    }
    assert!(fake.writes.get() >= 3);
    assert!(!state.quantization_accepted("panel", 39, 100));
}
