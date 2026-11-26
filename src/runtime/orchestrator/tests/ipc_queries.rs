//! Read-only IPC requests: status, themes and the day curve.

use super::*;

#[test]
fn ipc_status_reports_config_and_last_applied_values() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_weather_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    runtime.state.suspend_until_epoch_s = Some(1_800_000_100);
    runtime
        .state
        .record_apply_success("internal", 41, 1_800_000_000);
    runtime.external_holds.insert(
        String::from("internal"),
        apply::ExternalHold {
            since_epoch_s: 1_800_000_010,
            until_epoch_s: 1_800_000_090,
            observed_percent: 23,
        },
    );
    runtime.state.weather = Some(crate::state::WeatherSnapshotMetadata {
        provider: String::from("openweather"),
        fetched_at_epoch_s: 1_800_000_000,
        valid_at_epoch_s: 1_700_000_000,
        source_kind: crate::weather::WeatherSourceKind::Forecast,
        cloud_cover_percent: Some(50),
        smoothed_cloud_cover_percent: Some(50),
        temperature: Some(0.0),
        condition: crate::weather::WeatherCondition::PartlyCloudy,
        condition_description: Some(String::from("broken clouds")),
        day_phase: Some(crate::weather::WeatherDayPhase::Day),
        forecast: vec![],
        ..Default::default()
    });
    runtime.weather_refresh.last_attempted_at_epoch_s = Some(1_800_000_020);
    runtime.weather_refresh.next_refresh_at_epoch_s = Some(1_800_001_800);
    runtime.weather_refresh.consecutive_failures = 2;
    runtime.weather_refresh.last_error =
        Some(String::from("openweather request failed: network timeout"));

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::Status,
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_030, 0)
            .single()
            .expect("valid time"),
    );

    assert!(!outcome.tick_attempted);

    let Response::Status { status } = response.response else {
        panic!("status request should return a status response");
    };

    assert!(status.daemon_alive);
    assert_eq!(status.tick_seconds, 60);
    assert_eq!(status.configured_monitors, 1);
    assert_eq!(status.stateful_monitors, 1);
    assert!(status.suspended);
    assert!(!status.manual_override_active);
    assert_eq!(status.monitors[0].last_applied_percent, Some(41));
    assert_eq!(
        status.monitors[0]
            .external_hold
            .map(|hold| (hold.remaining_s, hold.observed_percent)),
        Some((60, 23))
    );
    let wire = serde_json::to_value(ipc::ResponseEnvelope::status(status.clone()))
        .expect("status should serialize");
    assert_eq!(
        wire["status"]["monitors"][0]["external_hold"]["remaining_s"],
        60
    );
    assert_eq!(
        wire["status"]["monitors"][0]["external_hold"]["observed_percent"],
        23
    );
    let weather = status.weather.expect("weather status should exist");
    assert!(weather.active);
    assert!(!weather.stale);
    assert_eq!(weather.last_refresh_attempt_epoch_s, Some(1_800_000_020));
    assert_eq!(weather.next_refresh_at_epoch_s, Some(1_800_001_800));
    assert_eq!(weather.consecutive_failures, 2);
    assert_eq!(
        weather.condition,
        crate::weather::WeatherCondition::PartlyCloudy
    );
    assert_eq!(
        weather.condition_description.as_deref(),
        Some("broken clouds")
    );
    assert_eq!(
        weather.day_phase,
        Some(crate::weather::WeatherDayPhase::Day)
    );
    assert_eq!(
        weather.last_error.as_deref(),
        Some("openweather request failed: network timeout")
    );
}

#[test]
fn ipc_status_uses_current_solar_phase_when_weather_snapshot_is_forecast() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut config = test_config_report();
    config.config.weather.enabled = true;
    config.config.weather.api_key = Some(String::from("test-key"));

    let mut runtime = DaemonRuntime::bootstrap_with_paths(config, state_path, socket_path)
        .expect("runtime should bootstrap");
    runtime.last_solar_elevation = Some(-20.6);
    runtime.state.weather = Some(crate::state::WeatherSnapshotMetadata {
        provider: String::from("openweather"),
        fetched_at_epoch_s: 1_800_000_000,
        valid_at_epoch_s: 1_800_010_800,
        source_kind: crate::weather::WeatherSourceKind::Forecast,
        condition: crate::weather::WeatherCondition::Clear,
        day_phase: Some(crate::weather::WeatherDayPhase::Day),
        ..Default::default()
    });

    let status = runtime.status_response(1_800_000_050);
    let weather = status.weather.expect("weather payload should be present");
    assert_eq!(
        weather.day_phase,
        Some(crate::weather::WeatherDayPhase::Night)
    );
}

