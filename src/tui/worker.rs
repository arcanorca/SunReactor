use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::model::{ActionKind, DaemonConnection, ErrorCategory};
use crate::ipc::{ControlSocket, IpcError, Request, RequestEnvelope, Response, StatusResponse};

pub(crate) enum IpcCommand {
    Send {
        command_id: u64,
        action: ActionKind,
        request: Request,
    },
    DiscoverMonitors,
    Shutdown,
}

pub(crate) enum IpcEvent {
    Status(Box<StatusResponse>),
    MonitorDiscovery(Box<crate::discovery::DiscoveryReport>),
    Disconnected,
    Connected,
    CommandSucceeded {
        command_id: u64,
        action: ActionKind,
        message: String,
    },
    CommandFailed {
        command_id: u64,
        action: ActionKind,
        error_category: ErrorCategory,
        message: String,
    },
}

/// Where the worker reaches the daemon and whether it may probe hardware.
#[derive(Debug, Clone, Default)]
pub(crate) struct WorkerTarget {
    /// `None` resolves the user's runtime control endpoint.
    pub socket_override: Option<ControlSocket>,
    /// Disabled in unit tests so no `ddcutil` process is ever started.
    pub hardware_discovery: bool,
}

impl WorkerTarget {
    fn socket(&self) -> Result<ControlSocket, crate::paths::PathError> {
        match &self.socket_override {
            Some(socket) => Ok(socket.clone()),
            None => ControlSocket::from_runtime(),
        }
    }
}

