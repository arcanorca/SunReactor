use std::fmt;
use std::io::{self, Read};
use std::process::{Child, Command, Stdio};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

#[cfg(target_os = "linux")]
use rustix::process::{waitid, Pid, WaitId, WaitidOptions};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Maximum bytes captured per output stream before the command is killed.
const MAX_CAPTURE_BYTES: usize = 1024 * 1024;
/// How long to wait for a SIGKILLed child before abandoning it to a reaper.
const REAP_GRACE: Duration = Duration::from_secs(2);

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

impl std::error::Error for CommandError {}

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
    /// Runs a command as an isolated process group while retaining ownership of
    /// its direct child until cleanup is complete. Stdout and stderr are drained
    /// concurrently (each capped at [`MAX_CAPTURE_BYTES`]) so verbose children
    /// cannot block on full pipes. Cleanup after a kill is bounded by
    /// [`REAP_GRACE`]: a child stuck in uninterruptible sleep is handed to a
    /// detached reaper thread instead of blocking the caller.
    #[allow(clippy::too_many_lines)]
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        let mut command = Command::new(program);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .spawn()
            .map_err(|error| map_spawn_error(program, &error))?;

        let Some(stdout) = child.stdout.take() else {
            let _ = terminate_process_group(&child);
            abandon_to_reaper(child, Vec::new());
            return Err(CommandError::Io {
                program: program.to_owned(),
                message: String::from("failed to capture stdout"),
            });
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = terminate_process_group(&child);
            abandon_to_reaper(child, Vec::new());
            return Err(CommandError::Io {
                program: program.to_owned(),
                message: String::from("failed to capture stderr"),
            });
        };
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout_thread = spawn_reader(stdout, MAX_CAPTURE_BYTES, Arc::clone(&overflow));
        let stderr_thread = spawn_reader(stderr, MAX_CAPTURE_BYTES, Arc::clone(&overflow));
        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(Instant::now);

        let child_exited = loop {
            if overflow.load(Ordering::Acquire) {
                break Err(());
            }
            match child_exited_without_reaping(&child) {
                Ok(true) => break Ok(()),
                Ok(false) if Instant::now() >= deadline => break Err(()),
                Ok(false) => thread::sleep(
                    POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
                ),
                Err(error) => {
                    let _ = terminate_process_group(&child);
                    abandon_to_reaper(child, vec![stdout_thread, stderr_thread]);
                    return Err(CommandError::Io {
                        program: program.to_owned(),
                        message: error.to_string(),
                    });
                }
            }
        };

        if child_exited.is_err() {
            let kill_error = terminate_process_group(&child).err();
            let grace_deadline = Instant::now() + REAP_GRACE;
            if !wait_reaped_bounded(&mut child, grace_deadline) {
                // Uninterruptible child: never block the caller on it.
                abandon_to_reaper(child, vec![stdout_thread, stderr_thread]);
                return Err(CommandError::Timeout {
                    program: program.to_owned(),
                    after: timeout,
                    stdout: String::new(),
                    stderr: format!(
                        "child did not exit {}s after SIGKILL; abandoned to reaper",
                        REAP_GRACE.as_secs()
                    ),
                });
            }
            let stdout_result =
                join_reader_bounded(stdout_thread, program, "stdout", grace_deadline);
            let stderr_result =
                join_reader_bounded(stderr_thread, program, "stderr", grace_deadline);

            if let ReaderOutcome::Failed(error) = &stdout_result {
                return Err(error.clone());
            }
            if let ReaderOutcome::Failed(error) = &stderr_result {
                return Err(error.clone());
            }
            if let Some(error) = kill_error {
                // The group may have disappeared between `try_wait` and the
                // signal; the successful wait still proves direct-child reap.
                if error.kind() != io::ErrorKind::NotFound {
                    return Err(CommandError::Io {
                        program: program.to_owned(),
                        message: format!("failed to terminate timed-out process group: {error}"),
                    });
                }
            }
            return Err(CommandError::Timeout {
                program: program.to_owned(),
                after: timeout,
                stdout: stdout_result.into_text(),
                stderr: stderr_result.into_text(),
            });
        }

        // The non-reaping observation above guarantees the direct child is
        // still waitable while this numeric process-group signal is sent.
        // Only after group cleanup may std::process::Child reap it.
        // A direct child can exit while a same-group descendant still owns
        // the capture pipes, so close that group before joining readers.
        // ESRCH is harmless when the group became empty with the child exit.
        let group_result = terminate_process_group(&child);
        let grace_deadline = Instant::now() + REAP_GRACE;
        if !wait_reaped_bounded(&mut child, grace_deadline) {
            abandon_to_reaper(child, vec![stdout_thread, stderr_thread]);
            return Err(CommandError::Io {
                program: program.to_owned(),
                message: String::from("failed to reap completed child within grace period"),
            });
        }
        let status = child
            .try_wait()
            .ok()
            .flatten()
            .ok_or_else(|| CommandError::Io {
                program: program.to_owned(),
                message: String::from("failed to reap completed child"),
            })?;
        let stdout = join_reader_bounded(stdout_thread, program, "stdout", grace_deadline);
        let stderr = join_reader_bounded(stderr_thread, program, "stderr", grace_deadline);
        for outcome in [&stdout, &stderr] {
            match outcome {
                ReaderOutcome::Failed(error) => return Err(error.clone()),
                ReaderOutcome::Stuck => {
                    return Err(CommandError::Io {
                        program: program.to_owned(),
                        message: String::from("output pipe did not close after child exit"),
                    })
                }
                ReaderOutcome::Text(_) => {}
            }
        }
        if let Err(error) = group_result {
            if error.kind() != io::ErrorKind::NotFound {
                return Err(CommandError::Io {
                    program: program.to_owned(),
                    message: format!("failed to terminate exited process group: {error}"),
                });
            }
        }
        Ok(CommandOutput {
            stdout: stdout.into_text(),
            stderr: stderr.into_text(),
            exit_code: status.code(),
        })
    }
}