#[test]
fn ipc_status_reports_topology_unavailability_and_ambiguity_in_configured_order() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut config = test_config_report();
    let mut external = config.config.monitors[0].clone();
    external.logical_id = String::from("external");
    external.backend = BackendKind::Ddc;
    external.selector = MonitorSelector {
        connector: None,
        serial: Some(String::from("external-serial")),
        model: None,
        edid: None,
        sysfs_path: None,
        ddc_bus: None,
        ddc_address: None,
    };
    config.config.monitors.push(external);

    let mut runtime = DaemonRuntime::bootstrap_with_paths(config, state_path, socket_path)
        .expect("runtime should bootstrap");
    runtime.last_capabilities = Some(CapabilitySnapshot {
        ddc_present: vec![
            crate::discovery::DdcMonitorDiscovery {
                occurrence_id: String::from("first"),
                stable_id: String::from("ddc:first"),
                identity_status: crate::discovery::PhysicalIdentityStatus::Unique,
                manufacturer: None,
                model: None,
                serial: Some(String::from("external-serial")),
                display_number: 1,
                bus_number: Some(7),
                connector: None,
                brightness_vcp_supported: Some(true),
                backend_viable: true,
                note: None,
            },
            crate::discovery::DdcMonitorDiscovery {
                occurrence_id: String::from("second"),
                stable_id: String::from("ddc:second"),
                identity_status: crate::discovery::PhysicalIdentityStatus::Unique,
                manufacturer: None,
                model: None,
                serial: Some(String::from("external-serial")),
                display_number: 2,
                bus_number: Some(8),
                connector: None,
                brightness_vcp_supported: Some(true),
                backend_viable: true,
                note: None,
            },
        ],
        backlights_present: Vec::new(),
    });

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::Status,
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_000, 0)
            .single()
            .expect("valid time"),
    );

    assert!(!outcome.tick_attempted);
    let Response::Status { status } = response.response else {
        panic!("status request should return a status response");
    };
    assert_eq!(status.monitors.len(), 2);
    assert_eq!(
        status.monitors[0].topology.as_deref(),
        Some("temporarily_unavailable")
    );
    assert_eq!(
        status.monitors[1].topology.as_deref(),
        Some("ambiguous_or_unsafe")
    );
}

#[test]
fn ipc_status_hides_expired_manual_overrides() {
    let temp = TempDir::new();
    let state_path = temp.path().join("state/runtime-state.json");
    let socket_path = temp.path().join("run/control.sock");
    let mut runtime =
        DaemonRuntime::bootstrap_with_paths(test_config_report(), state_path, socket_path)
            .expect("runtime should bootstrap");
    runtime.state.manual_override = Some(ManualOverrideState {
        global_percent: Some(33),
        global_expires_at_epoch_s: Some(1_800_000_000),
        targets: std::collections::BTreeMap::from([(String::from("internal"), 27)]),
        expires_at_epoch_s: Some(1_800_000_000),
    });

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::Status,
        &FakeRunner::new(),
        Utc.timestamp_opt(1_800_000_001, 0)
            .single()
            .expect("valid time"),
    );

    let Response::Status { status } = response.response else {
        panic!("status request should return a status response");
    };

    assert!(!status.manual_override_active);
    assert_eq!(status.global_override_percent, None);
    assert_eq!(status.global_override_until_epoch_s, None);
    assert_eq!(status.per_monitor_override_until_epoch_s, None);
    assert_eq!(status.monitors[0].override_percent, None);
}

