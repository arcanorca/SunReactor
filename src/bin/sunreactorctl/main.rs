#![allow(
    clippy::unnecessary_wraps,
    clippy::struct_excessive_bools,
    clippy::missing_errors_doc
)]

use std::{env, process};

use sunreactor::{
    cli_help, config, discovery,
    ipc::{self, Request, RequestEnvelope, Response, ResponseEnvelope},
    paths, version,
};

mod parse;
mod render;

use parse::{parse_command, CliCommand, HoldCommand, OverrideTarget, ThemeCommand, WeatherCommand};
use render::{
    render_config_validation, render_ipc_response, render_offline_status, render_theme,
    render_theme_list,
};

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
        CliCommand::Discover { json, apply } => run_discover_command(json, apply),
        CliCommand::TestBrightness { monitor_id, json } => {
            run_test_brightness_command(&monitor_id, json)
        }
        CliCommand::ConfigInit => {
            let path = config::write_default()?;
            Ok(format!("wrote default config to {}", path.display()))
        }
        CliCommand::ConfigValidate => validate_config(),
        CliCommand::Theme(command) => run_theme_command(command),
        CliCommand::Weather(WeatherCommand::CloudDimming { percent }) => {
            run_cloud_dimming_command(percent)
        }
        CliCommand::Hold(HoldCommand::Clear { monitor }) => {
            run_ipc_command(Request::ClearExternalHold { monitor })
        }
        #[cfg(target_os = "linux")]
        CliCommand::Doctor { json } => run_doctor_command(json),
        #[cfg(feature = "tui")]
        CliCommand::Tui => sunreactor::tui::run().map_err(|e| anyhow::anyhow!("{e}")),
    }
}

fn run_discover_command(json: bool, apply: bool) -> anyhow::Result<String> {
    let report = discovery::discover();
    if apply {
        let path = paths::config_file()?;
        let result = config::apply_discovered_transaction(&path, &report, || {
            verify_reloaded_config().map_err(|error| error.to_string())
        })?;
        if json {
            return Ok(serde_json::to_string_pretty(&result)?);
        }
        if result.unchanged {
            return Ok(format!(
                "no config changes: all viable monitors are already present in {}",
                result.path.display()
            ));
        }
        let backup = result.backup_path.as_ref().map_or_else(
            || String::from("none (new config)"),
            |path| path.display().to_string(),
        );
        return Ok(format!(
            "added monitor(s): {}\nconfig: {}\nbackup: {}\ndaemon reload and IPC verification: ok",
            result.added_monitor_ids.join(", "),
            result.path.display(),
            backup
        ));
    }
    if json {
        Ok(report.render_json())
    } else {
        Ok(report.render_human())
    }
}

