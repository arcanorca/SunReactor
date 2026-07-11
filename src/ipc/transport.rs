use super::protocol::{IpcError, RequestEnvelope, ResponseEnvelope};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

const SOCKET_IO_TIMEOUT: Duration = Duration::from_secs(15);
const SERVER_IO_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_IPC_MESSAGE_BYTES: usize = 64 * 1024;

pub(crate) fn read_request(
    stream: &mut UnixStream,
    path: &Path,
) -> Result<RequestEnvelope, IpcError> {
    read_json_message(stream, path)
}

pub(crate) fn write_response(
    stream: &mut UnixStream,
    response: &ResponseEnvelope,
    path: &Path,
) -> Result<(), IpcError> {
    write_json_message(stream, response, path)
}

pub(crate) fn read_response(
    stream: &mut UnixStream,
    path: &Path,
) -> Result<ResponseEnvelope, IpcError> {
    read_json_message(stream, path)
}

fn read_json_message<T>(stream: &mut UnixStream, path: &Path) -> Result<T, IpcError>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = read_bounded_message(stream, path)?;
    serde_json::from_slice(&bytes).map_err(|source| IpcError::Json { source })
}

pub(crate) fn write_json_message<T>(
    stream: &mut UnixStream,
    message: &T,
    path: &Path,
) -> Result<(), IpcError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(message).map_err(|source| IpcError::Json { source })?;
    stream.write_all(&bytes).map_err(|source| IpcError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    stream.write_all(b"\n").map_err(|source| IpcError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    stream.flush().map_err(|source| IpcError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub(crate) fn configure_client_stream(stream: &UnixStream, path: &Path) -> Result<(), IpcError> {
    stream
        .set_read_timeout(Some(SOCKET_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    stream
        .set_write_timeout(Some(SOCKET_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(())
}

pub(crate) fn configure_server_stream(stream: &UnixStream, path: &Path) -> Result<(), IpcError> {
    stream
        .set_read_timeout(Some(SERVER_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    stream
        .set_write_timeout(Some(SERVER_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(())
}

fn read_bounded_message(stream: &mut UnixStream, path: &Path) -> Result<Vec<u8>, IpcError> {
    let mut bytes = Vec::new();
    stream
        .take((MAX_IPC_MESSAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| IpcError::Io {
            path: path.to_path_buf(),
            source,
        })?;

    if bytes.len() > MAX_IPC_MESSAGE_BYTES {
        return Err(IpcError::Protocol {
            message: format!(
                "payload exceeds maximum allowed size of {} bytes",
                MAX_IPC_MESSAGE_BYTES
            ),
        });
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{read_request, write_response, MAX_IPC_MESSAGE_BYTES};
    use crate::ipc::protocol::{Request, RequestEnvelope, ResponseEnvelope};
    use crate::ipc::socket::BoundControlSocket;
    use std::fs;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::ipc::socket::ControlSocket;

    #[test]
    fn client_can_round_trip_request_and_response() {
        let temp = TempDir::new();
        let socket = ControlSocket {
            path: temp.path().join("run/control.sock"),
        };
        let listener = socket.bind_listener().expect("listener should bind");

        let server = thread::spawn(move || serve_ping(listener));
        let response = socket
            .send_request(&RequestEnvelope::new(Request::Ping))
            .expect("client request should succeed");
        server.join().expect("server thread should finish");

        assert_eq!(response.kind_name(), "pong");
    }

    #[test]
    #[test]
    fn oversized_request_is_rejected_without_unbounded_read() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let oversized = vec![b'x'; MAX_IPC_MESSAGE_BYTES + 1];

        let server = thread::spawn(move || {
            let error = read_request(&mut reader, Path::new("/tmp/control.sock"))
                .expect_err("oversized request must fail");
            assert!(matches!(error, super::IpcError::Protocol { .. }));
            assert!(error.to_string().contains("exceeds"));
        });

        writer
            .write_all(&oversized)
            .expect("oversized payload should write");
        writer
            .shutdown(std::net::Shutdown::Write)
            .expect("writer should close");
        server.join().expect("server thread should finish");
    }

    fn serve_ping(listener: BoundControlSocket) {
        for _ in 0..20 {
            match listener.accept().expect("accept should work") {
                Some(mut stream) => {
                    let request = read_request(&mut stream, Path::new("/tmp/control.sock"))
                        .expect("request should decode")
                        .validate()
                        .expect("request should validate");
                    assert_eq!(request, Request::Ping);
                    write_response(
                        &mut stream,
                        &ResponseEnvelope::pong(),
                        Path::new("/tmp/control.sock"),
                    )
                    .expect("response should encode");
                    return;
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        }

        panic!("timed out waiting for client connection");
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should work")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("sunreactor-ipc-test-{unique}"));
            fs::create_dir_all(&path).expect("temp dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).ok();
        }
    }
}