#[test]
fn ipc_theme_requests_serve_the_registry_and_status_carries_full_theme() {
    let temp = TempDir::new();
    let mut report = test_config_report();
    report.config.tui.theme = crate::config::ThemeId::new("handheld");
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        report,
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    let now = Utc.timestamp_opt(1_800_000_000, 0).single().unwrap();
    let runner = FakeRunner::new();

    let (response, _) = runtime.handle_ipc_request_with_runner(Request::Status, &runner, now);
    let Response::Status { status } = response.response else {
        panic!("status response expected");
    };
    let theme = status.theme.expect("status carries the saved theme");
    assert_eq!(theme.id.as_deref(), Some("handheld"));
    assert_eq!(theme.family.as_deref(), Some("pixel"));
    assert_eq!(theme.glyph_style.as_deref(), Some("pixel"));
    let palette = theme.palette.as_ref().expect("palette");
    assert_eq!(palette.bg, theme.bg);
    assert_eq!(palette.accent, theme.accent);
    assert_eq!(palette.bg, "#141c12");
    let widget = theme.widget.as_ref().expect("widget");
    assert_eq!(widget.renderer, "pixel");
    assert_eq!(widget.shades.len(), 4);

    let (response, outcome) =
        runtime.handle_ipc_request_with_runner(Request::ListThemes, &runner, now);
    assert!(!outcome.tick_attempted);
    let Response::Themes { themes } = response.response else {
        panic!("themes response expected");
    };
    assert_eq!(themes.len(), 41);
    assert!(themes
        .iter()
        .any(|t| t.id == "vfd_hifi" && t.family == "instrument"));

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::GetTheme {
            id: String::from("gameboy"),
        },
        &runner,
        now,
    );
    let Response::Theme { theme: by_alias } = response.response else {
        panic!("theme response expected");
    };
    assert_eq!(by_alias, theme);

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::GetTheme {
            id: String::from("noord"),
        },
        &runner,
        now,
    );
    let Response::Error { code, message } = response.response else {
        panic!("error response expected");
    };
    assert_eq!(code, ipc::ErrorCode::InvalidRequest);
    assert!(message.contains("did you mean `nord`"), "{message}");
}

#[test]
fn reload_with_broken_user_theme_keeps_daemon_running_and_warns_in_status() {
    let temp = TempDir::new();
    let themes = temp.path().join("themes");
    fs::create_dir_all(&themes).unwrap();
    fs::write(themes.join("broken.toml"), "id = \"broken\"\nname = [\n").unwrap();
    fs::write(
        themes.join("nord.toml"),
        "id = \"nord\"\nname = \"Nord\"\nfamily = \"retro\"\n",
    )
    .unwrap();
    fs::write(
        themes.join("good.toml"),
        "id = \"good\"\nname = \"Good\"\nextends = \"dracula\"\n",
    )
    .unwrap();
    let registry = crate::theme::Registry::load_dir(&themes);
    assert_eq!(registry.diagnostics().len(), 2);

    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    let now = Utc.timestamp_opt(1_800_000_000, 0).single().unwrap();
    let status_theme = |runtime: &mut DaemonRuntime| {
        let (response, _) =
            runtime.handle_ipc_request_with_runner(Request::Status, &FakeRunner::new(), now);
        let Response::Status { status } = response.response else {
            panic!("daemon must keep answering status");
        };
        (status.theme.unwrap().id.unwrap(), status.theme_warning)
    };
    let with_theme = |id: &str| {
        let mut report = test_config_report();
        report.config.tui.theme = crate::config::ThemeId::new(id);
        move || Ok(report)
    };

    // Broken user-only theme: reload succeeds, default theme + warning.
    runtime
        .reload_config_with_themes(with_theme("broken"), &registry)
        .expect("a broken theme file must not fail reload");
    let (id, warning) = status_theme(&mut runtime);
    assert_eq!(id, "amber");
    let warning = warning.expect("status carries the theme warning");
    assert!(warning.contains("broken.toml"), "{warning}");

    // Broken file shadowing a built-in: the built-in, plus warning.
    runtime
        .reload_config_with_themes(with_theme("nord"), &registry)
        .unwrap();
    let (id, warning) = status_theme(&mut runtime);
    assert_eq!(id, "nord");
    assert!(warning.unwrap().contains("unknown family `retro`"));

    // Typo in tui.theme: also non-fatal.
    runtime
        .reload_config_with_themes(with_theme("noord"), &registry)
        .unwrap();
    let (id, warning) = status_theme(&mut runtime);
    assert_eq!(id, "amber");
    assert!(warning.unwrap().contains("did you mean `nord`"));

    // The valid theme next to the broken ones loads, warning clears.
    runtime
        .reload_config_with_themes(with_theme("good"), &registry)
        .unwrap();
    assert_eq!(status_theme(&mut runtime), (String::from("good"), None));

    // And start-up with a broken theme in the config is fine too.
    let mut report = test_config_report();
    report.config.tui.theme = crate::config::ThemeId::new("broken");
    let started = DaemonRuntime::bootstrap_with_paths(
        report,
        temp.path().join("state2/runtime-state.json"),
        temp.path().join("run2/control.sock"),
    )
    .expect("an unresolvable theme must not stop the daemon from starting");
    assert_eq!(started.config.tui_theme.id, "amber");
    assert!(started.config.tui_theme_warning.is_some());
}

