//! Config reload keeps the previous config on failure and prunes state.

use super::*;

#[test]
fn reload_config_failure_keeps_previous_config() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    let previous_tick_seconds = runtime.config.daemon.tick_seconds;

    let error = runtime
        .reload_config_with(|| {
            Err(ConfigError::AlreadyExists(PathBuf::from(
                "/tmp/invalid.toml",
            )))
        })
        .expect_err("reload should fail");

    assert_eq!(runtime.config.daemon.tick_seconds, previous_tick_seconds);
    assert!(error.to_string().contains("config file already exists"));
}

#[test]
fn reload_config_prunes_stale_state_without_crashing() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path.clone(), socket_path)
            .expect("runtime should bootstrap");
    runtime
        .state
        .record_apply_success("stale", 70, 1_800_000_000);
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: None,
        global_expires_at_epoch_s: None,
        targets: std::collections::BTreeMap::from([
            (String::from("internal"), 25),
            (String::from("stale"), 70),
        ]),
        expires_at_epoch_s: Some(1_800_000_100),
    });
    runtime.weather_refresh.next_refresh_at_epoch_s = Some(1_800_000_500);
    runtime.weather_refresh.last_attempted_at_epoch_s = Some(1_800_000_300);
    runtime.weather_refresh.consecutive_failures = 3;
    runtime.weather_refresh.last_error = Some(String::from("temporary outage"));

    runtime
        .reload_config_with(|| Ok(test_config_report()))
        .expect("reload should succeed");

    assert!(runtime.state.monitor("stale").is_none());
    assert_eq!(runtime.weather_refresh.next_refresh_at_epoch_s, None);
    assert_eq!(runtime.weather_refresh.last_attempted_at_epoch_s, None);
    assert_eq!(runtime.weather_refresh.consecutive_failures, 0);
    assert_eq!(runtime.weather_refresh.last_error, None);
    assert_eq!(
        runtime
            .state
            .manual_override
            .as_ref()
            .and_then(|manual_override| manual_override.target_percent("internal", 1_800_000_000)),
        Some(25)
    );
    assert_eq!(
        runtime
            .state
            .manual_override
            .as_ref()
            .and_then(|manual_override| manual_override.target_percent("stale", 1_800_000_000)),
        None
    );
    let persisted = RuntimeState::load_from_path(&state_path).expect("state should persist");
    assert!(persisted.monitor("stale").is_none());
}
