//! IPC requests that edit state or config and queue an apply.

use super::*;

#[test]
#[cfg(target_os = "linux")]
fn ipc_suspend_and_resume_persist_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(40),
        global_expires_at_epoch_s: Some(1_800_000_100),
        targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });

    let (suspend_response, suspend_outcome) = runtime.handle_ipc_request_with_runner(
        Request::Suspend { minutes: Some(15) },
        &FakeRunner::new(),
        now,
    );
    assert!(!suspend_outcome.tick_attempted);
    assert!(matches!(suspend_response.response, Response::Ack { .. }));
    assert_eq!(runtime.state.suspend_until_epoch_s, Some(1_800_000_900));
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .suspend_until_epoch_s,
        Some(1_800_000_900)
    );

    let runner = RecordingRunner::new();
    let (resume_response, resume_outcome) =
        runtime.handle_ipc_request_with_runner(Request::Resume, &runner, now);
    assert!(resume_outcome.tick_attempted);
    assert!(matches!(resume_response.response, Response::Ack { .. }));
    assert_eq!(runtime.state.suspend_until_epoch_s, None);
    assert_eq!(runtime.state.manual_override, None);
    assert_eq!(runner.calls().len(), 0);
    assert!(runtime.pending_apply);
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .suspend_until_epoch_s,
        None
    );
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .manual_override,
        None
    );
}

#[test]
#[cfg(target_os = "linux")]
fn ipc_indefinite_suspend_persists_state_until_resume() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");

    let (suspend_response, suspend_outcome) = runtime.handle_ipc_request_with_runner(
        Request::Suspend { minutes: None },
        &FakeRunner::new(),
        now,
    );
    assert!(!suspend_outcome.tick_attempted);
    assert!(matches!(suspend_response.response, Response::Ack { .. }));
    assert!(runtime.state.suspend_indefinite);
    assert_eq!(runtime.state.suspend_until_epoch_s, None);
    assert!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .suspend_indefinite
    );

    let runner = RecordingRunner::new();
    let (resume_response, resume_outcome) =
        runtime.handle_ipc_request_with_runner(Request::Resume, &runner, now);
    assert!(resume_outcome.tick_attempted);
    assert!(matches!(resume_response.response, Response::Ack { .. }));
    assert!(!runtime.state.suspend_indefinite);
    assert_eq!(runtime.state.suspend_until_epoch_s, None);
    assert_eq!(runner.calls().len(), 0);
    assert!(runtime.pending_apply);
}

#[test]
#[cfg(target_os = "linux")]
fn ipc_resume_forces_reapply_and_clears_monitor_backoff() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");
    runtime.state.suspend_until_epoch_s = Some(1_800_000_900);
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(40),
        global_expires_at_epoch_s: Some(1_800_000_100),
        targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    runtime.state.monitor_mut("internal").last_applied_percent = Some(90);
    runtime
        .state
        .monitor_mut("internal")
        .last_applied_at_epoch_s = Some(1_799_999_940);
    runtime.state.monitor_mut("internal").backoff = Some(FailureBackoffState {
        backend: BackendKind::Backlight,
        failure_kind: FailureKind::Transient,
        consecutive_failures: 4,
        suppress_until_epoch_s: Some(1_800_000_500),
    });

    let runner = RecordingRunner::new();
    let (response, outcome) = runtime.handle_ipc_request_with_runner(Request::Resume, &runner, now);

    assert!(outcome.tick_attempted);
    assert!(matches!(response.response, Response::Ack { .. }));
    assert_eq!(runner.calls().len(), 0);
    assert!(runtime.pending_apply);
    assert_eq!(runtime.state.suspend_until_epoch_s, None);
    assert_eq!(runtime.state.manual_override, None);
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .and_then(|monitor| monitor.backoff.as_ref()),
        None
    );
    assert_eq!(
        runtime
            .state
            .monitor("internal")
            .and_then(|monitor| monitor.last_applied_at_epoch_s),
        Some(1_799_999_940)
    );
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .monitor("internal")
            .and_then(|monitor| monitor.backoff.as_ref()),
        None
    );
}