fn run_test_brightness_command(monitor_id: &str, json: bool) -> anyhow::Result<String> {
    #[cfg(target_os = "windows")]
    {
        if std::env::var("SUNREACTOR_ALLOW_HARDWARE_TESTS").map_or(true, |v| v != "1") {
            anyhow::bail!(
                "hardware mutation requires explicit confirmation: set SUNREACTOR_ALLOW_HARDWARE_TESTS=1 in your environment"
            );
        }
        let api = sunreactor::platform::windows::brightness::Win32MonitorConfigApi;
        let report = sunreactor::platform::windows::brightness::test_physical_monitor_brightness(
            &api, monitor_id,
        )
        .map_err(|e| anyhow::anyhow!("hardware qualification failed: {e}"))?;
        if json {
            Ok(serde_json::to_string_pretty(&report)?)
        } else {
            let mut out = format!(
                "Physical Monitor Qualification Report\nCanonical ID:     {}\nDisplay Label:    {}\nManufacturer:     {}\nProduct Code:     {}\nOutput Tech:      {}\nPhysical Handles: {}\n\nNative Range:     {}..{}\nOriginal Native:  {} ({}%)\nTest Target:      {} ({}%)\n\nLatencies:\n  Pre-write read: {:?}\n  Write latency:  {:?}\n  Readback read:  {:?}\n  Restore write:  {:?}\n  Restore read:   {:?}\n\nReadback Observed:   {}\nRestored Observed:   {}\nQualified:           {}\n",
                report.canonical_id,
                report.display_label.as_deref().unwrap_or("none"),
                report.manufacturer.as_deref().unwrap_or("unknown"),
                report
                    .product_code
                    .map_or(String::from("unknown"), |c| format!("0x{c:04x}")),
                report.output_technology,
                report.physical_handles_count,
                report.native_min,
                report.native_max,
                report.native_original,
                report.original_percent,
                report.test_target_native,
                report.test_target_percent,
                report.pre_write_read_latency,
                report.write_latency,
                report.readback_latency,
                report.restoration_write_latency,
                report.restoration_readback_latency,
                report.readback_observed_native,
                report.restored_observed_native,
                if report.qualified {
                    "YES (PASSED)"
                } else {
                    "NO (FAILED)"
                }
            );
            if !report.notes.is_empty() {
                use std::fmt::Write;
                out.push_str("\nNotes:\n");
                for note in &report.notes {
                    let _ = writeln!(out, "  - {note}");
                }
            }
            Ok(out)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (monitor_id, json);
        anyhow::bail!("test-brightness is only available on Windows for native external monitors");
    }
}

#[cfg(target_os = "linux")]
fn run_doctor_command(json: bool) -> anyhow::Result<String> {
    let report = sunreactor::doctor::run_diagnostics();
    let rendered = if json {
        serde_json::to_string_pretty(&report)?
    } else {
        let mut lines = Vec::new();
        lines.push(
            if report.overall_healthy {
                "Overall Health: OK"
            } else {
                "Overall Health: ISSUES FOUND"
            }
            .to_string(),
        );
        lines.push(String::new());
        for check in &report.checks {
            let status = match check.status {
                sunreactor::doctor::CheckStatus::Pass => "[PASS]",
                sunreactor::doctor::CheckStatus::Warn => "[WARN]",
                sunreactor::doctor::CheckStatus::Error => "[FAIL]",
            };
            lines.push(format!("{} {}: {}", status, check.code, check.message));
        }
        lines.join("\n")
    };
    if report.overall_healthy {
        Ok(rendered)
    } else {
        println!("{rendered}");
        anyhow::bail!("doctor found {} blocking error(s)", report.blocking_errors)
    }
}

fn run_cloud_dimming_command(percent: Option<u8>) -> anyhow::Result<String> {
    if let Some(percent) = percent {
        return run_ipc_command(Request::SetCloudDimming { percent });
    }

    let current = match ipc::ControlSocket::from_runtime() {
        Ok(socket) => match socket.send_request(&RequestEnvelope::new(Request::Status)) {
            Ok(ResponseEnvelope {
                response: Response::Status { status },
                ..
            }) => status.weather.map(|weather| weather.cloud_dimming_percent),
            Ok(_) | Err(ipc::IpcError::Unavailable { .. }) => None,
            Err(error) => return Err(error.into()),
        },
        Err(_) => None,
    }
    .or_else(|| {
        config::load()
            .ok()
            .map(|report| report.config.weather.cloud_dimming_percent())
    })
    .ok_or_else(|| anyhow::anyhow!("cloud dimming setting is unavailable"))?;
    Ok(format!("cloud_dimming_percent: {current}"))
}

fn run_theme_command(command: ThemeCommand) -> anyhow::Result<String> {
    use sunreactor::theme;

    // The CLI is a fresh process: this reads the user theme directory once.
    let registry = theme::registry();
    match command {
        ThemeCommand::List { json } => {
            if json {
                let themes: Vec<ipc::ThemeSummary> = registry
                    .themes()
                    .iter()
                    .map(|theme| ipc::ThemeSummary::from_theme(theme))
                    .collect();
                return Ok(serde_json::to_string_pretty(&themes)?);
            }
            let current = config::load()
                .ok()
                .map(|report| report.config.tui.theme.as_str().to_owned());
            Ok(render_theme_list(&registry, current.as_deref()))
        }
        ThemeCommand::Show { id, json } => {
            let theme = registry.resolve(&id)?;
            if json {
                Ok(serde_json::to_string_pretty(
                    &ipc::ThemeStatus::from_theme(theme),
                )?)
            } else {
                Ok(render_theme(theme))
            }
        }
        ThemeCommand::Export { id } => {
            let theme = registry.resolve(&id)?;
            Ok(format!(
                "# Exported from sunreactor theme `{id}` ({source}).\n\
                 # Save as {dir}<new_id>.toml, set id = \"<new_id>\", drop `aliases`, then edit.\n\
                 # Tip: `extends = \"{id}\"` lets a file keep only the keys it changes.\n{body}",
                id = theme.id,
                source = theme.source.label(),
                dir = paths::THEMES_DIR,
                body = theme.to_toml(),
            ))
        }
        ThemeCommand::Validate { file } => validate_themes(file.as_deref()),
        ThemeCommand::Set { id } => set_theme(&registry, &id),
    }
}

fn validate_themes(file: Option<&std::path::Path>) -> anyhow::Result<String> {
    use sunreactor::theme;

    let dir = paths::themes_dir()?;
    if let Some(file) = file {
        let theme = theme::validate_file(file, Some(&dir))?;
        return Ok(format!(
            "theme valid: {} ({}, family {})",
            theme.id, theme.name, theme.family
        ));
    }

    let registry = theme::reload_registry();
    let user: Vec<_> = registry
        .themes()
        .iter()
        .filter(|theme| matches!(theme.source, theme::ThemeSource::User(_)))
        .collect();
    let mut lines = vec![format!("user theme dir: {}", dir.display())];
    for theme in &user {
        lines.push(format!("ok: {} ({})", theme.id, theme.name));
    }
    if registry.diagnostics().is_empty() {
        lines.push(format!(
            "{} user theme(s) valid, {} built-in",
            user.len(),
            registry.themes().len() - user.len()
        ));
        return Ok(lines.join("\n"));
    }
    for diagnostic in registry.diagnostics() {
        lines.push(diagnostic.to_string());
    }
    anyhow::bail!(lines.join("\n"))
}

fn set_theme(registry: &sunreactor::theme::Registry, name: &str) -> anyhow::Result<String> {
    let path = paths::config_file()?;
    set_theme_at(registry, name, &path, || {
        ipc::ControlSocket::from_runtime()?
            .send_request(&RequestEnvelope::new(Request::ReloadConfig))
    })
}

/// Same write path as the TUI theme modal: load, edit, validate, save the
/// whole config, then ask a running daemon to reload.
fn set_theme_at(
    registry: &sunreactor::theme::Registry,
    name: &str,
    path: &std::path::Path,
    reload: impl FnOnce() -> Result<ipc::ResponseEnvelope, ipc::IpcError>,
) -> anyhow::Result<String> {
    let theme = registry.resolve(name)?;
    let mut config = if path.exists() {
        config::parse_text(&std::fs::read_to_string(path)?, path)?
    } else {
        config::Config::default()
    };
    let previous = config.tui.theme.clone();
    config.tui.theme = config::ThemeId::new(theme.id.clone());
    let path = config::save_to_path(&config, path)?;

    let mut lines = vec![
        format!("theme: {previous} -> {} ({})", theme.id, theme.name),
        format!("config: {}", path.display()),
    ];
    match reload() {
        Ok(response) => match render_ipc_response(response) {
            Ok(_) => lines.push(String::from("daemon: reloaded")),
            Err(error) => lines.push(format!("daemon: reload failed: {error}")),
        },
        Err(ipc::IpcError::Unavailable { .. }) => {
            lines.push(String::from("daemon: not running (applies on next start)"));
        }
        Err(error) => lines.push(format!("daemon: reload failed: {error}")),
    }
    Ok(lines.join("\n"))
}

fn verify_reloaded_config() -> anyhow::Result<()> {
    let socket = ipc::ControlSocket::from_runtime()?;
    render_ipc_response(socket.send_request(&RequestEnvelope::new(Request::ReloadConfig))?)?;
    render_ipc_response(socket.send_request(&RequestEnvelope::new(Request::Ping))?)?;
    Ok(())
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

fn validate_config() -> anyhow::Result<String> {
    let path = paths::config_file()?;
    if !path.exists() {
        anyhow::bail!(
            "config file not found at {}. Run `sunreactorctl config init` first.",
            path.display()
        );
    }

    render_config_validation(config::load()?, &sunreactor::theme::registry())
}

/// `config validate` output. The daemon tolerates theme problems (fallback +
/// `status.theme_warning`); this command is where they are errors.
#[cfg(test)]
mod tests {
    use super::set_theme_at;

    #[test]
    fn theme_set_writes_canonical_id_and_tolerates_offline_daemon() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[tui]\nfps = 4\ntheme = \"nord\"\n").unwrap();
        let registry = sunreactor::theme::Registry::builtin();
        let out = set_theme_at(&registry, "workbench", &path, || {
            Err(sunreactor::ipc::IpcError::Unavailable {
                target: String::from("test"),
                message: String::from("offline"),
            })
        })
        .unwrap();
        assert!(out.contains("theme: nord -> amiga (Amiga)"), "{out}");
        assert!(out.contains("daemon: not running"), "{out}");
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("theme = \"amiga\""), "{saved}");
        assert!(saved.contains("fps = 4"), "{saved}");

        let out = set_theme_at(&registry, "dracula", &path, || {
            Ok(sunreactor::ipc::ResponseEnvelope::ack("reloaded"))
        })
        .unwrap();
        assert!(out.contains("daemon: reloaded"), "{out}");

        let error = set_theme_at(&registry, "dracla", &path, || unreachable!())
            .unwrap_err()
            .to_string();
        assert!(error.contains("did you mean `dracula`"), "{error}");
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("theme = \"dracula\""));
    }

    #[test]
    fn config_validate_rejects_unusable_theme_but_only_warns_for_unrelated_files() {
        use sunreactor::config::{Config, ConfigReport, ConfigSource, ThemeId};
        use sunreactor::theme::{Registry, UserThemeSource};

        let source = |stem: &str, text: &str| UserThemeSource {
            stem: stem.to_owned(),
            path: std::path::PathBuf::from(format!("/virtual/{stem}.toml")),
            text: Ok(text.to_owned()),
        };
        let registry = Registry::with_user_sources(vec![source("broken", "id = \"broken\"\n")]);
        let report = |theme: &str| ConfigReport {
            path: std::path::PathBuf::from("/virtual/config.toml"),
            source: ConfigSource::FilePresent,
            config: Config {
                tui: sunreactor::config::TuiConfig {
                    theme: ThemeId::new(theme),
                    ..Default::default()
                },
                ..Config::default()
            },
            warnings: Vec::new(),
        };

        // Err => main() exits non-zero.
        let error = super::render_config_validation(report("broken"), &registry)
            .unwrap_err()
            .to_string();
        assert!(error.contains("tui.theme"), "{error}");
        assert!(error.contains("/virtual/broken.toml"), "{error}");
        assert!(error.contains("name: required key is missing"), "{error}");

        let error = super::render_config_validation(report("draculla"), &registry)
            .unwrap_err()
            .to_string();
        assert!(error.contains("did you mean `dracula`"), "{error}");

        let ok = super::render_config_validation(report("dracula"), &registry).unwrap();
        assert!(ok.contains("config valid"), "{ok}");
        assert!(ok.contains("theme warnings"), "{ok}");
        assert!(ok.contains("/virtual/broken.toml"), "{ok}");
    }
}
