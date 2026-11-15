//! Bootstrap paths and background capability observation.

use super::*;

#[test]
fn bootstrap_creates_state_dir_and_configures_platform_ipc() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");

    let runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        state_path.clone(),
        socket_path.clone(),
    )
    .expect("runtime should bootstrap");

    assert!(state_path.parent().expect("state dir").exists());
    #[cfg(target_os = "linux")]
    assert!(socket_path.parent().expect("socket dir").exists());
    #[cfg(target_os = "windows")]
    assert_eq!(
        runtime.socket.endpoint(),
        crate::paths::IpcEndpoint::NamedPipe(socket_path.display().to_string())
    );
    assert_eq!(runtime.state, RuntimeState::default());
}

#[cfg(target_os = "linux")]
#[test]
fn one_shot_bootstrap_does_not_create_socket_directory() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");

    DaemonRuntime::bootstrap_with_paths_mode(
        test_config_report(),
        state_path,
        socket_path.clone(),
        false,
    )
    .expect("one-shot bootstrap should succeed");

    assert!(!socket_path.parent().expect("socket directory").exists());
}

#[test]
fn slow_capability_refresh_does_not_block_ipc_or_publish_partial_snapshot() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    runtime.last_capabilities = None;
    let runner = SlowObservationRunner {
        delay: Duration::from_millis(150),
        started: Arc::new(AtomicBool::new(false)),
        discoveries: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let discoveries = Arc::clone(&runner.discoveries);
    let started = Arc::clone(&runner.started);

    runtime.begin_capability_refresh_with_runner(runner.clone());
    let started_deadline = Instant::now() + Duration::from_millis(100);
    while !started.load(Ordering::Acquire) && Instant::now() < started_deadline {
        std::thread::yield_now();
    }
    runtime.begin_capability_refresh_with_runner(runner);
    assert!(started.load(Ordering::Acquire));
    assert!(runtime.last_capabilities.is_none());
    assert_eq!(discoveries.load(Ordering::Relaxed), 1);

    let status_started = Instant::now();
    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        ipc::Request::Status,
        &RecordingRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );
    assert!(matches!(response.response, ipc::Response::Status { .. }));
    assert!(!outcome.tick_attempted);
    assert!(
        status_started.elapsed() < Duration::from_millis(50),
        "IPC status handling must not wait for observation"
    );

    std::thread::sleep(Duration::from_millis(600));
    runtime.publish_completed_capability_snapshot();
    assert!(runtime.last_capabilities.is_some());
    assert_eq!(discoveries.load(Ordering::Relaxed), 1);
}

#[test]
fn panicking_observation_clears_active_state_and_allows_retry() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");

    runtime.begin_capability_refresh_with_runner(PanicObservationRunner);
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut result = None;
    while Instant::now() < deadline {
        result = runtime.publish_completed_capability_snapshot();
        if result.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(result, Some(false));

    let runner = SlowObservationRunner {
        delay: Duration::from_millis(1),
        started: Arc::new(AtomicBool::new(false)),
        discoveries: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let started = Arc::clone(&runner.started);
    runtime.begin_capability_refresh_with_runner(runner);
    let deadline = Instant::now() + Duration::from_millis(100);
    while !started.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(started.load(Ordering::Acquire));
}

#[test]
fn bootstrap_rejects_future_state_without_replacing_it() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let original = br#"{"schema_version":2,"future":{"new":true},"monitors":{}}
"#;
    fs::create_dir_all(state_path.parent().expect("state dir")).expect("state dir should exist");
    fs::write(&state_path, original).expect("future state should write");

    let error =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect_err("future state must prevent startup");

    assert!(matches!(
        error,
        RuntimeError::State(StateError::UnsupportedSchemaVersion {
            found: 2,
            supported: 1,
            ..
        })
    ));
    assert_eq!(
        fs::read(&state_path).expect("state should remain"),
        original
    );
    assert_eq!(
        fs::read_dir(temp.path().join("state"))
            .expect("state dir should be readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
            .count(),
        0
    );
}

#[test]
fn bootstrap_prunes_stale_runtime_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut stale_state = RuntimeState::default();
    stale_state.record_apply_success("stale", 70, 1_800_000_000);
    stale_state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([(String::from("stale"), 55)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    stale_state
        .save_to_path(&state_path)
        .expect("stale state should save");

    let runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");

    assert!(runtime.state.monitor("stale").is_none());
    assert_eq!(runtime.state.manual_override, None);
    let persisted = RuntimeState::load_from_path(&state_path).expect("state file should load");
    assert!(persisted.monitor("stale").is_none());
    assert_eq!(persisted.manual_override, None);
}