pub(crate) fn spawn_ipc_worker(
    poll_interval: Duration,
    target: WorkerTarget,
) -> (mpsc::SyncSender<IpcCommand>, mpsc::Receiver<IpcEvent>) {
    let (cmd_tx, cmd_rx) = mpsc::sync_channel::<IpcCommand>(64);
    let (evt_tx, evt_rx) = mpsc::sync_channel::<IpcEvent>(64);

    thread::spawn(move || {
        let mut connection_state = DaemonConnection::Unknown;

        loop {
            loop {
                match cmd_rx.try_recv() {
                    Ok(IpcCommand::Shutdown) => return,
                    Ok(IpcCommand::Send {
                        command_id,
                        action,
                        request,
                    }) => {
                        send_ipc_request(
                            &target,
                            command_id,
                            action,
                            request,
                            &evt_tx,
                            &mut connection_state,
                        );
                    }
                    Ok(IpcCommand::DiscoverMonitors) => {
                        if target.hardware_discovery {
                            run_monitor_discovery(&evt_tx);
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }

            poll_daemon_status(&target, &evt_tx, &mut connection_state);

            match cmd_rx.recv_timeout(poll_interval) {
                Ok(IpcCommand::Shutdown) => return,
                Ok(IpcCommand::Send {
                    command_id,
                    action,
                    request,
                }) => {
                    send_ipc_request(
                        &target,
                        command_id,
                        action,
                        request,
                        &evt_tx,
                        &mut connection_state,
                    );
                }
                Ok(IpcCommand::DiscoverMonitors) => {
                    if target.hardware_discovery {
                        run_monitor_discovery(&evt_tx);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    });

    (cmd_tx, evt_rx)
}

fn run_monitor_discovery(evt_tx: &mpsc::SyncSender<IpcEvent>) {
    let started = Instant::now();
    let mut report = crate::discovery::discover();
    if report.has_incomplete_ddc_probe() {
        // ddcutil serializes access to I2C buses. A daemon capability refresh
        // can briefly hold that lock, so retry one incomplete observation
        // before exposing a partial set as importable configuration.
        std::thread::sleep(Duration::from_millis(250));
        let retry = crate::discovery::discover();
        let retry_is_better = retry.summary.viable_targets > report.summary.viable_targets
            || (retry.summary.viable_targets == report.summary.viable_targets
                && !retry.has_incomplete_ddc_probe());
        if retry_is_better {
            report = retry;
        }
    }
    tracing::info!(
        duration_ms = started.elapsed().as_millis() as u64,
        viable_targets = report.summary.viable_targets,
        incomplete_ddc_probe = report.has_incomplete_ddc_probe(),
        "tui_monitor_discovery_completed"
    );
    let _ = evt_tx.try_send(IpcEvent::MonitorDiscovery(Box::new(report)));
}

#[allow(clippy::too_many_lines)]
fn send_ipc_request(
    target: &WorkerTarget,
    command_id: u64,
    action: ActionKind,
    request: Request,
    evt_tx: &mpsc::SyncSender<IpcEvent>,
    connection_state: &mut DaemonConnection,
) {
    let start = Instant::now();
    tracing::info!(
        command_id,
        action = ?action,
        request = request.name(),
        "tui_ipc_request_dispatched"
    );

    let socket = match target.socket() {
        Ok(socket) => socket,
        Err(error) => {
            mark_disconnected(evt_tx, connection_state);
            tracing::warn!(
                command_id,
                action = ?action,
                error = %error,
                "tui_ipc_socket_unavailable"
            );
            let _ = evt_tx.try_send(IpcEvent::CommandFailed {
                command_id,
                action,
                error_category: ErrorCategory::DaemonUnavailable,
                message: String::from("daemon socket not found"),
            });
            return;
        }
    };

    match socket.send_request(&RequestEnvelope::new(request)) {
        Ok(response) => {
            mark_connected(evt_tx, connection_state);
            let duration_ms = start.elapsed().as_millis() as u64;

            let kind_name = response.kind_name();
            match response.response {
                Response::Ack { message } => {
                    tracing::info!(
                        command_id,
                        action = ?action,
                        duration_ms,
                        response = %message,
                        "tui_ipc_request_succeeded"
                    );
                    let _ = evt_tx.try_send(IpcEvent::CommandSucceeded {
                        command_id,
                        action,
                        message,
                    });
                    poll_daemon_status(target, evt_tx, connection_state);
                }
                Response::Error { code: _, message } => {
                    tracing::warn!(
                        command_id,
                        action = ?action,
                        duration_ms,
                        error = %message,
                        "tui_ipc_request_rejected"
                    );
                    let _ = evt_tx.try_send(IpcEvent::CommandFailed {
                        command_id,
                        action,
                        error_category: ErrorCategory::DaemonRejected,
                        message,
                    });
                }
                _ => {
                    tracing::warn!(
                        command_id,
                        action = ?action,
                        response_kind = kind_name,
                        "tui_ipc_unexpected_response"
                    );
                    let _ = evt_tx.try_send(IpcEvent::CommandFailed {
                        command_id,
                        action,
                        error_category: ErrorCategory::Transport,
                        message: format!("unexpected daemon response: {kind_name}"),
                    });
                }
            }
        }
        Err(IpcError::Unavailable { message, .. }) => {
            mark_disconnected(evt_tx, connection_state);
            tracing::warn!(
                command_id,
                action = ?action,
                error = %message,
                "tui_ipc_daemon_unavailable"
            );
            let _ = evt_tx.try_send(IpcEvent::CommandFailed {
                command_id,
                action,
                error_category: ErrorCategory::DaemonUnavailable,
                message: format!("daemon unavailable ({message})"),
            });
        }
        Err(IpcError::Io { source, .. })
            if source.kind() == std::io::ErrorKind::TimedOut
                || source.kind() == std::io::ErrorKind::WouldBlock =>
        {
            tracing::warn!(command_id, action = ?action, "tui_ipc_request_timed_out");
            let _ = evt_tx.try_send(IpcEvent::CommandFailed {
                command_id,
                action,
                error_category: ErrorCategory::Timeout,
                message: String::from("daemon did not respond (timeout)"),
            });
        }
        Err(error) => {
            mark_disconnected(evt_tx, connection_state);
            tracing::warn!(
                command_id,
                action = ?action,
                error = %error,
                "tui_ipc_transport_error"
            );
            let _ = evt_tx.try_send(IpcEvent::CommandFailed {
                command_id,
                action,
                error_category: ErrorCategory::Transport,
                message: error.to_string(),
            });
        }
    }
}

fn poll_daemon_status(
    target: &WorkerTarget,
    evt_tx: &mpsc::SyncSender<IpcEvent>,
    connection_state: &mut DaemonConnection,
) {
    match target.socket() {
        Ok(socket) => match socket.send_request(&RequestEnvelope::new(Request::Status)) {
            Ok(response) => {
                if let Response::Status { status } = response.response {
                    mark_connected(evt_tx, connection_state);
                    let _ = evt_tx.try_send(IpcEvent::Status(Box::new(status)));
                }
            }
            Err(_) => mark_disconnected(evt_tx, connection_state),
        },
        Err(_) => mark_disconnected(evt_tx, connection_state),
    }
}

/// Notifies the UI thread if the daemon connection transitions to connected.
///
/// - Complexity: O(1) time, O(1) space.
/// - Safety: Safe; non-blocking channel send.
/// - Allocations: 0 heap allocations.
fn mark_connected(evt_tx: &mpsc::SyncSender<IpcEvent>, state: &mut DaemonConnection) {
    if *state != DaemonConnection::Connected {
        let _ = evt_tx.try_send(IpcEvent::Connected);
        *state = DaemonConnection::Connected;
    }
}

/// Notifies the UI thread if the daemon connection transitions to disconnected.
///
/// - Complexity: O(1) time, O(1) space.
/// - Safety: Safe; non-blocking channel send.
/// - Allocations: 0 heap allocations.
fn mark_disconnected(evt_tx: &mpsc::SyncSender<IpcEvent>, state: &mut DaemonConnection) {
    if *state != DaemonConnection::Disconnected {
        let _ = evt_tx.try_send(IpcEvent::Disconnected);
        *state = DaemonConnection::Disconnected;
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::time::Duration;

    use super::{spawn_ipc_worker, IpcCommand, IpcEvent, WorkerTarget};
    use crate::ipc::ControlSocket;
    use crate::tui::model::MonitorWorkspaceState;
    use crate::tui::update::{self, Message};
    use crate::tui::Model;

    #[test]
    fn shows_daemon_unavailable_after_the_first_failed_status_poll() {
        let temp = tempfile::tempdir().expect("temporary directory should be created");
        let target = WorkerTarget {
            socket_override: Some(ControlSocket {
                path: temp.path().join("missing.sock"),
            }),
            hardware_discovery: false,
        };
        let (command_tx, event_rx) = spawn_ipc_worker(Duration::from_secs(30), target);

        let event = event_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("the initial failed poll should report a disconnected daemon");
        assert!(matches!(event, IpcEvent::Disconnected));

        let mut model = Model::new();
        update::update(&mut model, Message::Ipc(event));
        assert_eq!(
            model.monitor_workspace_state(),
            MonitorWorkspaceState::DaemonUnavailable
        );

        command_tx
            .send(IpcCommand::Shutdown)
            .expect("worker should accept shutdown");
    }
}
