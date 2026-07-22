#![allow(clippy::unnecessary_wraps)]

use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sunreactor::{daemon_help, runtime::DaemonRuntime, version};

fn main() -> anyhow::Result<()> {
    if let Err(error) = try_main() {
        eprintln!("sunreactord error:");
        for cause in error.chain() {
            eprintln!("  caused by: {cause}");
        }
        process::exit(1);
    }
    Ok(())
}

fn try_main() -> anyhow::Result<()> {
    let command = parse_args(&env::args().skip(1).collect::<Vec<_>>())?;

    match command {
        DaemonCommand::Help => {
            print!("{}", daemon_help());
            Ok(())
        }
        DaemonCommand::Version => {
            println!("sunreactord {}", version());
            Ok(())
        }
        DaemonCommand::Run { once } => {
            let shutdown_flag = Arc::new(AtomicBool::new(false));
            install_shutdown_handlers(Arc::clone(&shutdown_flag))?;
            let mut runtime = DaemonRuntime::bootstrap()?;

            if once {
                println!(
                    "level=info event=startup mode=once startup=\"{}\"",
                    runtime
                        .startup_message()
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                );
                let report = runtime.run_once()?;
                println!(
                    "level=info event=tick mode=once tick_duration_ms={} monitors_evaluated={} writes_attempted={} writes_skipped={} failures={}",
                    report.tick_duration.as_millis(),
                    report.monitors_evaluated,
                    report.apply_summary.attempted,
                    report.apply_summary.skipped,
                    report.apply_summary.failed,
                );
                println!("level=info event=shutdown mode=once reason=completed");
                Ok(())
            } else {
                runtime.run_loop(|| shutdown_flag.load(Ordering::Relaxed))?;
                Ok(())
            }
        }
    }
}

enum DaemonCommand {
    Help,
    Version,
    Run { once: bool },
}

fn parse_args(args: &[String]) -> anyhow::Result<DaemonCommand> {
    let mut once = false;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(DaemonCommand::Help),
            "-V" | "--version" => return Ok(DaemonCommand::Version),
            "--once" => once = true,
            _ => anyhow::bail!("unknown option: {arg}"),
        }
    }

    Ok(DaemonCommand::Run { once })
}

fn install_shutdown_handlers(shutdown_flag: Arc<AtomicBool>) -> anyhow::Result<()> {
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown_flag))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, shutdown_flag)?;
    Ok(())
}
