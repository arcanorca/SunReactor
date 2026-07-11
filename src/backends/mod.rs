//! Hardware interaction layer for applying brightness targets.
//!
//! This module defines the common interfaces and error types for dispatching
//! brightness changes to physical hardware backends, such as `ddcutil` for
//! external monitors and `brightnessctl` for internal laptop panels.

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub mod backlight;
pub mod ddc;
pub(crate) use crate::process::{CommandError, CommandOutput, ProcessRunner, RealProcessRunner};

/// Defines the physical communication channel for a monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Backlight,
    Ddc,
}

/// Categorizes backend failures to determine whether they should be retried
/// immediately or subjected to exponential backoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Transient,
    Persistent,
}

/// Represents a completed attempt to mutate hardware state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendWrite {
    pub backend: BackendKind,
    pub applied_percent: u8,
    pub attempts: u8,
    pub detail: String,
}

/// Structured error representing all potential failure modes when
/// interfacing with physical hardware control binaries.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendError {
    #[error("{backend:?} backend requires {expected}")]
    MissingSelector {
        backend: BackendKind,
        expected: &'static str,
    },
    #[error("{backend:?} selector `{field}` is invalid: {message}")]
    InvalidSelector {
        backend: BackendKind,
        field: &'static str,
        message: String,
    },
    #[error("{program} is not installed")]
    MissingProgram {
        backend: BackendKind,
        program: String,
        attempts: u8,
    },
    #[error("{program} timed out after {}s: {detail}", .after.as_secs())]
    CommandTimeout {
        backend: BackendKind,
        program: String,
        after: Duration,
        detail: String,
        attempts: u8,
    },
    #[error("{program} failed (exit code {exit_code:?}): {detail}")]
    CommandFailed {
        backend: BackendKind,
        program: String,
        exit_code: Option<i32>,
        detail: String,
        transient: bool,
        attempts: u8,
    },
    #[error("{program}: {message}")]
    Io {
        backend: BackendKind,
        program: String,
        message: String,
        attempts: u8,
    },
}

impl BackendError {
    #[must_use]
    pub fn failure_kind(&self) -> FailureKind {
        match self {
            Self::MissingSelector { .. }
            | Self::InvalidSelector { .. }
            | Self::MissingProgram { .. } => FailureKind::Persistent,
            Self::CommandTimeout { .. } => FailureKind::Transient,
            Self::CommandFailed { transient, .. } => {
                if *transient {
                    FailureKind::Transient
                } else {
                    FailureKind::Persistent
                }
            }
            Self::Io { message, .. } => classify_failure_detail(message),
        }
    }

    #[must_use]
    pub fn attempts(&self) -> u8 {
        match self {
            Self::MissingSelector { .. } | Self::InvalidSelector { .. } => 0,
            Self::MissingProgram { attempts, .. }
            | Self::CommandTimeout { attempts, .. }
            | Self::CommandFailed { attempts, .. }
            | Self::Io { attempts, .. } => *attempts,
        }
    }

    #[must_use]
    pub fn with_attempts(self, attempts: u8) -> Self {
        match self {
            Self::MissingSelector { .. } | Self::InvalidSelector { .. } => self,
            Self::MissingProgram {
                backend, program, ..
            } => Self::MissingProgram {
                backend,
                program,
                attempts,
            },
            Self::CommandTimeout {
                backend,
                program,
                after,
                detail,
                ..
            } => Self::CommandTimeout {
                backend,
                program,
                after,
                detail,
                attempts,
            },
            Self::CommandFailed {
                backend,
                program,
                exit_code,
                detail,
                transient,
                ..
            } => Self::CommandFailed {
                backend,
                program,
                exit_code,
                detail,
                transient,
                attempts,
            },
            Self::Io {
                backend,
                program,
                message,
                ..
            } => Self::Io {
                backend,
                program,
                message,
                attempts,
            },
        }
    }
}

pub(crate) fn clamp_percent(percent: u8) -> u8 {
    percent.min(100)
}

pub(crate) fn command_failure_detail(output: &CommandOutput) -> String {
    first_non_empty_line(&output.stderr)
        .or_else(|| first_non_empty_line(&output.stdout))
        .unwrap_or_else(|| match output.exit_code {
            Some(code) => format!("exit code {code}"),
            None => String::from("process terminated without an exit code"),
        })
}

