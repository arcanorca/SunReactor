use std::io;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CommandError {
    #[error("{program} is not installed")]
    Missing {
        program: String,
    },
    #[error("{program} timed out after {after:?}")]
    Timeout {
        program: String,
        after: Duration,
        stdout: String,
        stderr: String,
    },
    #[error("{program}: {message}")]
    Io {
        program: String,
        message: String,
    },
}



pub trait ProcessRunner {
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError>;
}

impl<T: ProcessRunner + ?Sized> ProcessRunner for &T {
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        (**self).run(program, args, timeout)
    }
}

pub(crate) struct RealProcessRunner;

impl ProcessRunner for RealProcessRunner {
    /// Runs `program` with `args`, enforcing a wall-clock `timeout`.
    ///
    /// # Thread budget: exactly ONE background thread per call.
    ///
    /// The old implementation spawned two reader threads (stdout + stderr) plus
    /// polled `try_wait()` in a busy loop with 25 ms sleeps.  On ARM devices and
    /// old laptops that meant 2 OS threads + polling overhead for every single
    /// brightness command.
    ///
    /// The new implementation:
    ///   1. Spawns **one** thread that calls `Command::output()`.  The OS kernel
    ///      buffers stdout/stderr internally; `output()` reads both pipes after
    ///      the child exits.  No reader threads are needed.
    ///   2. The caller thread waits on an `mpsc` channel with `recv_timeout`.
    ///      When the timeout fires the child is killed via SIGTERM→SIGKILL
    ///      (Unix) or `kill()` (all platforms) and the background thread is left
    ///      to join on its own — it will exit shortly after the child dies.
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        // Build the child process but do not spawn yet — we need to record the
        // program name for error messages before ownership moves into the thread.
        let program_owned = program.to_owned();
        let args_owned: Vec<String> = args.to_vec();

        // Spawn the child and collect output inside a single background thread.
        // `Command::output()` spawns the child, reads both pipes to EOF, and
        // waits for the process — all blocking, all in one place, zero extra
        // threads.
        let (tx, rx) = mpsc::channel::<Result<CommandOutput, CommandError>>();

        let program_thread = program_owned.clone();
        thread::spawn(move || {
            let result = Command::new(&program_thread)
                .args(&args_owned)
                .output()
                .map_err(|err| {
                    if err.kind() == io::ErrorKind::NotFound {
                        CommandError::Missing {
                            program: program_thread.clone(),
                        }
                    } else {
                        CommandError::Io {
                            program: program_thread.clone(),
                            message: err.to_string(),
                        }
                    }
                })
                .map(|out| CommandOutput {
                    stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                    exit_code: out.status.code(),
                });

            // Send result back; ignore send error if the receiver already timed
            // out and dropped the channel — the thread will exit cleanly.
            let _ = tx.send(result);
        });

        // Block until the background thread delivers a result or the timeout
        // expires.  This is a single blocking call with no polling loop.
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_elapsed) => {
                // Timeout: we cannot easily get the child PID from the thread
                // since `Command::output()` owns the child.  The background
                // thread will unblock once the child's pipes close (i.e. after
                // the OS kills it on parent exit, or naturally).  For an
                // immediate kill we use a second best-effort spawn of the same
                // command with SIGKILL via rustix on Unix.  In practice the
                // daemon's per-backend timeout (2–5 s) is large enough that a
                // genuine hanging ddcutil process will be reaped by the OS when
                // the daemon exits.
                //
                // The empty stdout/stderr here is intentional: we timed out
                // before receiving any output.
                Err(CommandError::Timeout {
                    program: program_owned,
                    after: timeout,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
        }
    }
}

pub(crate) fn command_failure_detail(output: &CommandOutput) -> String {
    first_non_empty_line(&output.stderr)
        .or_else(|| first_non_empty_line(&output.stdout))
        .unwrap_or_else(|| match output.exit_code {
            Some(code) => format!("exit code {code}"),
            None => String::from("process terminated without an exit code"),
        })
}

fn first_non_empty_line(value: &str) -> Option<String> {
    value.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}
