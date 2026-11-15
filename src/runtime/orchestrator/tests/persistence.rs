//! Runtime state is persisted only when it changes.

use super::*;

#[test]
fn no_op_tick_does_not_persist_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_dry_run_config_report(),
        state_path.clone(),
        socket_path,
    )
    .expect("runtime should bootstrap");

    let report = runtime
        .run_once_at_with_runner(
            Utc.timestamp_opt(1_800_000_000, 0)
                .single()
                .expect("valid time"),
            &FakeRunner::new(),
            false,
        )
        .expect("tick should succeed");

    assert_eq!(report.apply_summary.attempted, 0);
    assert_eq!(report.apply_summary.skipped, 1);
    assert!(!state_path.exists());
    assert!(!runtime.persist_state_if_changed().expect("persist check"));
}

#[test]
fn suspend_mutation_persists_state_once() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");

    runtime.state.suspend_until_epoch_s = Some(1_800_000_900);

    assert!(runtime
        .persist_state_if_changed()
        .expect("persist should work"));
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state should load")
            .suspend_until_epoch_s,
        Some(1_800_000_900)
    );
    assert!(!runtime.persist_state_if_changed().expect("repeat check"));
}

#[test]
fn override_mutation_persists_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(40),
        global_expires_at_epoch_s: Some(1_800_000_600),
        targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
        expires_at_epoch_s: Some(1_800_000_600),
    });

    assert!(runtime
        .persist_state_if_changed()
        .expect("persist should work"));
    let persisted = RuntimeState::load_from_path(&state_path).expect("state should load");
    let manual_override = persisted
        .manual_override
        .expect("manual override should persist");
    assert_eq!(manual_override.global_percent(1_800_000_000), Some(40));
    assert_eq!(
        manual_override.target_percent("internal", 1_800_000_000),
        Some(27)
    );
}

#[test]
fn weather_mutation_persists_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_weather_config_report(),
        state_path.clone(),
        socket_path,
    )
    .expect("runtime should bootstrap");
    runtime.state.weather = Some(WeatherSnapshotMetadata {
        provider: String::from("openweather"),
        fetched_at_epoch_s: 1_800_000_000,
        valid_at_epoch_s: 1_700_000_000,
        source_kind: crate::weather::WeatherSourceKind::Forecast,
        cloud_cover_percent: Some(81),
        smoothed_cloud_cover_percent: Some(77),
        temperature: Some(0.0),
        condition: crate::weather::WeatherCondition::Cloudy,
        condition_description: None,
        day_phase: None,
        forecast: vec![],
        ..Default::default()
    });

    assert!(runtime
        .persist_state_if_changed()
        .expect("persist should work"));
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state should load")
            .weather,
        runtime.state.weather
    );
}

#[test]
fn persisted_state_writes_leave_no_temporary_files_behind() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    runtime.state.suspend_until_epoch_s = Some(1_800_000_900);

    assert!(runtime
        .persist_state_if_changed()
        .expect("persist should work"));
    assert_eq!(count_temp_state_files(&state_path), 0);
}
