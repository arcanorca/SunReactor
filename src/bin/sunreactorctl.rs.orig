#![warn(clippy::pedantic)]
#![allow(
    clippy::unnecessary_wraps,
    clippy::struct_excessive_bools,
    clippy::missing_errors_doc
)]

use std::{env, process};

use sunreactor::{
    cli_help,
    config::{self, ConfigSource},
    discovery,
    ipc::{self, Request, RequestEnvelope, Response, StatusResponse},
    paths, version,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum OverrideTarget {
    Monitor(String),
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    Suspend {
        minutes: Option<u64>,
    },
    Resume,
    IdleDim,
    IdleWake,
    SetOverride {
        target: OverrideTarget,
        percent: u8,
        minutes: Option<u64>,
    },
    ClearOverride {
        monitor_id: Option<String>,
        global: bool,
    },
    ReloadConfig,
    Ping,
    RunOnce {
        force: bool,
    },
    Discover {
        json: bool,
    },
    ConfigInit,
    ConfigValidate,
    #[cfg(feature = "tui")]
    Tui,
}

fn main() -> anyhow::Result<()> {
    if let Err(error) = try_main() {
        eprintln!("sunreactorctl error:");
        for cause in error.chain() {
            eprintln!("  caused by: {cause}");
        }
        process::exit(1);
    }
    Ok(())
}

fn try_main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!("{}", cli_help());
        return Ok(());
    }

    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        println!("sunreactorctl {}", version());
        return Ok(());
    }

    let command = parse_command(&args)?;
    println!("{}", run_command(command)?);
    Ok(())
}

fn parse_command(args: &[String]) -> anyhow::Result<CliCommand> {
    match args.first().map(String::as_str) {
        #[cfg(feature = "tui")]
        None => Ok(CliCommand::Tui),
        #[cfg(not(feature = "tui"))]
        None => Ok(CliCommand::Status),
        Some("status") => Ok(CliCommand::Status),
        Some("suspend") => parse_suspend_command(&args[1..]),
        Some("resume") => Ok(CliCommand::Resume),
        Some("idle-dim") => Ok(CliCommand::IdleDim),
        Some("idle-wake") => Ok(CliCommand::IdleWake),
        Some("set") => parse_set_command(&args[1..]),
        Some("clear-override") => parse_clear_override_command(&args[1..]),
        Some("reload-config" | "reload") => Ok(CliCommand::ReloadConfig),
        Some("ping") => Ok(CliCommand::Ping),
        Some("run-once" | "apply-once") => parse_run_once_command(&args[1..]),
        Some("discover") => parse_discover_command(&args[1..]),
        Some("config") => match args.get(1).map(String::as_str) {
            Some("init") => Ok(CliCommand::ConfigInit),
            Some("validate") => Ok(CliCommand::ConfigValidate),
            _ => anyhow::bail!("usage: sunreactorctl config <init|validate>"),
        },
        #[cfg(feature = "tui")]
        Some("tui") => Ok(CliCommand::Tui),
        Some(other) => anyhow::bail!("unknown command: {other}"),
    }
}

fn parse_suspend_command(args: &[String]) -> anyhow::Result<CliCommand> {
    if args.is_empty() {
        return Ok(CliCommand::Suspend { minutes: None });
    }
    if args.len() != 2 {
        anyhow::bail!("usage: sunreactorctl suspend [--minutes <N>]");
    }

    match args[0].as_str() {
        "--minutes" | "-m" => {
            let minutes = args[1]
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("suspend minutes must be a positive integer"))?;
            if minutes == 0 {
                anyhow::bail!("suspend minutes must be greater than zero");
            }
            Ok(CliCommand::Suspend {
                minutes: Some(minutes),
            })
        }
        _ => anyhow::bail!("usage: sunreactorctl suspend [--minutes <N>]"),
    }
}

fn parse_run_once_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let mut force = false;
    for arg in args {
        match arg.as_str() {
            "--force" | "-f" => force = true,
            _ => anyhow::bail!("usage: sunreactorctl run-once [--force]"),
        }
    }
    Ok(CliCommand::RunOnce { force })
}

