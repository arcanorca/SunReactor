use std::fmt;
use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use wait_timeout::ChildExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: Option<i32>,
}

impl CommandOutput {
    pub(crate) fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandError {
    Missing {
        program: String,
    },
    Timeout {
        program: String,
        after: Duration,
        stdout: String,
        stderr: String,
    },
    Io {
        program: String,
        message: String,
    },
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { program } => write!(f, "{program} is not installed"),
            Self::Timeout {
                program,
                after,
                stdout,
                stderr,
            } => {
                let detail = first_non_empty_line(stderr)
                    .or_else(|| first_non_empty_line(stdout))
                    .unwrap_or_else(|| String::from("process timed out"));
                write!(
                    f,
                    "{program} timed out after {}s: {detail}",
                    after.as_secs()
                )
            }
            Self::Io { program, message } => write!(f, "{program}: {message}"),
        }
    }
}

impl std::error::Error for CommandError {}

pub(crate) trait ProcessRunner {
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError>;
}

pub(crate) struct RealProcessRunner;

impl ProcessRunner for RealProcessRunner {
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| map_spawn_error(program, error))?;

        let stdout = child.stdout.take().expect("piped stdout must be present");
        let stderr = child.stderr.take().expect("piped stderr must be present");
        let stdout_reader = thread::spawn(move || read_pipe(stdout));
        let stderr_reader = thread::spawn(move || read_pipe(stderr));

        let wait_result = child.wait_timeout(timeout);
        let timed_out = match wait_result {
            Ok(Some(_)) => false,
            Ok(None) => {
                // Killing and then waiting is mandatory: returning before wait()
                // would leave a live process or zombie behind.
                let _ = child.kill();
                child.wait().map_err(|error| CommandError::Io {
                    program: program.to_owned(),
                    message: format!("failed to reap timed-out child: {error}"),
                })?;
                true
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CommandError::Io {
                    program: program.to_owned(),
                    message: format!("failed while waiting for child: {error}"),
                });
            }
        };

        let stdout = join_reader(program, "stdout", stdout_reader)?;
        let stderr = join_reader(program, "stderr", stderr_reader)?;
        let stdout = String::from_utf8_lossy(&stdout).into_owned();
        let stderr = String::from_utf8_lossy(&stderr).into_owned();

        if timed_out {
            Err(CommandError::Timeout {
                program: program.to_owned(),
                after: timeout,
                stdout,
                stderr,
            })
        } else {
            let status = child.wait().map_err(|error| CommandError::Io {
                program: program.to_owned(),
                message: format!("failed to collect child status: {error}"),
            })?;
            Ok(CommandOutput {
                stdout,
                stderr,
                exit_code: status.code(),
            })
        }
    }
}

fn map_spawn_error(program: &str, error: io::Error) -> CommandError {
    if error.kind() == io::ErrorKind::NotFound {
        CommandError::Missing {
            program: program.to_owned(),
        }
    } else {
        CommandError::Io {
            program: program.to_owned(),
            message: error.to_string(),
        }
    }
}

fn read_pipe<R: Read>(mut reader: R) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(
    program: &str,
    stream: &str,
    reader: thread::JoinHandle<io::Result<Vec<u8>>>,
) -> Result<Vec<u8>, CommandError> {
    reader
        .join()
        .map_err(|_| CommandError::Io {
            program: program.to_owned(),
            message: format!("{stream} reader thread panicked"),
        })?
        .map_err(|error| CommandError::Io {
            program: program.to_owned(),
            message: format!("failed to read child {stream}: {error}"),
        })
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

#[cfg(all(test, unix))]
mod tests {
    use super::{CommandError, ProcessRunner, RealProcessRunner};
    use std::fs;
    use std::time::{Duration, Instant};

    #[test]
    fn timeout_kills_and_reaps_the_exact_child() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let pid_file = temp.path().join("child.pid");
        let args = vec![
            String::from("-c"),
            String::from("echo $$ > \"$1\"; exec sleep 30"),
            String::from("sunreactor-timeout-test"),
            pid_file.display().to_string(),
        ];
        let started = Instant::now();

        let error = RealProcessRunner
            .run("sh", &args, Duration::from_millis(150))
            .expect_err("child should time out");

        assert!(matches!(error, CommandError::Timeout { .. }));
        assert!(started.elapsed() < Duration::from_secs(2));
        let pid = fs::read_to_string(&pid_file)
            .expect("child should record its pid")
            .trim()
            .parse::<u32>()
            .expect("pid should be numeric");
        assert!(
            !std::path::Path::new(&format!("/proc/{pid}")).exists(),
            "timed-out child {pid} still exists"
        );
    }
}
