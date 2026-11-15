//! Bootstrap paths and background capability observation.

use super::*;
#[cfg(target_os = "linux")]
use crate::ipc;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::net::Shutdown;
#[cfg(target_os = "linux")]
use std::os::unix::net::UnixStream;

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
fn accept_with_deadline(listener: &crate::ipc::BoundControlSocket) -> UnixStream {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match listener.accept().expect("listener accept should work") {
            Some(stream) => return stream,
            None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(1)),
            None => panic!("timed out waiting for hostile IPC connection"),
        }
    }
}

#[cfg(target_os = "linux")]
fn assert_next_ping(
    runtime: &mut DaemonRuntime,
    listener: &crate::ipc::BoundControlSocket,
    path: &Path,
) {
    let target = path.display().to_string();
    let mut client = UnixStream::connect(path).expect("valid client should connect");
    ipc::transport::configure_client_stream(&client, &target).expect("client should configure");
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("client read timeout should set");
    ipc::transport::write_json_message(
        &mut client,
        &ipc::RequestEnvelope::new(ipc::Request::Ping),
        &target,
    )
    .expect("valid request should write");

    let server = accept_with_deadline(listener);
    let started = Instant::now();
    runtime.handle_ipc_stream(server);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "valid IPC request should not hang"
    );
    let response = ipc::transport::read_response(&mut client, &target)
        .expect("valid response should be readable");
    assert_eq!(response, ipc::ResponseEnvelope::pong());
}

#[cfg(target_os = "linux")]
#[test]
fn hostile_ipc_connections_do_not_poison_the_next_valid_request() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_dry_run_config_report(),
        state_path,
        socket_path.clone(),
    )
    .expect("runtime should bootstrap");
    let socket = crate::ipc::ControlSocket {
        path: socket_path.clone(),
    };
    let listener = socket.bind_listener().expect("listener should bind");
    let target = socket_path.display().to_string();

    // A complete oversized frame is rejected before it can consume the next
    // connection, and the peer is closed so the error reply exercises the
    // server's failed-write path.
    let mut oversized = UnixStream::connect(&socket_path).expect("oversized client should connect");
    oversized
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("oversized client write timeout should set");
    oversized
        .write_all(&vec![b'x'; 64 * 1024 + 1])
        .expect("oversized frame should write");
    oversized
        .write_all(b"\n")
        .expect("oversized delimiter should write");
    oversized
        .shutdown(Shutdown::Both)
        .expect("oversized client should close");
    let server = accept_with_deadline(&listener);
    let started = Instant::now();
    runtime.handle_ipc_stream(server);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_next_ping(&mut runtime, &listener, &socket_path);

    // Malformed JSON with a closed peer must produce no panic and leave the
    // listener able to accept a subsequent valid request.
    let mut malformed = UnixStream::connect(&socket_path).expect("malformed client should connect");
    malformed
        .write_all(b"not-json\n")
        .expect("malformed frame should write");
    malformed
        .shutdown(Shutdown::Both)
        .expect("malformed client should close");
    let server = accept_with_deadline(&listener);
    let started = Instant::now();
    runtime.handle_ipc_stream(server);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_next_ping(&mut runtime, &listener, &socket_path);

    // A slow drip must be bounded by the server read timeout.
    let drip = UnixStream::connect(&socket_path).expect("slow client should connect");
    let drip_thread = std::thread::spawn(move || {
        let mut drip = drip;
        let _ = drip.write_all(b"{");
        std::thread::sleep(Duration::from_millis(150));
        let _ = drip.write_all(b"\"version\":1}\n");
    });
    let server = accept_with_deadline(&listener);
    let started = Instant::now();
    runtime.handle_ipc_stream(server);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "slow-drip IPC request should time out"
    );
    drip_thread.join().expect("slow client should finish");
    assert_next_ping(&mut runtime, &listener, &socket_path);

    // A request whose write half is closed is still a complete frame and must
    // receive a normal response.
    let mut half_closed =
        UnixStream::connect(&socket_path).expect("half-closed client should connect");
    ipc::transport::configure_client_stream(&half_closed, &target)
        .expect("half-closed client should configure");
    ipc::transport::write_json_message(
        &mut half_closed,
        &ipc::RequestEnvelope::new(ipc::Request::Ping),
        &target,
    )
    .expect("half-closed request should write");
    half_closed
        .shutdown(Shutdown::Write)
        .expect("half-closed write side should close");
    let server = accept_with_deadline(&listener);
    runtime.handle_ipc_stream(server);
    assert_eq!(
        ipc::transport::read_response(&mut half_closed, &target)
            .expect("half-closed response should be readable"),
        ipc::ResponseEnvelope::pong()
    );
    assert_next_ping(&mut runtime, &listener, &socket_path);

    // The peer can disappear after the request has been accepted and before
    // the reply is written; this is a normal connection failure, not a daemon
    // failure.
    let mut closed = UnixStream::connect(&socket_path).expect("closed client should connect");
    ipc::transport::write_json_message(
        &mut closed,
        &ipc::RequestEnvelope::new(ipc::Request::Ping),
        &target,
    )
    .expect("closed request should write");
    let server = accept_with_deadline(&listener);
    drop(closed);
    let started = Instant::now();
    runtime.handle_ipc_stream(server);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_next_ping(&mut runtime, &listener, &socket_path);
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
