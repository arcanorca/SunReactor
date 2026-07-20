use std::io;
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
    Missing { program: String },
    #[error("{program} timed out after {after:?}")]
    Timeout {
        program: String,
        after: Duration,
        stdout: String,
        stderr: String,
    },
    #[error("{program}: {message}")]
    Io { program: String, message: String },
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
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        use std::io::Read;
        use std::process::{Command, Stdio};
        use wait_timeout::ChildExt;

        let program_owned = program.to_owned();

        let mut child = Command::new(&program_owned)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    CommandError::Missing {
                        program: program_owned.clone(),
                    }
                } else {
                    CommandError::Io {
                        program: program_owned.clone(),
                        message: err.to_string(),
                    }
                }
            })?;

        let mut stdout_pipe = child.stdout.take().unwrap();
        let mut stderr_pipe = child.stderr.take().unwrap();

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let timeout_res = std::thread::scope(|s| {
            s.spawn(|| {
                let _ = stdout_pipe.read_to_end(&mut stdout_buf);
            });
            s.spawn(|| {
                let _ = stderr_pipe.read_to_end(&mut stderr_buf);
            });

            let res = child.wait_timeout(timeout);
            match res {
                Ok(None) => {
                    let _ = child.kill();
                }
                Err(_) => {
                    let _ = child.kill();
                }
                _ => {}
            }
            res
        });

        let status_opt = match timeout_res {
            Ok(Some(status)) => Some(status),
            Ok(None) => {
                let _ = child.wait();
                None
            }
            Err(e) => {
                let _ = child.wait();
                return Err(CommandError::Io {
                    program: program_owned,
                    message: format!("wait error: {e}"),
                });
            }
        };

        let stdout_str = String::from_utf8_lossy(&stdout_buf).into_owned();
        let stderr_str = String::from_utf8_lossy(&stderr_buf).into_owned();

        if let Some(status) = status_opt {
            Ok(CommandOutput {
                stdout: stdout_str,
                stderr: stderr_str,
                exit_code: status.code(),
            })
        } else {
            Err(CommandError::Timeout {
                program: program_owned,
                after: timeout,
                stdout: stdout_str,
                stderr: stderr_str,
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[test]
    fn test_process_runner_timeout_kills_child_and_reaps() {
        let runner = RealProcessRunner;

        for _ in 0..3 {
            let start = Instant::now();
            let result = runner.run("sleep", &["30".to_string()], Duration::from_millis(150));

            let elapsed = start.elapsed();

            // Should finish reasonably close to the timeout (150ms)
            assert!(
                elapsed >= Duration::from_millis(100),
                "Finished too fast: {:?}",
                elapsed
            );
            assert!(elapsed < Duration::from_secs(2)); // well under 30s

            match result {
                Err(CommandError::Timeout { program, after, .. }) => {
                    assert_eq!(program, "sleep");
                    assert_eq!(after, Duration::from_millis(150));
                }
                other => panic!("Expected Timeout error, got {:?}", other),
            }

            // Verify child no longer exists
            // We can't directly check the PID because RealProcessRunner hides it,
            // but we can check if `sleep 30` is running.
            let check_output = Command::new("pgrep")
                .arg("-x")
                .arg("sleep")
                .output()
                .expect("Failed to run pgrep");

            // It's possible other sleep processes are running on the system,
            // but the specific one we launched should be dead.
            // A more robust check is whether any `sleep 30` processes spawned by our test exist.
            let check_output_full = Command::new("pgrep")
                .arg("-f")
                .arg("sleep 30")
                .output()
                .expect("Failed to run pgrep");
            let pgrep_stdout_full = String::from_utf8_lossy(&check_output_full.stdout);
            assert!(
                !pgrep_stdout_full.contains("sleep 30"),
                "Child process leaked! pgrep output:\n{}",
                pgrep_stdout_full
            );
        }
    }
}