#[test]
fn ipc_set_and_clear_override_persist_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_dry_run_config_report(),
        state_path.clone(),
        socket_path,
    )
    .expect("runtime should bootstrap");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");

    let (set_response, set_outcome) = runtime.handle_ipc_request_with_runner(
        Request::SetOverride {
            monitor_id: Some(String::from("internal")),
            percent: 36,
            minutes: Some(10),
        },
        &FakeRunner::new(),
        now,
    );
    assert!(set_outcome.tick_attempted);
    assert!(matches!(set_response.response, Response::Ack { .. }));
    assert_eq!(
        runtime
            .state
            .manual_override
            .as_ref()
            .and_then(|manual_override| manual_override.target_percent("internal", 1_800_000_000)),
        Some(36)
    );

    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: 1_800_000_000,
            until_epoch_s: 1_800_001_800,
            observed_percent: 20,
        },
    );
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .manual_override
            .as_ref()
            .and_then(|manual_override| manual_override.target_percent("internal", 1_800_000_000)),
        Some(36)
    );

    let (clear_response, clear_outcome) = runtime.handle_ipc_request_with_runner(
        Request::ClearOverride {
            monitor_id: Some(String::from("internal")),
            global: false,
        },
        &FakeRunner::new(),
        now,
    );
    assert!(clear_outcome.tick_attempted);
    assert!(matches!(clear_response.response, Response::Ack { .. }));
    assert_eq!(runtime.state.manual_override, None);
    assert!(runtime.external_holds.is_empty());
}

#[test]
fn ipc_clear_external_hold_clears_all_monitors_and_queues_smooth_apply() {
    let temp = TempDir::new();
    let mut report = test_config_report();
    let mut second = report.config.monitors[0].clone();
    second.logical_id = String::from("external");
    second.selector.sysfs_path = Some(String::from("/sys/class/backlight/acpi_video0"));
    report.config.monitors.push(second);
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        report,
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: 1_800_000_000,
            until_epoch_s: 1_800_001_800,
            observed_percent: 20,
        },
    );
    runtime.external_holds.insert(
        String::from("external"),
        apply::ExternalHold {
            since_epoch_s: 1_800_000_000,
            until_epoch_s: 1_800_001_800,
            observed_percent: 70,
        },
    );

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::ClearExternalHold { monitor: None },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(outcome.tick_attempted);
    assert!(matches!(response.response, Response::Ack { .. }));
    assert!(runtime.external_holds.is_empty());
    assert!(runtime.has_pending_apply());
    assert_eq!(runtime.take_pending_apply(), Some((false, false)));
}

#[test]
fn ipc_clear_external_hold_clears_only_the_requested_monitor() {
    let temp = TempDir::new();
    let mut report = test_config_report();
    let mut second = report.config.monitors[0].clone();
    second.logical_id = String::from("external");
    second.selector.sysfs_path = Some(String::from("/sys/class/backlight/acpi_video0"));
    report.config.monitors.push(second);
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        report,
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    for monitor_id in ["internal", "external"] {
        runtime.external_holds.insert(
            String::from(monitor_id),
            apply::ExternalHold {
                since_epoch_s: 1_800_000_000,
                until_epoch_s: 1_800_001_800,
                observed_percent: 42,
            },
        );
    }

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::ClearExternalHold {
            monitor: Some(String::from("internal")),
        },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(outcome.tick_attempted);
    assert!(matches!(response.response, Response::Ack { .. }));
    assert!(!runtime.external_holds.contains_key("internal"));
    assert!(runtime.external_holds.contains_key("external"));
    assert_eq!(runtime.take_pending_apply(), Some((false, false)));
}

#[test]
fn ipc_clear_external_hold_rejects_unknown_monitor_without_mutation() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: 1_800_000_000,
            until_epoch_s: 1_800_001_800,
            observed_percent: 20,
        },
    );

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::ClearExternalHold {
            monitor: Some(String::from("missing")),
        },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(!outcome.tick_attempted);
    assert!(matches!(
        response.response,
        Response::Error {
            code: crate::ipc::ErrorCode::InvalidRequest,
            ..
        }
    ));
    assert!(runtime.external_holds.contains_key("internal"));
    assert!(!runtime.has_pending_apply());
}

