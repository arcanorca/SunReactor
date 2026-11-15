use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverrideTarget {
    Monitor(String),
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ThemeCommand {
    List { json: bool },
    Show { id: String, json: bool },
    Set { id: String },
    Validate { file: Option<PathBuf> },
    Export { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WeatherCommand {
    CloudDimming { percent: Option<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HoldCommand {
    Clear { monitor: Option<String> },
}

pub(crate) const THEME_USAGE: &str = "usage: sunreactorctl theme <list [--json] | show <id> [--json] | set <id> | validate [file] | export <id>>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliCommand {
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
        apply: bool,
    },
    TestBrightness {
        monitor_id: String,
        json: bool,
    },
    ConfigInit,
    ConfigValidate,
    Theme(ThemeCommand),
    Weather(WeatherCommand),
    Hold(HoldCommand),
    #[cfg(target_os = "linux")]
    Doctor {
        json: bool,
    },
    #[cfg(feature = "tui")]
    Tui,
}

pub(crate) fn parse_command(args: &[String]) -> anyhow::Result<CliCommand> {
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
        Some("test-brightness" | "test-hardware") => parse_test_brightness_command(&args[1..]),
        Some("config") => match args.get(1).map(String::as_str) {
            Some("init") => Ok(CliCommand::ConfigInit),
            Some("validate") => Ok(CliCommand::ConfigValidate),
            _ => anyhow::bail!("usage: sunreactorctl config <init|validate>"),
        },
        Some("theme" | "themes") => parse_theme_command(&args[1..]),
        Some("weather") => parse_weather_command(&args[1..]),
        Some("hold") => parse_hold_command(&args[1..]),
        #[cfg(target_os = "linux")]
        Some("doctor") => parse_doctor_command(&args[1..]),
        #[cfg(feature = "tui")]
        Some("tui") => Ok(CliCommand::Tui),
        Some(other) => anyhow::bail!("unknown command: {other}"),
    }
}

pub(crate) fn parse_weather_command(args: &[String]) -> anyhow::Result<CliCommand> {
    if args.first().map(String::as_str) != Some("cloud-dimming") || args.len() > 2 {
        anyhow::bail!("usage: sunreactorctl weather cloud-dimming [<percent>]");
    }
    let percent = args
        .get(1)
        .map(|raw| {
            raw.parse::<u8>().map_err(|_| {
                anyhow::anyhow!("cloud dimming percent must be an integer in the range 0..=50")
            })
        })
        .transpose()?;
    if percent.is_some_and(|value| value > 50) {
        anyhow::bail!("cloud dimming percent must be an integer in the range 0..=50");
    }
    Ok(CliCommand::Weather(WeatherCommand::CloudDimming {
        percent,
    }))
}

pub(crate) fn parse_hold_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let usage = "usage: sunreactorctl hold clear [MONITOR]";
    match args {
        [command] if command == "clear" => {
            Ok(CliCommand::Hold(HoldCommand::Clear { monitor: None }))
        }
        [command, monitor] if command == "clear" => {
            if monitor.trim().is_empty() {
                anyhow::bail!("monitor-id must not be empty");
            }
            Ok(CliCommand::Hold(HoldCommand::Clear {
                monitor: Some(monitor.clone()),
            }))
        }
        _ => anyhow::bail!("{usage}"),
    }
}

pub(crate) fn parse_theme_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let json = args
        .iter()
        .skip(1)
        .any(|arg| arg == "--json" || arg == "-j");
    let positional: Vec<&str> = args
        .iter()
        .skip(1)
        .map(String::as_str)
        .filter(|arg| *arg != "--json" && *arg != "-j")
        .collect();
    let command = match (args.first().map(String::as_str), positional.as_slice()) {
        (None, []) => ThemeCommand::List { json: false },
        (Some("list" | "ls"), []) => ThemeCommand::List { json },
        (Some("show"), [id]) => ThemeCommand::Show {
            id: (*id).to_owned(),
            json,
        },
        (Some("set"), [id]) if !json => ThemeCommand::Set {
            id: (*id).to_owned(),
        },
        (Some("validate"), []) if !json => ThemeCommand::Validate { file: None },
        (Some("validate"), [file]) if !json => ThemeCommand::Validate {
            file: Some(PathBuf::from(file)),
        },
        (Some("export"), [id]) if !json => ThemeCommand::Export {
            id: (*id).to_owned(),
        },
        _ => anyhow::bail!("{THEME_USAGE}"),
    };
    Ok(CliCommand::Theme(command))
}

pub(crate) fn parse_suspend_command(args: &[String]) -> anyhow::Result<CliCommand> {
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

pub(crate) fn parse_run_once_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let mut force = false;
    for arg in args {
        match arg.as_str() {
            "--force" | "-f" => force = true,
            _ => anyhow::bail!("usage: sunreactorctl run-once [--force]"),
        }
    }
    Ok(CliCommand::RunOnce { force })
}

pub(crate) fn parse_discover_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let mut json = false;
    let mut apply = false;

    for arg in args {
        match arg.as_str() {
            "--json" | "-j" => json = true,
            "--apply" => apply = true,
            _ => anyhow::bail!("usage: sunreactorctl discover [--json] [--apply]"),
        }
    }

    Ok(CliCommand::Discover { json, apply })
}

