//! Direct ticks: overrides, suspension, weather and failure backoff.

use super::*;

#[test]
#[cfg(target_os = "linux")]
fn run_once_applies_manual_override_and_persists_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([(String::from("internal"), 33)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    runtime.last_capabilities = Some(test_capability_snapshot());
    let runner = FakeRunner::new().with_success(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "intel_backlight",
            "set",
            "33%",
        ],
        "",
    );

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &runner,
            false,
        )
        .expect("tick should succeed");

    assert!(report.manual_override_active);
    assert_eq!(report.apply_summary.succeeded, 1);
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .and_then(|monitor| monitor.last_applied_percent),
        Some(33)
    );

    let persisted = RuntimeState::load_from_path(&state_path).expect("state should persist");
    assert_eq!(
        persisted
            .monitor("internal")
            .and_then(|monitor| monitor.last_applied_percent),
        Some(33)
    );
}

#[test]
fn run_once_without_capability_snapshot_fails_closed() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    runtime.last_capabilities = None;
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(33),
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::new(),
        expires_at_epoch_s: None,
    });
    let runner = RecordingRunner::new();

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &runner,
            true,
        )
        .expect("tick should complete with a skipped apply");

    assert!(runtime.last_capabilities.is_none());
    assert_eq!(report.apply_summary.attempted, 0);
    assert_eq!(report.apply_summary.skipped, 1);
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[test]
fn run_once_skips_apply_while_suspended_even_with_manual_override() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    runtime.state.suspend_until_epoch_s = Some(1_800_000_100);
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(55),
        global_expires_at_epoch_s: Some(1_800_000_100),
        targets: std::collections::BTreeMap::from([(String::from("internal"), 33)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    let runner = FakeRunner::new();

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &runner,
            false,
        )
        .expect("tick should succeed");

    assert!(report.suspended);
    assert!(report.manual_override_active);
    assert_eq!(report.apply_summary.attempted, 0);
    assert_eq!(report.apply_summary.skipped, 1);
    assert!(runner.calls().is_empty());
}

#[test]
#[cfg(target_os = "linux")]
fn run_once_applies_global_override() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(44),
        global_expires_at_epoch_s: Some(1_800_000_100),
        targets: std::collections::BTreeMap::new(),
        expires_at_epoch_s: None,
    });
    runtime.last_capabilities = Some(test_capability_snapshot());
    let runner = FakeRunner::new().with_success(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "intel_backlight",
            "set",
            "44%",
        ],
        "",
    );

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &runner,
            false,
        )
        .expect("tick should succeed");

    assert!(report.manual_override_active);
    assert_eq!(report.apply_summary.succeeded, 1);
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .and_then(|monitor| monitor.last_applied_percent),
        Some(44)
    );
}

#[test]
fn run_once_uses_persisted_weather_metadata_as_bounded_modifier() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_weather_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    let weather_snapshot = crate::state::WeatherSnapshotMetadata {
        provider: String::from("openweather"),
        fetched_at_epoch_s: 1_800_000_000,
        valid_at_epoch_s: 1_700_000_000,
        source_kind: crate::weather::WeatherSourceKind::Forecast,
        cloud_cover_percent: Some(100),
        smoothed_cloud_cover_percent: Some(100),
        temperature: Some(0.0),
        condition: crate::weather::WeatherCondition::Cloudy,
        condition_description: None,
        day_phase: None,
        forecast: vec![],
        ..Default::default()
    };
    runtime.state.weather = Some(weather_snapshot);
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([(String::from("internal"), 25)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    let runner = FakeRunner::new().with_success(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "intel_backlight",
            "set",
            "25%",
        ],
        "",
    );

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_030, 0)
                .single()
                .expect("valid time"),
            &runner,
            false,
        )
        .expect("tick should succeed");

    assert!(report.weather_modifier_applied);
}

#[test]
#[cfg(target_os = "linux")]
fn apply_failure_backoff_persists_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([(String::from("internal"), 33)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    let runner = FakeRunner::new().with_output(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "intel_backlight",
            "set",
            "33%",
        ],
        Some(1),
        "",
        "temporarily unavailable",
    );

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &runner,
            false,
        )
        .expect("tick should succeed");

    assert_eq!(report.apply_summary.failed, 1);
    let persisted = RuntimeState::load_from_path(&state_path).expect("state should persist");
    let backoff = persisted
        .monitor("internal")
        .and_then(|monitor| monitor.backoff.as_ref())
        .expect("backoff state should persist");
    assert_eq!(backoff.backend, BackendKind::Backlight);
    assert_eq!(backoff.failure_kind, FailureKind::Transient);
}

#[test]
fn run_once_expires_manual_override_before_apply() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_dry_run_config_report(),
        state_path.clone(),
        socket_path,
    )
    .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([(String::from("internal"), 29)]),
        expires_at_epoch_s: Some(1_799_999_999),
    });

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &FakeRunner::new(),
            false,
        )
        .expect("tick should succeed");

    assert!(!report.manual_override_active);
    assert_eq!(runtime.state.manual_override, None);
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .manual_override,
        None
    );
}

#[test]
#[cfg(target_os = "linux")]
fn ipc_run_once_executes_tick_and_returns_summary() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    runtime.state.monitor_mut("internal").backoff = Some(FailureBackoffState {
        backend: BackendKind::Backlight,
        failure_kind: FailureKind::Transient,
        consecutive_failures: 3,
        suppress_until_epoch_s: Some(1_800_000_500),
    });
    let runner = FakeRunner::new().with_success(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "intel_backlight",
            "set",
            "27%",
        ],
        "",
    );

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::RunOnce { force: true },
        &runner,
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(outcome.tick_attempted);

    let Response::RunOnce { run_once, .. } = response.response else {
        panic!("run_once request should return a run_once response");
    };

    assert_eq!(run_once.monitors_evaluated, 0);
    assert_eq!(run_once.writes_attempted, 0);
    assert_eq!(run_once.writes_skipped, 0);
    assert_eq!(run_once.writes_succeeded, 0);
    assert!(
        run_once.queued,
        "deferred apply must not look like real stats"
    );
}
