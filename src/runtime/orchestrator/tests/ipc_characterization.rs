//! Byte-exact replies for every IPC request variant.

use super::*;

/// Runs `requests` in order on a fresh runtime and renders every reply as
/// `wire-json | tick=.. reload=.. pending=..` for byte-exact comparison.
fn characterize_ipc(report: ConfigReport, requests: Vec<Request>) -> Vec<String> {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        report,
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    let now = Utc
        .timestamp_opt(1_800_000_000, 0)
        .single()
        .expect("valid time");
    requests
        .into_iter()
        .map(|request| {
            let (response, outcome) =
                runtime.handle_ipc_request_with_runner(request, &FakeRunner::new(), now);
            format!(
                "{} | tick={} reload={} pending={:?}",
                serde_json::to_string(&response).expect("response serializes"),
                outcome.tick_attempted,
                outcome.config_reloaded,
                runtime.take_pending_apply(),
            )
        })
        .collect()
}

#[test]
fn ipc_characterization_ping_replies_pong() {
    assert_eq!(
        characterize_ipc(test_dry_run_config_report(), vec![Request::Ping]),
        [
            r#"{"version":1,"response":"pong","message":"pong"} | tick=false reload=false pending=None"#,
        ]
    );
}

#[test]
fn ipc_characterization_suspend_validation_and_suspended_external_change() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![
                Request::Suspend { minutes: Some(0) },
                Request::Suspend { minutes: Some(5) },
                Request::Suspend { minutes: None },
                Request::ExternalBrightnessChange,
                Request::Resume,
            ]
        ),
        [
            r#"{"version":1,"response":"error","code":"invalid_request","message":"suspend minutes must be greater than zero"} | tick=false reload=false pending=None"#,
            r#"{"version":1,"response":"ack","message":"suspended until epoch 1800000300 (5 minute(s))"} | tick=false reload=false pending=Some((false, false))"#,
            r#"{"version":1,"response":"ack","message":"suspended until resume"} | tick=false reload=false pending=Some((false, false))"#,
            r#"{"version":1,"response":"ack","message":"external brightness change observed while suspended"} | tick=false reload=false pending=None"#,
            r#"{"version":1,"response":"ack","message":"resumed automatic apply and cleared manual overrides; apply queued"} | tick=true reload=false pending=Some((true, true))"#,
        ]
    );
}

#[test]
fn ipc_characterization_idle_dim_and_wake_queue_forced_apply() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![Request::IdleDim, Request::IdleWake]
        ),
        [
            r#"{"version":1,"response":"ack","message":"desktop idle dimming active; apply queued"} | tick=true reload=false pending=Some((true, false))"#,
            r#"{"version":1,"response":"ack","message":"desktop idle wake queued for re-observation"} | tick=true reload=false pending=Some((true, false))"#,
        ]
    );
}

#[test]
fn ipc_characterization_refresh_weather_rejected_when_disabled() {
    assert_eq!(
        characterize_ipc(test_dry_run_config_report(), vec![Request::RefreshWeather]),
        [
            r#"{"version":1,"response":"error","code":"invalid_request","message":"weather is disabled in configuration"} | tick=false reload=false pending=None"#,
        ]
    );
}

#[test]
fn ipc_characterization_refresh_weather_requested_when_enabled() {
    assert_eq!(
        characterize_ipc(test_weather_config_report(), vec![Request::RefreshWeather]),
        [
            r#"{"version":1,"response":"ack","message":"weather refresh requested"} | tick=false reload=false pending=None"#,
        ]
    );
}

#[test]
fn ipc_characterization_external_brightness_change_reports_held_monitors() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![Request::ExternalBrightnessChange]
        ),
        [
            r#"{"version":1,"response":"ack","message":"external brightness change observed; 0 monitor(s) held"} | tick=false reload=false pending=None"#,
        ]
    );
}

#[test]
fn ipc_characterization_run_once_only_queues_apply() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![
                Request::RunOnce { force: true },
                Request::RunOnce { force: false },
            ]
        ),
        [
            r#"{"version":1,"response":"run_once","run_once":{"tick_duration_ms":0,"monitors_evaluated":0,"writes_attempted":0,"writes_skipped":0,"writes_succeeded":0,"writes_failed":0,"queued":true},"message":"daemon tick queued"} | tick=true reload=false pending=Some((true, false))"#,
            r#"{"version":1,"response":"run_once","run_once":{"tick_duration_ms":0,"monitors_evaluated":0,"writes_attempted":0,"writes_skipped":0,"writes_succeeded":0,"writes_failed":0,"queued":true},"message":"daemon tick queued"} | tick=true reload=false pending=Some((false, false))"#,
        ]
    );
}