fn parse_discover_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let mut json = false;

    for arg in args {
        match arg.as_str() {
            "--json" | "-j" => json = true,
            _ => anyhow::bail!("usage: sunreactorctl discover [--json]"),
        }
    }

    Ok(CliCommand::Discover { json })
}

fn parse_set_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let usage = "usage: sunreactorctl set <monitor-id> <pct> [--minutes <N>] | sunreactorctl set --global <pct> [--minutes <N>]";
    if args.is_empty() {
        anyhow::bail!("{usage}");
    }

    let (target, percent_index) = if args[0] == "--global" {
        (OverrideTarget::Global, 1usize)
    } else {
        if args[0].trim().is_empty() {
            anyhow::bail!("monitor-id must not be empty");
        }
        (OverrideTarget::Monitor(args[0].clone()), 1usize)
    };

    if args.len() <= percent_index {
        anyhow::bail!("{usage}");
    }

    let percent = parse_percent(&args[percent_index])?;
    let minutes = parse_optional_minutes(&args[percent_index + 1..], usage)?;

    Ok(CliCommand::SetOverride {
        target,
        percent,
        minutes,
    })
}

fn parse_clear_override_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let usage = "usage: sunreactorctl clear-override [--monitor-id <ID> | --global]";
    match args {
        [] => Ok(CliCommand::ClearOverride {
            monitor_id: None,
            global: false,
        }),
        [flag, value] if flag == "--monitor-id" => {
            if value.trim().is_empty() {
                anyhow::bail!("monitor-id must not be empty")
            }
            Ok(CliCommand::ClearOverride {
                monitor_id: Some(value.clone()),
                global: false,
            })
        }
        [flag] if flag == "--global" => Ok(CliCommand::ClearOverride {
            monitor_id: None,
            global: true,
        }),
        _ => anyhow::bail!("{usage}"),
    }
}

fn parse_percent(raw: &str) -> anyhow::Result<u8> {
    let percent = raw.parse::<u8>().map_err(|_| {
        anyhow::anyhow!("brightness percent must be an integer in the range 0..=100")
    })?;
    if percent > 100 {
        anyhow::bail!("brightness percent must be an integer in the range 0..=100");
    }
    Ok(percent)
}

fn parse_optional_minutes(args: &[String], usage: &str) -> anyhow::Result<Option<u64>> {
    if args.is_empty() {
        return Ok(None);
    }
    if args.len() != 2 {
        anyhow::bail!("{usage}");
    }

    match args[0].as_str() {
        "--minutes" | "-m" => {
            let minutes = args[1]
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("override minutes must be a positive integer"))?;
            if minutes == 0 {
                anyhow::bail!("override minutes must be greater than zero");
            }
            Ok(Some(minutes))
        }
        _ => anyhow::bail!("{usage}"),
    }
}

fn run_command(command: CliCommand) -> anyhow::Result<String> {
    match command {
        CliCommand::Status => run_status_command(),
        CliCommand::Suspend { minutes } => run_ipc_command(Request::Suspend { minutes }),
        CliCommand::Resume => run_ipc_command(Request::Resume),
        CliCommand::IdleDim => run_ipc_command(Request::IdleDim),
        CliCommand::IdleWake => run_ipc_command(Request::IdleWake),
        CliCommand::SetOverride {
            target,
            percent,
            minutes,
        } => {
            let monitor_id = match target {
                OverrideTarget::Monitor(monitor_id) => Some(monitor_id),
                OverrideTarget::Global => None,
            };
            run_ipc_command(Request::SetOverride {
                monitor_id,
                percent,
                minutes,
            })
        }
        CliCommand::ClearOverride { monitor_id, global } => {
            run_ipc_command(Request::ClearOverride { monitor_id, global })
        }
        CliCommand::ReloadConfig => run_ipc_command(Request::ReloadConfig),
        CliCommand::Ping => run_ipc_command(Request::Ping),
        CliCommand::RunOnce { force } => run_ipc_command(Request::RunOnce { force }),
        CliCommand::Discover { json } => {
            let report = discovery::discover();
            if json {
                Ok(report.render_json())
            } else {
                Ok(report.render_human())
            }
        }
        CliCommand::ConfigInit => {
            let path = config::write_default()?;
            Ok(format!("wrote default config to {}", path.display()))
        }
        CliCommand::ConfigValidate => validate_config(),
        #[cfg(feature = "tui")]
        CliCommand::Tui => sunreactor::tui::run().map_err(|e| anyhow::anyhow!("{e}")),
    }
}

