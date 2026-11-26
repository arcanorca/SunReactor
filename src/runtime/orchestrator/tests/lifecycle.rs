//! Daemon lifecycle hardening: accept-error policy, persisted holds and
//! write ledger, wall-clock steps, and selective fade cancellation.

use super::*;
use crate::runtime::capabilities::CAPABILITY_SAFETY_REFRESH_INTERVAL;

fn now_s() -> u64 {
    state::current_epoch_s()
}

fn bootstrap_with_state(temp: &TempDir, state: &RuntimeState) -> DaemonRuntime {
    let state_path = temp.path().join("state/runtime-state.json");
    state.save_to_path(&state_path).expect("seed state file");
    DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        state_path,
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap")
}

fn hold(since: u64, until: u64, observed: u8) -> state::PersistedHold {
    state::PersistedHold {
        since_epoch_s: since,
        until_epoch_s: until,
        observed_percent: observed,
    }
}

fn io_error(code: i32) -> ipc::IpcError {
    ipc::IpcError::Io {
        target: String::from("test"),
        source: std::io::Error::from_raw_os_error(code),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn accept_errors_drop_the_connection_and_only_a_dead_listener_is_fatal() {
    use crate::runtime::orchestrator::{classify_ipc_accept_error, AcceptErrorAction};
    let protocol = ipc::IpcError::Protocol {
        message: String::from("Rejecting connection from unauthorized UID: 4242"),
    };
    assert_eq!(
        classify_ipc_accept_error(&protocol),
        AcceptErrorAction::DropConnection
    );
    for code in [
        libc::ECONNRESET,
        libc::ECONNABORTED,
        libc::EINTR,
        libc::ETIMEDOUT,
        libc::EPIPE,
        libc::ENOTCONN,
    ] {
        assert_eq!(
            classify_ipc_accept_error(&io_error(code)),
            AcceptErrorAction::DropConnection,
            "errno {code}"
        );
    }
    for code in [libc::EMFILE, libc::ENFILE, libc::ENOMEM, libc::ENOBUFS] {
        assert_eq!(
            classify_ipc_accept_error(&io_error(code)),
            AcceptErrorAction::BackOff,
            "errno {code}"
        );
    }
    for code in [libc::EBADF, libc::ENOTSOCK, libc::EINVAL] {
        assert_eq!(
            classify_ipc_accept_error(&io_error(code)),
            AcceptErrorAction::ListenerDead,
            "errno {code}"
        );
    }
}

/// Replaces the daemon's listening socket fd with /dev/null (same fd number,
/// so nothing else is disturbed) to simulate a dead listener.
#[cfg(target_os = "linux")]
fn kill_listener_fd(socket_path: &Path) -> bool {
    use std::os::unix::io::AsRawFd;
    let wanted = socket_path.to_string_lossy().into_owned();
    let Ok(unix) = fs::read_to_string("/proc/net/unix") else {
        return false;
    };
    let inodes: Vec<String> = unix
        .lines()
        .filter(|line| line.ends_with(&wanted))
        .filter_map(|line| line.split_whitespace().nth(6).map(str::to_owned))
        .collect();
    let Ok(entries) = fs::read_dir("/proc/self/fd") else {
        return false;
    };
    let null = fs::File::open("/dev/null").expect("/dev/null");
    for entry in entries.filter_map(Result::ok) {
        let Ok(target) = fs::read_link(entry.path()) else {
            continue;
        };
        let target = target.to_string_lossy().into_owned();
        if inodes
            .iter()
            .any(|inode| target == format!("socket:[{inode}]"))
        {
            let fd: i32 = entry.file_name().to_string_lossy().parse().unwrap_or(-1);
            // SAFETY: both descriptors are valid; dup2 atomically replaces fd.
            return fd >= 0 && unsafe { libc::dup2(null.as_raw_fd(), fd) } == fd;
        }
    }
    false
}

#[cfg(target_os = "linux")]
#[test]
fn run_loop_persists_state_even_when_the_listener_dies() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_dry_run_config_report(),
        state_path.clone(),
        socket_path.clone(),
    )
    .expect("runtime should bootstrap");
    // An unsaved change that only the exit-path persist can write.
    runtime.state.suspend_indefinite = true;

    let mut calls = 0;
    let mut killed = false;
    let result = runtime.run_loop(|| {
        calls += 1;
        if calls == 2 {
            killed = kill_listener_fd(&socket_path);
        }
        calls > 400
    });

    assert!(killed, "test must be able to break the listener");
    assert!(matches!(result, Err(RuntimeError::Ipc(_))), "{result:?}");
    let saved = RuntimeState::load_from_path(&state_path).expect("state loads");
    assert!(saved.suspend_indefinite, "exit path must persist state");
}