#[test]
fn ipc_day_curve_is_bounded_monotone_and_cached() {
    let temp = TempDir::new();
    let mut runtime = DaemonRuntime::bootstrap_with_paths(
        test_config_report(),
        temp.path().join("state/runtime-state.json"),
        temp.path().join("run/control.sock"),
    )
    .expect("runtime should bootstrap");
    // 2027-01-15 08:00 UTC = 11:00 Europe/Istanbul.
    let now = Utc.with_ymd_and_hms(2027, 1, 15, 8, 0, 0).unwrap();
    let runner = RecordingRunner::new();

    let (response, outcome) = runtime.handle_ipc_request_with_runner(
        Request::DayCurve { monitor_id: None },
        &runner,
        now,
    );
    assert!(!outcome.tick_attempted);
    let Response::DayCurve { day_curve } = response.response else {
        panic!("day_curve response expected, got {response:?}");
    };
    assert_eq!(day_curve.date, "2027-01-15");
    assert_eq!(day_curve.step_minutes, 10);
    assert_eq!(day_curve.generated_at_epoch_s, now.timestamp() as u64);
    assert_eq!(day_curve.weather_multiplier, None);
    assert!(day_curve.milestones_error.is_none());
    let sun = &day_curve.sun;
    let midnight = Utc
        .with_ymd_and_hms(2027, 1, 14, 21, 0, 0)
        .unwrap()
        .timestamp();
    for event in [sun.dawn, sun.sunrise, sun.solar_noon, sun.sunset, sun.dusk] {
        let t = event.expect("mid-latitude events exist");
        assert!((midnight..midnight + 86_400).contains(&t));
    }
    assert_eq!(day_curve.monitors.len(), 1);
    let monitor = &day_curve.monitors[0];
    assert_eq!(monitor.monitor_id, "internal");
    assert_eq!((monitor.min_pct, monitor.max_pct), (10, 100));
    assert_eq!(monitor.samples.len(), 145);
    assert_eq!(monitor.samples[0].t, midnight);
    assert!(monitor.samples.windows(2).all(|w| w[1].t - w[0].t == 600));
    assert!(monitor.samples.iter().all(|s| (10..=100).contains(&s.pct)
        && s.pct <= s.pct_no_weather
        && s.pct == s.pct_no_weather));
    assert!(monitor.samples.iter().any(|s| s.pct > 10));
    assert_eq!(monitor.milestones.len(), 9);
    assert_eq!(monitor.milestones[0].id, "rise_start");
    assert!(monitor.milestones.windows(2).all(|w| w[0].t < w[1].t));
    #[cfg(target_os = "linux")]
    assert!(
        runner.calls().is_empty(),
        "day_curve must not touch hardware"
    );

    // Same day, same config: served from the cache.
    let later = now + chrono::Duration::minutes(5);
    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::DayCurve { monitor_id: None },
        &runner,
        later,
    );
    let Response::DayCurve { day_curve: cached } = response.response else {
        panic!("day_curve response expected");
    };
    assert_eq!(cached, day_curve);

    // A limit change invalidates the cache through the key.
    runtime.config.monitors[0].max_pct = 50;
    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::DayCurve {
            monitor_id: Some(String::from("internal")),
        },
        &runner,
        later,
    );
    let Response::DayCurve { day_curve: fresh } = response.response else {
        panic!("day_curve response expected");
    };
    assert_eq!(fresh.monitors[0].max_pct, 50);
    assert!(fresh.monitors[0].samples.iter().all(|s| s.pct <= 50));

    let (response, _) = runtime.handle_ipc_request_with_runner(
        Request::DayCurve {
            monitor_id: Some(String::from("missing")),
        },
        &runner,
        later,
    );
    assert!(matches!(
        response.response,
        Response::Error {
            code: ipc::ErrorCode::InvalidRequest,
            ..
        }
    ));
}