/// Polls `try_wait` until the child is reaped or `deadline` passes.
fn wait_reaped_bounded(child: &mut Child, deadline: Instant) -> bool {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            // ECHILD and friends: nothing left to wait for.
            Err(_) => return true,
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        thread::sleep(POLL_INTERVAL.min(deadline - now));
    }
}

/// Hands a child that would not die (and its reader threads) to a detached
/// thread that blocks on `wait`, so the zombie is reaped whenever the kernel
/// finally releases it and the caller never blocks.
fn abandon_to_reaper(mut child: Child, readers: Vec<JoinHandle<io::Result<Vec<u8>>>>) {
    let spawned = thread::Builder::new()
        .name(String::from("process-reaper"))
        .spawn(move || {
            let _ = child.wait();
            for reader in readers {
                let _ = reader.join();
            }
        });
    if let Err(error) = spawned {
        tracing::warn!(error = %error, "process_reaper_spawn_failed");
    }
}

enum ReaderOutcome {
    Text(String),
    Failed(CommandError),
    Stuck,
}

impl ReaderOutcome {
    fn into_text(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Failed(_) | Self::Stuck => String::new(),
        }
    }
}

fn join_reader_bounded(
    handle: JoinHandle<io::Result<Vec<u8>>>,
    program: &str,
    stream: &str,
    deadline: Instant,
) -> ReaderOutcome {
    loop {
        if handle.is_finished() {
            return match join_reader(handle, program, stream) {
                Ok(text) => ReaderOutcome::Text(text),
                Err(error) => ReaderOutcome::Failed(error),
            };
        }
        let now = Instant::now();
        if now >= deadline {
            abandon_reader(handle);
            return ReaderOutcome::Stuck;
        }
        thread::sleep(POLL_INTERVAL.min(deadline - now));
    }
}

fn abandon_reader(handle: JoinHandle<io::Result<Vec<u8>>>) {
    let _ = thread::Builder::new()
        .name(String::from("process-reaper"))
        .spawn(move || {
            let _ = handle.join();
        });
}

#[cfg(target_os = "linux")]
fn child_exited_without_reaping(child: &Child) -> io::Result<bool> {
    let pid = Pid::from_child(child);
    let status = waitid(
        WaitId::Pid(pid),
        WaitidOptions::EXITED | WaitidOptions::NOHANG | WaitidOptions::NOWAIT,
    )?;
    Ok(status.is_some_and(|status| status.exited()))
}

#[cfg(target_os = "windows")]
#[allow(clippy::unnecessary_wraps)]
fn child_exited_without_reaping(child: &Child) -> io::Result<bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };

    let pid = child.id();
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return Ok(true);
    }
    let wait_res = unsafe { WaitForSingleObject(handle, 0) };
    unsafe { CloseHandle(handle) };
    Ok(wait_res == WAIT_OBJECT_0)
}

