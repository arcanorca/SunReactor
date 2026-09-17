#![warn(clippy::pedantic)]
#![allow(clippy::unnecessary_wraps)]

use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use sunreactor::{daemon_help, runtime::DaemonRuntime, version};

static TRACING_INITIALIZED: OnceLock<Result<(), String>> = OnceLock::new();

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
            initialize_tracing()?;
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
                runtime.refresh_capabilities();
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

fn initialize_tracing() -> anyhow::Result<()> {
    initialize_tracing_with(
        &TRACING_INITIALIZED,
        || daemon_subscriber(std::io::stderr),
        |subscriber| {
            tracing::subscriber::set_global_default(subscriber)
                .map_err(|error| format!("failed to initialize daemon tracing: {error}"))
        },
    )
}

fn initialize_tracing_with<S, Build, Install>(
    state: &OnceLock<Result<(), String>>,
    build: Build,
    install: Install,
) -> anyhow::Result<()>
where
    S: tracing::Subscriber,
    Build: FnOnce() -> S,
    Install: FnOnce(S) -> Result<(), String>,
{
    state
        .get_or_init(|| install(build()))
        .clone()
        .map_err(anyhow::Error::msg)
}

fn daemon_subscriber<W>(writer: W) -> impl tracing::Subscriber
where
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .compact()
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(writer)
        .finish()
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
    #[cfg(target_os = "linux")]
    {
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown_flag))?;
        signal_hook::flag::register(signal_hook::consts::SIGTERM, shutdown_flag)?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        sunreactor::platform::windows::register_console_shutdown_handler(shutdown_flag)
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex, OnceLock};

    #[derive(Clone)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("sink mutex").extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn daemon_subscriber_writes_structured_events_to_its_sink() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let sink_output = Arc::clone(&output);
        let state = OnceLock::new();
        super::initialize_tracing_with(
            &state,
            move || super::daemon_subscriber(move || SharedWriter(sink_output.clone())),
            |subscriber| {
                tracing::subscriber::with_default(subscriber, || {
                    tracing::info!(correlation_id = 41, "wake_hint_consumed");
                });
                Ok(())
            },
        )
        .expect("tracing initialization should succeed");

        let rendered = String::from_utf8(output.lock().expect("sink mutex").clone())
            .expect("sink output should be utf8");
        assert!(rendered.contains("wake_hint_consumed"));
        assert!(rendered.contains("correlation_id=41"));
    }
}