fn run_status_command() -> anyhow::Result<String> {
    let socket = ipc::ControlSocket::from_runtime()?;

    match socket.send_request(&RequestEnvelope::new(Request::Status)) {
        Ok(response) => render_ipc_response(response),
        Err(ipc::IpcError::Unavailable { .. }) => Ok(render_offline_status(&socket)),
        Err(error) => Err(error.into()),
    }
}

fn run_ipc_command(request: Request) -> anyhow::Result<String> {
    let socket = ipc::ControlSocket::from_runtime()?;
    let response = socket.send_request(&RequestEnvelope::new(request))?;
    render_ipc_response(response)
}

fn render_ipc_response(response: ipc::ResponseEnvelope) -> anyhow::Result<String> {
    match response.response {
        Response::Pong { message } | Response::Ack { message } => Ok(message),
        Response::Status { status } => Ok(render_status(&status)),
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
        Response::Error { message, .. } => anyhow::bail!(message),
    }
}

fn render_status(status: &StatusResponse) -> String {
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

    match &status.weather {
        Some(weather) => {
            lines.push(format!("weather_enabled: {}", weather.enabled));
            lines.push(format!("weather_active: {}", weather.active));
            lines.push(format!("weather_stale: {}", weather.stale));
            lines.push(format!(
                "weather_provider: {}",
                optional_text(weather.provider.as_deref())
            ));
            lines.push(format!(
                "weather_observed_at_epoch_s: {}",
                optional_u64(weather.observed_at_epoch_s)
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
        }
        None => lines.push(String::from("weather_enabled: false")),
    }

    if status.monitors.is_empty() {
        lines.push(String::from("monitors: none"));
    } else {
        lines.push(String::from("monitors:"));
        for monitor in &status.monitors {
            lines.push(format!(
                "  - logical_id={} backend={} enabled={} override_percent={} last_applied_percent={} last_applied_at_epoch_s={} backoff_until_epoch_s={}",
                monitor.logical_id,
                backend_name(monitor.backend),
                monitor.enabled,
                optional_u8(monitor.override_percent),
                optional_u8(monitor.last_applied_percent),
                optional_u64(monitor.last_applied_at_epoch_s),
                optional_u64(monitor.backoff_until_epoch_s),
            ));
        }
    }

    lines.join("\n")
}

fn render_offline_status(socket: &ipc::ControlSocket) -> String {
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
        format!("socket_path: {}", socket.path.display()),
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
        String::from("weather_observed_at_epoch_s: unavailable"),
        String::from("weather_last_refresh_attempt_epoch_s: unavailable"),
        String::from("weather_next_refresh_at_epoch_s: unavailable"),
        String::from("weather_consecutive_failures: unavailable"),
        String::from("weather_last_error: unavailable"),
        String::from("weather_cloud_cover_percent: unavailable"),
    ]
    .join("\n")
}

fn validate_config() -> anyhow::Result<String> {
    let path = paths::config_file()?;
    if !path.exists() {
        anyhow::bail!(
            "config file not found at {}. Run `sunreactorctl config init` first.",
            path.display()
        );
    }

    let report = config::load()?;
    let source = match report.source {
        ConfigSource::Defaults => "defaults",
        ConfigSource::FilePresent => "file",
    };

    let mut lines = vec![
        format!("config valid: {}", report.path.display()),
        format!("source: {source}"),
        format!("monitors: {}", report.config.monitors.len()),
    ];

    if !report.warnings.is_empty() {
        lines.push(String::from("warnings:"));
        for warning in report.warnings {
            lines.push(format!("- {warning}"));
        }
    }

    Ok(lines.join("\n"))
}

fn backend_name(backend: sunreactor::backends::BackendKind) -> &'static str {
    match backend {
        sunreactor::backends::BackendKind::Backlight => "backlight",
        sunreactor::backends::BackendKind::Ddc => "ddc",
    }
}

fn optional_text(value: Option<&str>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| String::from("none"), str::to_owned)
}

