use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::ipc::{ControlSocket, Request, RequestEnvelope, Response, StatusResponse};

pub(crate) enum IpcCommand {
    Send(Request),
    Shutdown,
}

pub(crate) enum IpcEvent {
    Status(Box<StatusResponse>),
    Disconnected,
    Connected,
}

pub(crate) fn spawn_ipc_worker(
    poll_interval: Duration,
) -> (mpsc::SyncSender<IpcCommand>, mpsc::Receiver<IpcEvent>) {
    let (cmd_tx, cmd_rx) = mpsc::sync_channel::<IpcCommand>(64);
    let (evt_tx, evt_rx) = mpsc::sync_channel::<IpcEvent>(64);

    thread::spawn(move || {
        let mut was_connected = false;

        loop {
            loop {
                match cmd_rx.try_recv() {
                    Ok(IpcCommand::Shutdown) => return,
                    Ok(IpcCommand::Send(request)) => send_ipc_request(request),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }

            poll_daemon_status(&evt_tx, &mut was_connected);

            match cmd_rx.recv_timeout(poll_interval) {
                Ok(IpcCommand::Shutdown) => return,
                Ok(IpcCommand::Send(request)) => send_ipc_request(request),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    });

    (cmd_tx, evt_rx)
}

fn send_ipc_request(request: Request) {
    if let Ok(socket) = ControlSocket::from_runtime() {
        let _ = socket.send_request(&RequestEnvelope::new(request));
    }
}

fn poll_daemon_status(evt_tx: &mpsc::SyncSender<IpcEvent>, was_connected: &mut bool) {
    match ControlSocket::from_runtime() {
        Ok(socket) => match socket.send_request(&RequestEnvelope::new(Request::Status)) {
            Ok(response) => {
                if let Response::Status { status } = response.response {
                    if !*was_connected {
                        let _ = evt_tx.try_send(IpcEvent::Connected);
                        *was_connected = true;
                    }
                    let _ = evt_tx.try_send(IpcEvent::Status(Box::new(status)));
                }
            }
            Err(_) => mark_disconnected(evt_tx, was_connected),
        },
        Err(_) => mark_disconnected(evt_tx, was_connected),
    }
}

fn mark_disconnected(evt_tx: &mpsc::SyncSender<IpcEvent>, was_connected: &mut bool) {
    if *was_connected {
        let _ = evt_tx.try_send(IpcEvent::Disconnected);
        *was_connected = false;
    }
}