pub(crate) fn map_command_error(backend: BackendKind, error: CommandError) -> BackendError {
    match error {
        CommandError::Missing { program } => BackendError::MissingProgram {
            backend,
            program,
            attempts: 1,
        },
        CommandError::Timeout {
            program,
            after,
            stdout,
            stderr,
        } => BackendError::CommandTimeout {
            backend,
            program,
            after,
            detail: first_non_empty_line(&stderr)
                .or_else(|| first_non_empty_line(&stdout))
                .unwrap_or_else(|| String::from("process timed out")),
            attempts: 1,
        },
        CommandError::Io { program, message } => BackendError::Io {
            backend,
            program,
            message,
            attempts: 1,
        },
    }
}

pub(crate) fn command_failure(
    backend: BackendKind,
    program: &str,
    output: &CommandOutput,
) -> BackendError {
    let detail = command_failure_detail(output);
    BackendError::CommandFailed {
        backend,
        program: program.to_owned(),
        exit_code: output.exit_code,
        transient: is_transient_failure(&detail),
        detail,
        attempts: 1,
    }
}

fn is_transient_failure(detail: &str) -> bool {
    classify_failure_detail(detail) == FailureKind::Transient
}

fn classify_failure_detail(detail: &str) -> FailureKind {
    let detail = detail.to_ascii_lowercase();
    let transient = [
        "busy",
        "temporarily unavailable",
        "resource temporarily unavailable",
        "resource busy",
        "retry",
        "i/o error",
        "io error",
        "communication error",
        "no monitor",
        "monitor not found",
        "display not found",
        "device not found",
        "no backlight device",
        "no such file or directory",
        "disconnected",
    ]
    .iter()
    .any(|needle| detail.contains(needle));

    if transient {
        FailureKind::Transient
    } else {
        FailureKind::Persistent
    }
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
pub(crate) mod testutil {
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::Mutex;
    use std::time::Duration;

    use crate::process::{CommandError, CommandOutput, ProcessRunner};

    /// A thread-safe fake runner for tests.
    ///
    /// Uses `Mutex` instead of `RefCell` so that `FakeRunner: Sync`, which is
    /// required when passing it to the concurrent apply engine via
    /// `thread::scope`.
    pub struct FakeRunner {
        responses: Mutex<BTreeMap<String, VecDeque<Result<CommandOutput, CommandError>>>>,
        calls: Mutex<Vec<String>>,
    }

    impl Default for FakeRunner {
        fn default() -> Self {
            Self {
                responses: Mutex::new(BTreeMap::new()),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl FakeRunner {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_success(self, program: &str, args: &[&str], stdout: &str) -> Self {
            self.with_output(program, args, Some(0), stdout, "")
        }

        pub fn with_output(
            self,
            program: &str,
            args: &[&str],
            exit_code: Option<i32>,
            stdout: &str,
            stderr: &str,
        ) -> Self {
            self.push_response(
                program,
                args,
                Ok(CommandOutput {
                    stdout: stdout.to_owned(),
                    stderr: stderr.to_owned(),
                    exit_code,
                }),
            );
            self
        }

        pub fn with_timeout(
            self,
            program: &str,
            args: &[&str],
            after: Duration,
            stderr: &str,
        ) -> Self {
            self.push_response(
                program,
                args,
                Err(CommandError::Timeout {
                    program: program.to_owned(),
                    after,
                    stdout: String::new(),
                    stderr: stderr.to_owned(),
                }),
            );
            self
        }

        pub fn push_response(
            &self,
            program: &str,
            args: &[&str],
            response: Result<CommandOutput, CommandError>,
        ) {
            self.responses
                .lock()
                .unwrap()
                .entry(command_key(program, args))
                .or_default()
                .push_back(response);
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl ProcessRunner for FakeRunner {
        fn run(
            &self,
            program: &str,
            args: &[String],
            _timeout: Duration,
        ) -> Result<CommandOutput, CommandError> {
            let key = command_key_owned(program, args);
            self.calls.lock().unwrap().push(key.clone());
            self.responses
                .lock()
                .unwrap()
                .get_mut(&key)
                .and_then(|responses| responses.pop_front())
                .unwrap_or_else(|| {
                    Err(CommandError::Io {
                        program: program.to_owned(),
                        message: format!("unexpected command: {key}"),
                    })
                })
        }
    }

    pub fn command_key(program: &str, args: &[&str]) -> String {
        let mut key = String::from(program);
        for arg in args {
            key.push('|');
            key.push_str(arg);
        }
        key
    }

    pub fn command_key_owned(program: &str, args: &[String]) -> String {
        let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
        command_key(program, &borrowed)
    }
}