#[test]
fn ipc_characterization_override_validation_and_messages() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![
                Request::SetOverride {
                    monitor_id: None,
                    percent: 101,
                    minutes: None
                },
                Request::SetOverride {
                    monitor_id: None,
                    percent: 50,
                    minutes: Some(0)
                },
                Request::SetOverride {
                    monitor_id: Some(String::from("nope")),
                    percent: 50,
                    minutes: None
                },
                Request::SetOverride {
                    monitor_id: None,
                    percent: 50,
                    minutes: None
                },
                Request::ClearOverride {
                    monitor_id: Some(String::from("internal")),
                    global: true
                },
                Request::ClearOverride {
                    monitor_id: Some(String::from("nope")),
                    global: false
                },
                Request::ClearOverride {
                    monitor_id: None,
                    global: true
                },
                Request::ClearOverride {
                    monitor_id: None,
                    global: false
                },
            ]
        ),
        [
            r#"{"version":1,"response":"error","code":"invalid_request","message":"override percent must be in the range 0..=100"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"override minutes must be greater than zero"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"unknown monitor logical_id `nope`"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"ack","message":"set global manual override to 50%; apply queued"} | tick=true reload=false pending=Some((true, false))"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"clear-override accepts either --monitor-id or --global"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"unknown monitor logical_id `nope`"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"ack","message":"cleared global manual override; apply queued"} | tick=true reload=false pending=Some((true, false))"#,
            r#"{"version":1,"response":"ack","message":"no manual overrides were active; apply queued"} | tick=true reload=false pending=Some((true, false))"#,
        ]
    );
}

#[test]
fn ipc_characterization_limits_and_cloud_dimming_validation() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![
                Request::SetMonitorLimits {
                    monitor_id: String::from("internal"),
                    min_pct: 0,
                    max_pct: 101
                },
                Request::SetMonitorLimits {
                    monitor_id: String::from("internal"),
                    min_pct: 60,
                    max_pct: 50
                },
                Request::SetMonitorLimits {
                    monitor_id: String::from("nope"),
                    min_pct: 10,
                    max_pct: 50
                },
                Request::SetCloudDimming { percent: 51 },
            ]
        ),
        [
            r#"{"version":1,"response":"error","code":"invalid_request","message":"limits must be in the range 0..=100"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"min_pct (60) cannot exceed max_pct (50)"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"unknown monitor logical_id `nope`"} | tick=true reload=false pending=None"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"cloud dimming percent must be within 0..=50"} | tick=false reload=false pending=None"#,
        ]
    );
}

#[test]
fn ipc_characterization_clear_external_hold_messages() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![
                Request::ClearExternalHold {
                    monitor: Some(String::from("nope"))
                },
                Request::ClearExternalHold {
                    monitor: Some(String::from("internal"))
                },
                Request::ClearExternalHold { monitor: None },
            ]
        ),
        [
            r#"{"version":1,"response":"error","code":"invalid_request","message":"unknown monitor logical_id `nope`"} | tick=false reload=false pending=None"#,
            r#"{"version":1,"response":"ack","message":"cleared external brightness hold for internal; apply queued"} | tick=true reload=false pending=Some((false, false))"#,
            r#"{"version":1,"response":"ack","message":"cleared external brightness holds; apply queued"} | tick=true reload=false pending=Some((false, false))"#,
        ]
    );
}

#[test]
fn ipc_characterization_get_theme_rejects_unknown_id() {
    assert_eq!(
        characterize_ipc(
            test_dry_run_config_report(),
            vec![Request::GetTheme {
                id: String::from("no-such-theme")
            },]
        ),
        [
            r#"{"version":1,"response":"error","code":"invalid_request","message":"unknown theme `no-such-theme`"} | tick=false reload=false pending=None"#,
        ]
    );
}

#[test]
fn ipc_characterization_reload_config_reply_reports_path_and_reload_outcome() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_dry_run_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");

    let result = runtime.reload_config_with(|| Ok(test_dry_run_config_report()));
    let (response, outcome) = runtime.reload_config_reply(result);
    assert_eq!(
        serde_json::to_string(&response).expect("response serializes"),
        r#"{"version":1,"response":"ack","message":"reloaded config from /tmp/sunreactor-config.toml; apply queued"}"#
    );
    assert_eq!(
        outcome,
        IpcOutcome {
            tick_attempted: true,
            config_reloaded: true,
        }
    );
    assert_eq!(runtime.take_pending_apply(), Some((true, false)));

    let result = runtime.reload_config_with(|| {
        Err(ConfigError::AlreadyExists(PathBuf::from(
            "/tmp/invalid.toml",
        )))
    });
    let (response, outcome) = runtime.reload_config_reply(result);
    let Response::Error { code, message } = response.response else {
        panic!("reload failure must reply with an error");
    };
    assert_eq!(code, ipc::ErrorCode::InternalError);
    assert!(message.contains("config file already exists"));
    assert_eq!(outcome, IpcOutcome::default());
    assert_eq!(runtime.take_pending_apply(), None);
}