fn optional_u8(value: Option<u8>) -> String {
    value.map_or_else(|| String::from("none"), |value| value.to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("none"), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use sunreactor::{
        backends::BackendKind,
        ipc::{MonitorStatus, StatusResponse, WeatherStatus},
    };

    use super::{parse_command, render_status, CliCommand, OverrideTarget};

    #[test]
    fn parses_suspend_command() {
        let args = vec![
            String::from("suspend"),
            String::from("--minutes"),
            String::from("15"),
        ];

        let command = parse_command(&args).expect("suspend command should parse");

        match command {
            CliCommand::Suspend { minutes } => assert_eq!(minutes, Some(15)),
            _ => panic!("expected suspend command"),
        }
    }

    #[test]
    fn parses_indefinite_suspend_command() {
        let command =
            parse_command(&[String::from("suspend")]).expect("indefinite suspend should parse");

        match command {
            CliCommand::Suspend { minutes } => assert_eq!(minutes, None),
            _ => panic!("expected suspend command"),
        }
    }

    #[test]
    fn parses_reload_and_run_once_aliases() {
        let reload =
            parse_command(&[String::from("reload-config")]).expect("reload-config should parse");
        assert!(matches!(reload, CliCommand::ReloadConfig));

        let run_once = parse_command(&[String::from("run-once")]).expect("run-once should parse");
        assert!(matches!(run_once, CliCommand::RunOnce { force: false }));
    }

    #[test]
    fn parses_set_override_commands() {
        let per_monitor = parse_command(&[
            String::from("set"),
            String::from("internal"),
            String::from("42"),
            String::from("--minutes"),
            String::from("15"),
        ])
        .expect("per-monitor set should parse");
        assert_eq!(
            per_monitor,
            CliCommand::SetOverride {
                target: OverrideTarget::Monitor(String::from("internal")),
                percent: 42,
                minutes: Some(15),
            }
        );

        let global = parse_command(&[
            String::from("set"),
            String::from("--global"),
            String::from("30"),
        ])
        .expect("global set should parse");
        assert_eq!(
            global,
            CliCommand::SetOverride {
                target: OverrideTarget::Global,
                percent: 30,
                minutes: None,
            }
        );
    }

    #[test]
    fn parses_clear_override_commands() {
        let clear_all =
            parse_command(&[String::from("clear-override")]).expect("clear-override should parse");
        assert_eq!(
            clear_all,
            CliCommand::ClearOverride {
                monitor_id: None,
                global: false,
            }
        );

        let clear_monitor = parse_command(&[
            String::from("clear-override"),
            String::from("--monitor-id"),
            String::from("internal"),
        ])
        .expect("clear-override --monitor-id should parse");
        assert_eq!(
            clear_monitor,
            CliCommand::ClearOverride {
                monitor_id: Some(String::from("internal")),
                global: false,
            }
        );

        let clear_global =
            parse_command(&[String::from("clear-override"), String::from("--global")])
                .expect("clear-override --global should parse");
        assert_eq!(
            clear_global,
            CliCommand::ClearOverride {
                monitor_id: None,
                global: true,
            }
        );
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
                active: false,
                stale: true,
                provider: Some(String::from("openweather")),
                observed_at_epoch_s: Some(1_700_000_000),
                last_refresh_attempt_epoch_s: Some(1_700_000_120),
                next_refresh_at_epoch_s: Some(1_700_000_180),
                consecutive_failures: 2,
                last_error: Some(String::from("network timeout")),
                cloud_cover_percent: Some(80),
                temperature: Some(10.0),
                forecast: vec![],
            }),
            monitors: vec![MonitorStatus {
                logical_id: String::from("desk"),
                backend: BackendKind::Ddc,
                enabled: true,
                override_percent: None,
                last_applied_percent: Some(40),
                last_applied_at_epoch_s: Some(1_700_000_000),
                backoff_until_epoch_s: None,
            }],
            solar_elevation: Some(15.0),
            lunar_phase: None,
        };

        let rendered = render_status(&status);

        assert!(rendered.contains("weather_stale: true"));
        assert!(rendered.contains("weather_consecutive_failures: 2"));
        assert!(rendered.contains("weather_last_error: network timeout"));
    }
}