#[test]
fn deferred_mutation_and_following_status_do_not_wait_for_apply() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");
    let runner = SlowApplyRunner;

    let started = std::time::Instant::now();
    let (set_response, _) = runtime.handle_ipc_request_with_runner(
        Request::SetOverride {
            monitor_id: Some(String::from("internal")),
            percent: 36,
            minutes: Some(10),
        },
        &runner,
        now,
    );
    assert!(started.elapsed() < Duration::from_millis(200));
    assert!(matches!(set_response.response, Response::Ack { .. }));

    let status_started = std::time::Instant::now();
    let (status_response, _) =
        runtime.handle_ipc_request_with_runner(Request::Status, &runner, now);
    assert!(status_started.elapsed() < Duration::from_millis(200));
    assert!(matches!(status_response.response, Response::Status { .. }));
}

#[test]
fn ipc_set_monitor_limits_reports_persistence_errors_without_mutating_config() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    let unwritable_path = temp.path().join("config-directory");
    std::fs::create_dir_all(&unwritable_path).expect("config directory");
    runtime.config.path = unwritable_path;

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::SetMonitorLimits {
            monitor_id: String::from("internal"),
            min_pct: 30,
            max_pct: 80,
        },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(matches!(response.response, Response::Error { .. }));
    let monitor = runtime
        .config
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == "internal")
        .expect("configured monitor");
    assert_eq!((monitor.min_pct, monitor.max_pct), (10, 100));
}

#[test]
fn ipc_set_cloud_dimming_persists_and_status_reports_the_new_value() {
    let temp = TempDir::new();
    let config_path = temp.path().join("config.toml");
    let mut report = test_weather_config_report();
    report.path = config_path.clone();
    config::save_to_path(&report.config, &config_path).expect("fixture config should save");
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(report, state_path, socket_path)
        .expect("runtime should bootstrap");

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::SetCloudDimming { percent: 40 },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );
    assert!(matches!(response.response, Response::Ack { .. }));

    let persisted = config::parse_text(
        &std::fs::read_to_string(&config_path).expect("config should be readable"),
        &config_path,
    )
    .expect("persisted config should validate");
    assert!((persisted.weather.min_multiplier - 0.6).abs() < f64::EPSILON);
    let status = runtime.status_response(1_800_000_000);
    assert_eq!(
        status
            .weather
            .expect("weather status should be present")
            .cloud_dimming_percent,
        40
    );
    assert!(runtime.has_pending_apply());
}

#[test]
fn ipc_set_monitor_limits_edits_only_that_monitor_without_reloading_disk_config() {
    let temp = TempDir::new();
    let config_path = temp.path().join("config.toml");
    let mut report = test_config_report();
    report.path = config_path.clone();
    config::save_to_path(&report.config, &config_path).expect("fixture config should save");
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(report.clone(), state_path, socket_path)
        .expect("runtime should bootstrap");

    // The user edits an unrelated key on disk without reloading the daemon.
    let mut on_disk = report.config.clone();
    on_disk.daemon.tick_seconds = 120;
    on_disk.location.latitude = 52.52;
    config::save_to_path(&on_disk, &config_path).expect("user edit should save");

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::SetMonitorLimits {
            monitor_id: String::from("internal"),
            min_pct: 30,
            max_pct: 80,
        },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );
    assert!(matches!(response.response, Response::Ack { .. }));
    assert!(!outcome.config_reloaded);

    let monitor = runtime
        .config
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == "internal")
        .expect("configured monitor");
    assert_eq!((monitor.min_pct, monitor.max_pct), (30, 80));
    assert_eq!(runtime.config.daemon.tick_seconds, 60);
    assert!((runtime.config.location.latitude - 41.0082).abs() < 1e-9);

    let persisted = config::parse_text(
        &std::fs::read_to_string(&config_path).expect("config should be readable"),
        &config_path,
    )
    .expect("persisted config should validate");
    assert_eq!(persisted.daemon.tick_seconds, 120);
    assert!((persisted.location.latitude - 52.52).abs() < 1e-9);
    let persisted_monitor = persisted
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == "internal")
        .expect("persisted monitor");
    assert_eq!(
        (persisted_monitor.min_pct, persisted_monitor.max_pct),
        (30, 80)
    );
}