#[test]
fn restart_restores_a_plausible_hold_and_budget_and_does_not_correct_the_monitor() {
    let temp = TempDir::new();
    let now = now_s();
    let mut seeded = RuntimeState::default();
    seeded.record_apply_success("internal", 40, now - 120);
    seeded.record_budget_event("internal", now - 30, 600);
    seeded.record_budget_event("internal", now - 20, 600);
    seeded
        .external_holds
        .insert(String::from("internal"), hold(now - 60, now + 1_500, 77));

    let mut runtime = bootstrap_with_state(&temp, &seeded);

    let restored = runtime
        .external_holds
        .get("internal")
        .copied()
        .expect("hold restored");
    assert_eq!(restored.observed_percent, 77);
    assert_eq!(restored.until_epoch_s, now + 1_500);
    assert_eq!(
        runtime.state.budget_events_in_window(
            "internal",
            now,
            crate::apply::types::WRITE_BUDGET_WINDOW_SECONDS
        ),
        2,
        "budget events survive the restart"
    );

    // The startup baseline reset (resync) must still leave the held monitor alone.
    let runner = RecordingRunner::new();
    runtime
        .run_once_at_with_runner(Utc::now(), &runner, true)
        .expect("tick");
    assert!(
        runner
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|call| !call.contains("|set|")),
        "a restored hold must not be corrected at startup: {:?}",
        runner.calls.lock().unwrap()
    );
    assert!(runtime.external_holds.contains_key("internal"));
}

#[test]
fn restart_discards_implausible_holds_and_ledger_events() {
    let now = now_s();
    let cases = [
        (
            "future start",
            hold(now + 3 * 3600, now + 3 * 3600 + 1_000, 50),
        ),
        (
            "longer than the configured hold",
            hold(now - 10, now + 5 * 3600, 50),
        ),
        ("already expired", hold(now - 4_000, now - 100, 50)),
        ("inverted", hold(now + 100, now + 50, 50)),
    ];
    for (name, bad) in cases {
        let temp = TempDir::new();
        let mut seeded = RuntimeState::default();
        seeded.record_apply_success("internal", 40, now - 60);
        seeded.external_holds.insert(String::from("internal"), bad);
        // Ledger: one event from the future, one older than the window, one valid.
        let events = &mut seeded.monitor_mut("internal").ledger.recent_events;
        events.extend([now + 3 * 3600, now - 5_000, now - 10]);

        let runtime = bootstrap_with_state(&temp, &seeded);

        assert!(
            runtime.external_holds.is_empty(),
            "{name}: hold must be discarded"
        );
        assert_eq!(
            runtime
                .state
                .monitor("internal")
                .expect("monitor")
                .ledger
                .recent_events,
            vec![now - 10],
            "{name}: only the plausible event survives"
        );
    }
}

#[test]
fn holds_survive_a_save_load_cycle_through_the_runtime() {
    let temp = TempDir::new();
    let now = now_s();
    let mut runtime = bootstrap_with_state(&temp, &RuntimeState::default());
    runtime.state.record_apply_success("internal", 55, now);
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: now,
            until_epoch_s: now + 900,
            observed_percent: 33,
        },
    );
    assert!(runtime.persist_state_if_changed().expect("persist"));
    drop(runtime);

    let reborn = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("second bootstrap");
    assert_eq!(reborn.external_holds["internal"].observed_percent, 33);
}

#[test]
fn clearing_a_hold_is_persisted_so_a_restart_does_not_resurrect_it() {
    let temp = TempDir::new();
    let now = now_s();
    let mut runtime = bootstrap_with_state(&temp, &RuntimeState::default());
    runtime.state.record_apply_success("internal", 55, now);
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: now,
            until_epoch_s: now + 900,
            observed_percent: 33,
        },
    );
    runtime.persist_state_if_changed().expect("persist");
    runtime.clear_external_holds(Some("internal"));
    let saved =
        RuntimeState::load_from_path(&temp.path().join("state/runtime-state.json")).expect("load");
    assert!(saved.external_holds.is_empty());
}

