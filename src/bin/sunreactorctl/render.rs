use sunreactor::{
    config::{self, ConfigSource},
    ipc::{self, ExternalHoldStatus, Response, StatusResponse, WriteBudgetStatus},
    paths,
};

pub(crate) fn render_theme_list(
    registry: &sunreactor::theme::Registry,
    current: Option<&str>,
) -> String {
    let mut lines = vec![format!(
        "  {:<18} {:<11} {:<8} {}",
        "ID", "FAMILY", "SOURCE", "NAME"
    )];
    for theme in registry.themes() {
        let marker = if Some(theme.id.as_str()) == current {
            '*'
        } else {
            ' '
        };
        let aliases = if theme.aliases.is_empty() {
            String::new()
        } else {
            format!("  (aliases: {})", theme.aliases.join(", "))
        };
        lines.push(format!(
            "{marker} {:<18} {:<11} {:<8} {}{aliases}",
            theme.id,
            theme.family.as_str(),
            theme.source.label(),
            theme.name,
        ));
    }
    let invalid = registry.diagnostics().len();
    if invalid > 0 {
        lines.push(format!(
            "\n{invalid} invalid user theme file(s) skipped; run `sunreactorctl theme validate` for details"
        ));
    }
    lines.join("\n")
}

pub(crate) fn render_theme(theme: &sunreactor::theme::Theme) -> String {
    let source = match &theme.source {
        sunreactor::theme::ThemeSource::Builtin => String::from("builtin"),
        sunreactor::theme::ThemeSource::User(path) => format!("user ({})", path.display()),
    };
    let widget = &theme.widget;
    let mut lines = vec![
        format!("id: {}", theme.id),
        format!("name: {}", theme.name),
        format!("family: {}", theme.family),
        format!("source: {source}"),
        format!(
            "aliases: {}",
            if theme.aliases.is_empty() {
                String::from("none")
            } else {
                theme.aliases.join(", ")
            }
        ),
    ];
    if let Some(extends) = &theme.extends {
        lines.push(format!("extends: {extends}"));
    }
    lines.push(String::from("palette:"));
    for (key, colour) in theme.palette.entries() {
        lines.push(format!("  {key}: {colour}"));
    }
    lines.push(format!("tui.glyph_style: {}", theme.tui.glyph_style));
    lines.push(String::from("widget:"));
    lines.push(format!("  renderer: {}", widget.renderer));
    lines.push(format!("  font: {}", widget.font));
    lines.push(format!("  pixel_scale: {}", widget.pixel_scale));
    lines.push(format!(
        "  shades: {}",
        widget
            .shades
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    ));
    lines.push(format!("  density: {}", widget.density));
    lines.push(format!("  border: {}", widget.border));
    lines.push(format!("  radius: {}", widget.radius));
    lines.push(format!("  dither: {}", widget.dither));
    lines.push(format!("  glow: {}", widget.glow));
    lines.push(format!(
        "  modules: {}",
        widget
            .modules
            .iter()
            .map(|module| module.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.join("\n")
}

pub(crate) fn render_ipc_response(response: ipc::ResponseEnvelope) -> anyhow::Result<String> {
    match response.response {
        Response::Pong { message } | Response::Ack { message } => Ok(message),
        Response::Status { status } => Ok(render_status(&status)),
        Response::RunOnce { run_once, message } if run_once.queued => {
            Ok(format!("{message}\nqueued — apply runs in background"))
        }
        Response::RunOnce { run_once, message } => Ok([
            message,
            format!("tick_duration_ms: {}", run_once.tick_duration_ms),
            format!("monitors_evaluated: {}", run_once.monitors_evaluated),
            format!("writes_attempted: {}", run_once.writes_attempted),
            format!("writes_skipped: {}", run_once.writes_skipped),
            format!("writes_succeeded: {}", run_once.writes_succeeded),
            format!("writes_failed: {}", run_once.writes_failed),
        ]
        .join("\n")),
        Response::Themes { themes } => Ok(serde_json::to_string_pretty(&themes)?),
        Response::Theme { theme } => Ok(serde_json::to_string_pretty(&theme)?),
        Response::DayCurve { day_curve } => Ok(serde_json::to_string_pretty(&day_curve)?),
        Response::Error { message, .. } => anyhow::bail!(message),
    }
}

pub(crate) fn render_status(status: &StatusResponse) -> String {
    let suspend_until = if status.suspended && status.suspend_until_epoch_s.is_none() {
        String::from("indefinite")
    } else {
        optional_u64(status.suspend_until_epoch_s)
    };
    let mut lines = vec![
        format!("daemon_alive: {}", status.daemon_alive),
        format!("config_path: {}", status.config_path),
        format!("tick_seconds: {}", status.tick_seconds),
        format!("dry_run: {}", status.dry_run),
        format!("suspended: {}", status.suspended),
        format!("suspend_until_epoch_s: {suspend_until}"),
        format!("manual_override_active: {}", status.manual_override_active),
        format!(
            "per_monitor_override_until_epoch_s: {}",
            optional_u64(status.per_monitor_override_until_epoch_s)
        ),
        format!(
            "global_override_percent: {}",
            optional_u8(status.global_override_percent)
        ),
        format!(
            "global_override_until_epoch_s: {}",
            optional_u64(status.global_override_until_epoch_s)
        ),
        format!("configured_monitors: {}", status.configured_monitors),
        format!("stateful_monitors: {}", status.stateful_monitors),
    ];

    push_theme_lines(&mut lines, status);
    push_weather_lines(&mut lines, status);
    push_monitor_lines(&mut lines, status);

    lines.join("\n")
}

fn push_theme_lines(lines: &mut Vec<String>, status: &StatusResponse) {
    if let Some(theme) = &status.theme {
        lines.push(format!("theme: {}", theme.name));
        if let Some(id) = &theme.id {
            lines.push(format!("theme_id: {id}"));
        }
        if let Some(warning) = &status.theme_warning {
            lines.push(format!("theme_warning: {}", warning.replace('\n', " ")));
        }
        lines.push(format!("theme_accent: {}", theme.accent));
    }
}

fn push_weather_lines(lines: &mut Vec<String>, status: &StatusResponse) {
    match &status.weather {
        Some(weather) => {
            lines.push(format!("weather_enabled: {}", weather.enabled));
            lines.push(format!("weather_state: {:?}", weather.state).to_lowercase());
            lines.push(format!("weather_active: {}", weather.active));
            lines.push(format!("weather_stale: {}", weather.stale));
            lines.push(format!(
                "weather_provider: {}",
                optional_text(weather.provider.as_deref())
            ));
            lines.push(format!(
                "weather_fetched_at_epoch_s: {}",
                optional_u64(weather.fetched_at_epoch_s)
            ));
            lines.push(format!(
                "weather_valid_at_epoch_s: {}",
                optional_u64(weather.valid_at_epoch_s)
            ));
            lines.push(format!(
                "weather_source_kind: {}",
                weather.source_kind.map_or_else(
                    || String::from("unavailable"),
                    |kind| format!("{kind:?}").to_lowercase(),
                )
            ));
            lines.push(format!(
                "weather_last_refresh_attempt_epoch_s: {}",
                optional_u64(weather.last_refresh_attempt_epoch_s)
            ));
            lines.push(format!(
                "weather_next_refresh_at_epoch_s: {}",
                optional_u64(weather.next_refresh_at_epoch_s)
            ));
            lines.push(format!(
                "weather_consecutive_failures: {}",
                weather.consecutive_failures
            ));
            lines.push(format!(
                "weather_last_error: {}",
                optional_text(weather.last_error.as_deref())
            ));
            lines.push(format!(
                "weather_cloud_cover_percent: {}",
                optional_u8(weather.cloud_cover_percent)
            ));
            lines.push(format!(
                "weather_cloud_dimming_percent: {}",
                weather.cloud_dimming_percent
            ));
        }
        None => lines.push(String::from("weather_enabled: false")),
    }
}

fn push_monitor_lines(lines: &mut Vec<String>, status: &StatusResponse) {
    if status.monitors.is_empty() {
        lines.push(String::from("monitors: none"));
    } else {
        lines.push(String::from("monitors:"));
        for monitor in &status.monitors {
            let hold = monitor
                .external_hold
                .as_ref()
                .map(format_external_hold)
                .map_or_else(String::new, |text| format!(" {text}"));
            let contested = if monitor.contested {
                match monitor.contested_until_epoch_s {
                    Some(until) => format!(" contested(until={until})"),
                    None => String::from(" contested"),
                }
            } else {
                String::new()
            };
            let manual_cap = monitor
                .manual_deferred_until_epoch_s
                .map_or_else(String::new, |until| format!(" manual_cap(until={until})"));
            let writes = monitor
                .writes
                .as_ref()
                .map_or_else(String::new, |writes| format!(" {}", format_writes(writes)));
            lines.push(format!(
                "  - logical_id={} backend={} enabled={} topology={} override_percent={} last_applied_percent={} last_applied_at_epoch_s={} backoff_until_epoch_s={}{hold}{contested}{manual_cap}{writes}",
                monitor.logical_id,
                backend_name(monitor.backend),
                monitor.enabled,
                optional_text(monitor.topology.as_deref()),
                optional_u8(monitor.override_percent),
                optional_u8(monitor.last_applied_percent),
                optional_u64(monitor.last_applied_at_epoch_s),
                optional_u64(monitor.backoff_until_epoch_s),
            ));
        }
    }
}

/// Compact write accounting token, e.g. `writes=17 (3/12 in 10m)`;
/// `deferred` is appended while automatic writes wait for the budget.
pub(crate) fn format_writes(writes: &WriteBudgetStatus) -> String {
    let window = if writes.window_seconds > 0 && writes.window_seconds.is_multiple_of(60) {
        format!("{}m", writes.window_seconds / 60)
    } else {
        format!("{}s", writes.window_seconds)
    };
    let deferred = if writes.deferred { " deferred" } else { "" };
    format!(
        "writes={} ({}/{} in {window}){deferred}",
        writes.writes_total, writes.window_used, writes.window_max
    )
}

pub(crate) fn format_external_hold(hold: &ExternalHoldStatus) -> String {
    let minutes = if hold.remaining_s < 60 {
        String::from("<1m")
    } else {
        format!("{}m", hold.remaining_s.saturating_add(59) / 60)
    };
    format!(
        "external hold · {minutes} left (observed {}%)",
        hold.observed_percent
    )
}

pub(crate) fn render_offline_status(socket: &ipc::ControlSocket) -> String {
    let local_config = config::load().ok();
    let config_path = local_config
        .as_ref()
        .map(|report| report.path.display().to_string())
        .or_else(|| {
            paths::config_file()
                .ok()
                .map(|path| path.display().to_string())
        })
        .unwrap_or_else(|| String::from("unavailable"));
    let tick_seconds = local_config.as_ref().map_or_else(
        || String::from("unavailable"),
        |report| report.config.daemon.tick_seconds.to_string(),
    );
    let dry_run = local_config.as_ref().map_or_else(
        || String::from("unavailable"),
        |report| report.config.daemon.dry_run.to_string(),
    );
    let configured_monitors = local_config.as_ref().map_or_else(
        || String::from("unavailable"),
        |report| report.config.monitors.len().to_string(),
    );

    [
        String::from("daemon_alive: false"),
        format!("socket_path: {}", socket.display_target()),
        format!("config_path: {config_path}"),
        format!("tick_seconds: {tick_seconds}"),
        format!("dry_run: {dry_run}"),
        String::from("suspended: unavailable"),
        String::from("manual_override_active: unavailable"),
        String::from("per_monitor_override_until_epoch_s: unavailable"),
        String::from("global_override_percent: unavailable"),
        String::from("global_override_until_epoch_s: unavailable"),
        format!("configured_monitors: {configured_monitors}"),
        String::from("stateful_monitors: unavailable"),
        String::from("weather_enabled: unavailable"),
        String::from("weather_active: unavailable"),
        String::from("weather_stale: unavailable"),
        String::from("weather_provider: unavailable"),
        String::from("weather_fetched_at_epoch_s: unavailable"),
        String::from("weather_valid_at_epoch_s: unavailable"),
        String::from("weather_source_kind: unavailable"),
        String::from("weather_last_refresh_attempt_epoch_s: unavailable"),
        String::from("weather_next_refresh_at_epoch_s: unavailable"),
        String::from("weather_consecutive_failures: unavailable"),
        String::from("weather_last_error: unavailable"),
        String::from("weather_cloud_cover_percent: unavailable"),
        String::from("weather_cloud_dimming_percent: unavailable"),
    ]
    .join("\n")
}

pub(crate) fn render_config_validation(
    report: config::ConfigReport,
    registry: &sunreactor::theme::Registry,
) -> anyhow::Result<String> {
    let source = match report.source {
        ConfigSource::Defaults => "defaults",
        ConfigSource::FilePresent => "file",
    };

    if let Err(error) = report.config.validate_theme_with(registry) {
        anyhow::bail!(
            "config validation failed for {}\n- {error}",
            report.path.display()
        );
    }

    let mut lines = vec![
        format!("config valid: {}", report.path.display()),
        format!("source: {source}"),
        format!("monitors: {}", report.config.monitors.len()),
    ];
    if !registry.diagnostics().is_empty() {
        lines.push(String::from(
            "theme warnings (skipped user theme files; not used by this config):",
        ));
        for diagnostic in registry.diagnostics() {
            lines.push(format!("- {diagnostic}"));
        }
    }

    if !report.warnings.is_empty() {
        lines.push(String::from("warnings:"));
        for warning in report.warnings {
            lines.push(format!("- {warning}"));
        }
    }

    Ok(lines.join("\n"))
}

pub(crate) fn backend_name(backend: sunreactor::backends::BackendKind) -> &'static str {
    match backend {
        sunreactor::backends::BackendKind::Backlight => "backlight",
        sunreactor::backends::BackendKind::Ddc => "ddc",
    }
}

pub(crate) fn optional_text(value: Option<&str>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| String::from("none"), str::to_owned)
}

pub(crate) fn optional_u8(value: Option<u8>) -> String {
    value.map_or_else(|| String::from("none"), |value| value.to_string())
}

pub(crate) fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("none"), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use sunreactor::{
        backends::BackendKind,
        ipc::{self, MonitorStatus, StatusResponse, WeatherStatus},
    };

    use super::{render_ipc_response, render_status, render_theme};

    #[test]
    fn renders_theme_details() {
        let registry = sunreactor::theme::Registry::builtin();
        let text = render_theme(registry.get("handheld").unwrap());
        assert!(text.contains("id: handheld"));
        assert!(text.contains("family: pixel"));
        assert!(text.contains("aliases: gameboy, dmg, dmg01"));
        assert!(text.contains("  bg: #141c12"));
        assert!(text.contains("  renderer: pixel"));
    }

    #[test]
    fn queued_run_once_response_does_not_print_fake_counters() {
        let rendered = render_ipc_response(ipc::ResponseEnvelope::run_once(
            ipc::RunOnceResponse {
                tick_duration_ms: 0,
                monitors_evaluated: 0,
                writes_attempted: 0,
                writes_skipped: 0,
                writes_succeeded: 0,
                writes_failed: 0,
                queued: true,
            },
            "daemon tick queued",
        ))
        .expect("queued run-once should render");
        assert!(rendered.contains("queued — apply runs in background"));
        assert!(!rendered.contains("writes_"));
        assert!(!rendered.contains("tick_duration_ms"));
    }

    #[test]
    fn render_status_includes_weather_health_details() {
        let status = StatusResponse {
            now_epoch_s: 0,
            sunrise_epoch_s: Some(0),
            sunset_epoch_s: Some(0),
            daemon_alive: true,
            config_path: String::from("/tmp/config.toml"),
            tick_seconds: 60,
            dry_run: false,
            suspended: false,
            desktop_idle_dimmed: false,
            suspend_until_epoch_s: None,
            manual_override_active: false,
            per_monitor_override_until_epoch_s: None,
            global_override_percent: None,
            global_override_until_epoch_s: None,
            configured_monitors: 1,
            stateful_monitors: 1,
            weather: Some(WeatherStatus {
                multiplier: Some(1.0),
                enabled: true,
                state: sunreactor::weather::WeatherState::Stale,
                active: false,
                stale: true,
                provider: Some(String::from("openweather")),
                fetched_at_epoch_s: Some(1_700_000_000),
                valid_at_epoch_s: Some(1_700_000_000),
                source_kind: None,
                last_refresh_attempt_epoch_s: Some(1_700_000_120),
                next_refresh_at_epoch_s: Some(1_700_000_180),
                consecutive_failures: 2,
                last_error: Some(String::from("network timeout")),
                cloud_cover_percent: Some(80),
                temperature: Some(10.0),
                condition: sunreactor::weather::WeatherCondition::Cloudy,
                condition_description: Some(String::from("overcast clouds")),
                day_phase: Some(sunreactor::weather::WeatherDayPhase::Day),
                forecast: vec![],
                ..Default::default()
            }),
            monitors: vec![MonitorStatus {
                logical_id: String::from("desk"),
                backend: BackendKind::Ddc,
                enabled: true,
                override_percent: None,
                last_applied_percent: Some(40),
                last_applied_at_epoch_s: Some(1_700_000_000),
                backoff_until_epoch_s: None,
                topology: None,
                min_pct: Some(15),
                max_pct: Some(60),
                external_hold: Some(sunreactor::ipc::ExternalHoldStatus {
                    remaining_s: 1_321,
                    observed_percent: 70,
                }),
                writes: Some(sunreactor::ipc::WriteBudgetStatus {
                    writes_total: 17,
                    window_used: 3,
                    window_max: 12,
                    window_seconds: 600,
                    deferred: false,
                }),
                contested: false,
                contested_until_epoch_s: None,
                manual_deferred_until_epoch_s: None,
            }],
            solar_elevation: Some(15.0),
            lunar_phase: None,
            theme: None,
            theme_warning: None,
            temperature_unit: None,
        };

        let rendered = render_status(&status);

        assert!(rendered.contains("weather_stale: true"));
        assert!(rendered.contains("weather_consecutive_failures: 2"));
        assert!(rendered.contains("weather_last_error: network timeout"));
        assert!(rendered.contains("external hold · 23m left (observed 70%)"));
        assert!(rendered.contains("writes=17 (3/12 in 10m)"));
    }

    #[test]
    fn writes_token_marks_deferred_budget() {
        let writes = sunreactor::ipc::WriteBudgetStatus {
            writes_total: 40,
            window_used: 12,
            window_max: 12,
            window_seconds: 600,
            deferred: true,
        };
        assert_eq!(
            super::format_writes(&writes),
            "writes=40 (12/12 in 10m) deferred"
        );
    }

    #[test]
    fn render_status_shows_contested_and_manual_cap_reasons() {
        let mut status = StatusResponse {
            now_epoch_s: 1_000,
            sunrise_epoch_s: None,
            sunset_epoch_s: None,
            daemon_alive: true,
            config_path: String::from("/tmp/config.toml"),
            tick_seconds: 60,
            dry_run: false,
            suspended: false,
            desktop_idle_dimmed: false,
            suspend_until_epoch_s: None,
            manual_override_active: false,
            per_monitor_override_until_epoch_s: None,
            global_override_percent: None,
            global_override_until_epoch_s: None,
            configured_monitors: 1,
            stateful_monitors: 1,
            weather: None,
            monitors: vec![MonitorStatus {
                logical_id: String::from("desk"),
                backend: BackendKind::Ddc,
                enabled: true,
                override_percent: None,
                last_applied_percent: Some(40),
                last_applied_at_epoch_s: Some(990),
                backoff_until_epoch_s: None,
                topology: Some(String::from("present")),
                min_pct: Some(15),
                max_pct: Some(60),
                external_hold: None,
                writes: None,
                contested: true,
                contested_until_epoch_s: Some(22_600),
                manual_deferred_until_epoch_s: Some(1_001),
            }],
            solar_elevation: None,
            lunar_phase: None,
            theme: None,
            theme_warning: None,
            temperature_unit: None,
        };
        let rendered = render_status(&status);
        assert!(rendered.contains("contested(until=22600)"), "{rendered}");
        assert!(rendered.contains("manual_cap(until=1001)"), "{rendered}");

        status.monitors[0].contested_until_epoch_s = None;
        status.monitors[0].manual_deferred_until_epoch_s = None;
        let rendered = render_status(&status);
        assert!(rendered.contains(" contested"), "{rendered}");
    }
}