#[cfg(unix)]
fn terminate_process_group(child: &std::process::Child) -> io::Result<()> {
    let pgid = child.id() as libc::pid_t;
    let result = unsafe { libc::kill(-pgid, libc::SIGKILL) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "windows")]
fn terminate_process_group(child: &std::process::Child) -> io::Result<()> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    let pid = child.id();
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let res = unsafe { TerminateProcess(handle, 1) };
    unsafe { CloseHandle(handle) };
    if res != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn map_spawn_error(program: &str, error: &io::Error) -> CommandError {
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

fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 8192];
        loop {
            let read = match reader.read(&mut chunk) {
                Ok(0) => return Ok(bytes),
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            if bytes.len() + read > limit {
                // Dropping the reader closes the pipe; the supervisor sees the
                // flag and terminates the process group.
                overflow.store(true, Ordering::Release);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("output exceeded {limit} bytes"),
                ));
            }
            bytes.extend_from_slice(&chunk[..read]);
        }
    })
}

fn join_reader(
    handle: JoinHandle<io::Result<Vec<u8>>>,
    program: &str,
    stream: &str,
) -> Result<String, CommandError> {
    let bytes = handle
        .join()
        .map_err(|_| CommandError::Io {
            program: program.to_owned(),
            message: format!("{stream} reader thread panicked"),
        })?
        .map_err(|error| CommandError::Io {
            program: program.to_owned(),
            message: format!("failed to read {stream}: {error}"),
        })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
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

#[cfg(unix)]
#[cfg(test)]
mod tests {
    use super::{
        child_exited_without_reaping, Command, CommandError, ProcessRunner, RealProcessRunner,
    };
    use std::os::unix::process::CommandExt;
    use std::time::{Duration, Instant};

    fn run_script(script: &str, timeout: Duration) -> Result<super::CommandOutput, CommandError> {
        run_script_with_args(script, &[], timeout)
    }

    fn run_script_with_args(
        script: &str,
        script_args: &[&str],
        timeout: Duration,
    ) -> Result<super::CommandOutput, CommandError> {
        let mut args = vec![
            String::from("-c"),
            script.to_owned(),
            String::from("sunreactor-test"),
        ];
        args.extend(script_args.iter().map(|arg| (*arg).to_owned()));
        RealProcessRunner.run("sh", &args, timeout)
    }

    #[test]
    fn normal_completion_captures_stdout_and_stderr() {
        let output = run_script(
            "printf normal-out; printf normal-err >&2",
            Duration::from_secs(2),
        )
        .expect("command should complete");
        assert!(output.success());
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout, "normal-out");
        assert_eq!(output.stderr, "normal-err");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn non_reaping_observation_leaves_child_waitable_until_final_wait() {
        let mut child = Command::new("sh")
            .args([String::from("-c"), String::from("exit 23")])
            .process_group(0)
            .spawn()
            .expect("child should spawn");

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if child_exited_without_reaping(&child).expect("waitid should work") {
                let status = child.wait().expect("final wait should reap child");
                assert_eq!(status.code(), Some(23));
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!("child did not exit within the test bound");
    }

    #[test]
    fn timeout_kills_and_reaps_direct_child() {
        let started = Instant::now();
        let pid_file = tempfile::NamedTempFile::new().expect("pid file should be created");
        let error = run_script_with_args(
            "printf '%s' \"$$\" > \"$1\"; printf before-timeout; printf error-before-timeout >&2; exec sleep 30",
            &[pid_file.path().to_str().expect("pid path should be UTF-8")],
            Duration::from_millis(100),
        )
        .expect_err("long-running command should time out");
        assert!(started.elapsed() < Duration::from_secs(5));
        let CommandError::Timeout { stdout, stderr, .. } = error else {
            panic!("long-running command did not time out");
        };
        assert!(stdout.contains("before-timeout"));
        assert!(stderr.contains("error-before-timeout"));
        let pid: i32 = std::fs::read_to_string(pid_file.path())
            .expect("pid should be recorded")
            .parse()
            .expect("pid should be numeric");
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if !process_exists(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timed-out child {pid} is still present");
    }

    fn process_exists(pid: i32) -> bool {
        std::path::Path::new("/proc").join(pid.to_string()).exists()
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn inherited_pipe_writer_is_cleaned_after_direct_child_exit() {
        let pid_file = tempfile::NamedTempFile::new().expect("pid file should be created");
        let started = Instant::now();
        let output = run_script_with_args(
            "(printf descendant-marker; exec sleep 30) & printf '%s' \"$!\" > \"$1\"; printf direct-marker; exit 0",
            &[pid_file.path().to_str().expect("pid path should be UTF-8")],
            Duration::from_secs(2),
        )
        .expect("direct child should exit normally");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(output.stdout.contains("direct-marker"));
        assert!(output.stdout.contains("descendant-marker"));
        assert_descendant_gone(&pid_file);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn timeout_with_inherited_pipe_writer_cleans_process_group() {
        let pid_file = tempfile::NamedTempFile::new().expect("pid file should be created");
        let started = Instant::now();
        let error = run_script_with_args(
            "(printf descendant-marker; exec sleep 30) & printf '%s' \"$!\" > \"$1\"; printf direct-marker; sleep 30",
            &[pid_file.path().to_str().expect("pid path should be UTF-8")],
            Duration::from_millis(100),
        )
        .expect_err("direct child should time out");
        assert!(started.elapsed() < Duration::from_secs(5));
        let CommandError::Timeout { stdout, .. } = error else {
            panic!("long-running command did not time out");
        };
        assert!(stdout.contains("direct-marker"));
        assert!(stdout.contains("descendant-marker"));
        assert_descendant_gone(&pid_file);
    }

    #[cfg(target_os = "linux")]
    fn assert_descendant_gone(pid_file: &tempfile::NamedTempFile) {
        let pid: i32 = std::fs::read_to_string(pid_file.path())
            .expect("descendant pid should be recorded")
            .parse()
            .expect("descendant pid should be numeric");
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if !std::path::Path::new("/proc").join(pid.to_string()).exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("descendant {pid} is still present");
    }

    #[test]
    fn timeout_path_handles_output_larger_than_pipe_buffers() {
        let started = Instant::now();
        let error = run_script(
            "dd if=/dev/zero bs=1k count=512 2>/dev/null; dd if=/dev/zero bs=1k count=512 >&2 2>/dev/null; exec sleep 30",
            Duration::from_millis(400),
        )
        .expect_err("command should time out after producing large output");
        assert!(started.elapsed() < Duration::from_secs(5));
        let CommandError::Timeout { stdout, stderr, .. } = error else {
            panic!("large-output command did not time out");
        };
        assert!(stdout.len() >= 512 * 1024);
        assert!(stderr.len() >= 512 * 1024);
    }

    #[test]
    fn repeated_timeouts_return_without_accumulating_workers() {
        for _ in 0..8 {
            let error = run_script("exec sleep 30", Duration::from_millis(50))
                .expect_err("command should time out");
            assert!(matches!(error, CommandError::Timeout { .. }));
        }
    }

    #[test]
    fn stdout_over_cap_is_reported_as_error_and_child_is_killed() {
        let started = Instant::now();
        let error = run_script(
            "dd if=/dev/zero bs=1k count=2048 2>/dev/null; exec sleep 30",
            Duration::from_secs(10),
        )
        .expect_err("oversized stdout must be an error");
        assert!(started.elapsed() < Duration::from_secs(5));
        let CommandError::Io { message, .. } = error else {
            panic!("expected Io error, got {error:?}");
        };
        assert!(message.contains("exceeded"), "{message}");
    }

    #[test]
    fn abandoned_child_is_reaped_by_detached_thread() {
        let mut child = Command::new("sh")
            .args(["-c", "sleep 0.4"])
            .spawn()
            .expect("child should spawn");
        let pid = child.id() as i32;
        let bounded =
            super::wait_reaped_bounded(&mut child, Instant::now() + Duration::from_millis(50));
        assert!(
            !bounded,
            "child must still be running after the short grace"
        );
        super::abandon_to_reaper(child, Vec::new());
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !process_exists(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("abandoned child {pid} was never reaped (zombie)");
    }

    #[test]
    fn missing_executable_preserves_missing_error() {
        let error = RealProcessRunner
            .run(
                "/sunreactor/nonexistent/process",
                &[],
                Duration::from_secs(1),
            )
            .expect_err("missing executable should fail");
        assert!(matches!(error, CommandError::Missing { .. }));
    }

    #[test]
    fn nonzero_exit_preserves_output_and_exit_code() {
        let output = run_script(
            "printf failed-out; printf failed-err >&2; exit 23",
            Duration::from_secs(2),
        )
        .expect("spawn should succeed despite nonzero exit");
        assert!(!output.success());
        assert_eq!(output.exit_code, Some(23));
        assert_eq!(output.stdout, "failed-out");
        assert_eq!(output.stderr, "failed-err");
    }
}