#[test]
fn ipc_set_monitor_limits_accepts_client_json_and_echoes_updated_limits_in_status() {
    let temp = TempDir::new();
    let config_path = temp.path().join("config.toml");
    let mut report = test_config_report();
    report.path = config_path.clone();
    config::save_to_path(&report.config, &config_path).expect("fixture config should save");
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(report, state_path, socket_path)
        .expect("runtime should bootstrap");

    let client_json = r#"{"max_pct":82,"min_pct":22,"monitor_id":"internal","request":"set_monitor_limits","version":1}"#;
    let envelope: ipc::RequestEnvelope =
        serde_json::from_str(client_json).expect("client set_monitor_limits JSON should parse");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");
    let (response, _) =
        runtime.handle_ipc_request_with_runner(envelope.request, &FakeRunner::new(), now);
    assert!(matches!(response.response, Response::Ack { .. }));

    let (status_reply, _) =
        runtime.handle_ipc_request_with_runner(Request::Status, &FakeRunner::new(), now);
    let Response::Status { status } = status_reply.response else {
        panic!("expected status response, got {:?}", status_reply.response);
    };
    let internal = status
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == "internal")
        .expect("internal monitor in status");
    assert_eq!((internal.min_pct, internal.max_pct), (Some(22), Some(82)));
}

#[test]
fn ipc_set_cloud_dimming_edits_only_weather_without_reloading_disk_config() {
    let temp = TempDir::new();
    let config_path = temp.path().join("config.toml");
    let mut report = test_weather_config_report();
    report.path = config_path.clone();
    config::save_to_path(&report.config, &config_path).expect("fixture config should save");
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(report.clone(), state_path, socket_path)
        .expect("runtime should bootstrap");

    let mut on_disk = report.config.clone();
    on_disk.daemon.tick_seconds = 120;
    on_disk.monitors[0].max_pct = 70;
    config::save_to_path(&on_disk, &config_path).expect("user edit should save");

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::SetCloudDimming { percent: 40 },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );
    assert!(matches!(response.response, Response::Ack { .. }));
    assert_eq!(runtime.config.weather.cloud_dimming_percent(), 40);
    assert_eq!(runtime.config.daemon.tick_seconds, 60);
    assert_eq!(runtime.config.monitors[0].max_pct, 100);

    let persisted = config::parse_text(
        &std::fs::read_to_string(&config_path).expect("config should be readable"),
        &config_path,
    )
    .expect("persisted config should validate");
    assert!((persisted.weather.min_multiplier - 0.6).abs() < f64::EPSILON);
    assert_eq!(persisted.daemon.tick_seconds, 120);
    assert_eq!(persisted.monitors[0].max_pct, 70);
}

#[test]
fn ipc_set_cloud_dimming_rejects_values_above_fifty() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_weather_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::SetCloudDimming { percent: 51 },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );
    assert!(matches!(response.response, Response::Error { .. }));
    assert_eq!(runtime.config.weather.cloud_dimming_percent(), 25);
}

#[test]
fn ipc_set_cloud_dimming_reports_persistence_errors_without_mutating_config() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_weather_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    let unwritable_path = temp.path().join("config-directory");
    std::fs::create_dir_all(&unwritable_path).expect("config directory");
    runtime.config.path = unwritable_path;

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::SetCloudDimming { percent: 40 },
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );
    assert!(matches!(response.response, Response::Error { .. }));
    assert_eq!(runtime.config.weather.cloud_dimming_percent(), 25);
}

#[test]
#[cfg(target_os = "linux")]
fn ipc_set_override_reports_immediate_apply_failures_without_losing_state() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    let runner = FakeRunner::new().with_output(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "intel_backlight",
            "set",
            "36%",
        ],
        Some(1),
        "",
        "permission denied",
    );

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::SetOverride {
            monitor_id: Some(String::from("internal")),
            percent: 36,
            minutes: Some(10),
        },
        &runner,
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(outcome.tick_attempted);

    assert!(matches!(response.response, Response::Ack { .. }));
    assert!(runtime.pending_apply);
    assert_eq!(
        runtime
            .state
            .manual_override
            .as_ref()
            .and_then(|manual_override| manual_override.target_percent("internal", 1_800_000_000)),
        Some(36)
    );
    assert_eq!(
        RuntimeState::load_from_path(&state_path)
            .expect("state file should load")
            .manual_override
            .as_ref()
            .and_then(|manual_override| manual_override.target_percent("internal", 1_800_000_000)),
        Some(36)
    );
}