#[test]
fn clock_stepped_back_three_hours_does_not_freeze_windows() {
    crate::apply::health::reset_for_test();
    let temp = TempDir::new();
    let now = now_s();
    let mut runtime = bootstrap_with_state(&temp, &RuntimeState::default());
    let future = now + 3 * 3600;
    runtime.state.record_apply_success("internal", 55, future);
    runtime.state.record_budget_event("internal", future, 600);
    runtime.state.record_apply_failure(
        "internal",
        BackendKind::Backlight,
        crate::backends::FailureKind::Transient,
        future,
    );
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: future,
            until_epoch_s: future + 1_800,
            observed_percent: 20,
        },
    );
    runtime.recovery_until_epoch_s = Some(future + 120);
    for offset in [0_u64, 100, 200] {
        crate::apply::health::note_contest_event(
            "internal",
            future + offset + 10,
            Some(55),
            Some(future + offset),
            80,
        );
    }
    assert!(crate::apply::is_contested("internal", future + 220));

    runtime.clamp_future_windows(now);

    let monitor = runtime.state.monitor("internal").expect("monitor");
    assert_eq!(monitor.last_applied_at_epoch_s, Some(now));
    assert_eq!(monitor.ledger.recent_events, vec![now]);
    let until = monitor
        .backoff
        .as_ref()
        .and_then(|b| b.suppress_until_epoch_s)
        .expect("backoff");
    assert!(
        until <= now + crate::state::BACKOFF_MAX_SECONDS,
        "backoff clamped, got {until}"
    );
    let hold = runtime.external_holds["internal"];
    assert_eq!(hold.since_epoch_s, now);
    assert!(
        hold.until_epoch_s <= now + 30 * 60,
        "hold cut to the configured length"
    );
    assert!(
        runtime.recovery_until_epoch_s.expect("recovery") <= now + apply::RECOVERY_WINDOW_SECONDS
    );
    assert!(
        crate::apply::contested_until("internal", now).expect("contested")
            <= now + crate::apply::CONTESTED_BACKOFF_SECONDS
    );
    assert!(runtime.clock_warned, "logged once");

    // A second pass finds nothing to clamp and re-arms the warning.
    runtime.clamp_future_windows(now);
    assert!(!runtime.clock_warned);
    crate::apply::health::reset_for_test();
}

#[test]
fn small_skew_and_plausible_future_windows_are_left_alone() {
    let temp = TempDir::new();
    let now = now_s();
    let mut runtime = bootstrap_with_state(&temp, &RuntimeState::default());
    runtime.state.record_apply_success("internal", 55, now + 3);
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: now - 10,
            until_epoch_s: now + 1_200,
            observed_percent: 20,
        },
    );
    runtime.clamp_future_windows(now);
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .expect("monitor")
            .last_applied_at_epoch_s,
        Some(now + 3)
    );
    assert_eq!(
        runtime.external_holds["internal"].until_epoch_s,
        now + 1_200
    );
    assert!(!runtime.clock_warned);
}

#[test]
fn publishing_a_snapshot_cancels_only_fades_of_changed_monitors() {
    let temp = TempDir::new();
    let mut report = test_config_report();
    let mut second = report.config.monitors[0].clone();
    second.logical_id = String::from("external");
    second.selector.sysfs_path = Some(String::from("/sys/class/backlight/other_backlight"));
    report.config.monitors.push(second);
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        report,
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");

    let mut before = test_capability_snapshot();
    let mut other = before.backlights_present[0].clone();
    other.stable_id = String::from("backlight:other_backlight");
    other.device_name = String::from("other_backlight");
    other.sysfs_path = String::from("/sys/class/backlight/other_backlight");
    before.backlights_present.push(other);
    runtime.last_capabilities = Some(before.clone());
    runtime
        .state
        .record_apply_success("internal", 20, NOW_FOR_FADES - 1);
    runtime
        .state
        .record_apply_success("external", 20, NOW_FOR_FADES - 1);
    assert!(runtime
        .fade_engine
        .maybe_enqueue_with("internal", 20, 80, true));
    assert!(runtime
        .fade_engine
        .maybe_enqueue_with("external", 20, 80, true));

    // Identical snapshot: nothing is cancelled.
    runtime.apply_published_snapshot(before.clone());
    assert_eq!(runtime.fade_engine.active_fades.len(), 2);

    // Only "external" disappears: only its fade is cancelled.
    let mut after = before;
    after.backlights_present.pop();
    runtime.apply_published_snapshot(after);
    assert!(runtime.fade_engine.active_fades.contains_key("internal"));
    assert!(!runtime.fade_engine.active_fades.contains_key("external"));
}

const NOW_FOR_FADES: u64 = 1_800_000_000;

#[test]
fn ordinary_ticks_never_request_a_capability_refresh() {
    let temp = TempDir::new();
    let mut runtime = bootstrap_with_state(&temp, &RuntimeState::default());
    runtime.capability_refresh.note_started(Instant::now());
    // A scheduled tick runs without any observation being due.
    let runner = RecordingRunner::new();
    for _ in 0..3 {
        runtime
            .run_once_at_with_runner(Utc::now(), &runner, false)
            .expect("tick");
    }
    assert!(!runtime
        .capability_refresh
        .due(Instant::now() + Duration::from_mins(14), true));
    assert!(runtime.capability_refresh.due(
        Instant::now() + CAPABILITY_SAFETY_REFRESH_INTERVAL + Duration::from_mins(1),
        true
    ));
    // Hotplug/resume do request one.
    runtime.handle_display_event(crate::platform::DisplayRecoveryReason::DrmHotplug);
    assert!(runtime
        .capability_refresh
        .due(Instant::now() + Duration::from_secs(11), true));
}