#[cfg(target_os = "linux")]
pub(crate) fn parse_doctor_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" | "-j" => json = true,
            _ => anyhow::bail!("usage: sunreactorctl doctor [--json]"),
        }
    }
    Ok(CliCommand::Doctor { json })
}

pub(crate) fn parse_test_brightness_command(args: &[String]) -> anyhow::Result<CliCommand> {
    let mut monitor_id = None;
    let mut json = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--monitor-id" | "-m" => {
                monitor_id = iter.next().cloned();
            }
            "--json" | "-j" => {
                json = true;
            }
            other => {
                if !other.starts_with('-') && monitor_id.is_none() {
                    monitor_id = Some(other.to_string());
                } else {
                    anyhow::bail!(
                        "usage: sunreactorctl test-brightness --monitor-id <CANONICAL_ID> [--json]"
                    );
                }
            }
        }
    }
    let monitor_id = monitor_id.ok_or_else(|| {
        anyhow::anyhow!("usage: sunreactorctl test-brightness --monitor-id <CANONICAL_ID> [--json]")
    })?;
    Ok(CliCommand::TestBrightness { monitor_id, json })
}

pub(crate) fn parse_set_command(args: &[String]) -> anyhow::Result<CliCommand> {
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

pub(crate) fn parse_clear_override_command(args: &[String]) -> anyhow::Result<CliCommand> {
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

pub(crate) fn parse_percent(raw: &str) -> anyhow::Result<u8> {
    let percent = raw.parse::<u8>().map_err(|_| {
        anyhow::anyhow!("brightness percent must be an integer in the range 0..=100")
    })?;
    if percent > 100 {
        anyhow::bail!("brightness percent must be an integer in the range 0..=100");
    }
    Ok(percent)
}

pub(crate) fn parse_optional_minutes(args: &[String], usage: &str) -> anyhow::Result<Option<u64>> {
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

#[cfg(test)]
mod tests {
    use super::{
        parse_command, CliCommand, HoldCommand, OverrideTarget, ThemeCommand, WeatherCommand,
    };

    fn parse(args: &[&str]) -> anyhow::Result<CliCommand> {
        parse_command(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_theme_subcommands() {
        let theme = |command| CliCommand::Theme(command);
        assert_eq!(
            parse(&["theme"]).unwrap(),
            theme(ThemeCommand::List { json: false })
        );
        assert_eq!(
            parse(&["theme", "list", "--json"]).unwrap(),
            theme(ThemeCommand::List { json: true })
        );
        assert_eq!(
            parse(&["theme", "show", "nord", "-j"]).unwrap(),
            theme(ThemeCommand::Show {
                id: String::from("nord"),
                json: true
            })
        );
        assert_eq!(
            parse(&["themes", "set", "gameboy"]).unwrap(),
            theme(ThemeCommand::Set {
                id: String::from("gameboy")
            })
        );
        assert_eq!(
            parse(&["theme", "validate"]).unwrap(),
            theme(ThemeCommand::Validate { file: None })
        );
        assert_eq!(
            parse(&["theme", "validate", "/tmp/x.toml"]).unwrap(),
            theme(ThemeCommand::Validate {
                file: Some(std::path::PathBuf::from("/tmp/x.toml"))
            })
        );
        assert_eq!(
            parse(&["theme", "export", "amber"]).unwrap(),
            theme(ThemeCommand::Export {
                id: String::from("amber")
            })
        );
        for bad in [
            &["theme", "set"][..],
            &["theme", "set", "a", "b"],
            &["theme", "show"],
            &["theme", "frobnicate"],
            &["theme", "export", "amber", "--json"],
        ] {
            let error = parse(bad).unwrap_err().to_string();
            assert!(
                error.starts_with("usage: sunreactorctl theme"),
                "{bad:?}: {error}"
            );
        }
    }

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
    fn parses_cloud_dimming_commands_and_bounds_percent() {
        let show = parse_command(&[String::from("weather"), String::from("cloud-dimming")])
            .expect("cloud dimming query should parse");
        assert_eq!(
            show,
            CliCommand::Weather(WeatherCommand::CloudDimming { percent: None })
        );

        let set = parse_command(&[
            String::from("weather"),
            String::from("cloud-dimming"),
            String::from("50"),
        ])
        .expect("cloud dimming update should parse");
        assert_eq!(
            set,
            CliCommand::Weather(WeatherCommand::CloudDimming { percent: Some(50) })
        );

        let error = parse_command(&[
            String::from("weather"),
            String::from("cloud-dimming"),
            String::from("51"),
        ])
        .expect_err("51% must fail");
        assert!(error.to_string().contains("0..=50"));
    }

    #[test]
    fn parses_external_hold_clear_commands() {
        assert_eq!(
            parse(&["hold", "clear"]).unwrap(),
            CliCommand::Hold(HoldCommand::Clear { monitor: None })
        );
        assert_eq!(
            parse(&["hold", "clear", "desk"]).unwrap(),
            CliCommand::Hold(HoldCommand::Clear {
                monitor: Some(String::from("desk")),
            })
        );
        let error = parse(&["hold", "clear", "desk", "extra"]).unwrap_err();
        assert!(error
            .to_string()
            .contains("usage: sunreactorctl hold clear"));
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
}
