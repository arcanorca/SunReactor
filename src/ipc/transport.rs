use super::protocol::{IpcError, RequestEnvelope, ResponseEnvelope};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::thread;
use std::time::{Duration, Instant};

const SOCKET_IO_TIMEOUT: Duration = Duration::from_secs(15);
const SERVER_IO_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_IPC_MESSAGE_BYTES: usize = 64 * 1024;
const FRAME_READ_CHUNK_BYTES: usize = 4096;

pub(crate) trait IpcStream: Read + Write {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
}

#[cfg(target_os = "linux")]
impl IpcStream for std::os::unix::net::UnixStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.set_read_timeout(timeout)
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.set_write_timeout(timeout)
    }
}

pub(crate) fn read_request<S: IpcStream>(
    stream: &mut S,
    target: &str,
) -> Result<RequestEnvelope, IpcError> {
    read_json_message(stream, target, SERVER_IO_TIMEOUT)
}

pub(crate) fn write_response<S: IpcStream>(
    stream: &mut S,
    response: &ResponseEnvelope,
    target: &str,
) -> Result<(), IpcError> {
    write_json_message_with_timeout(stream, response, target, SERVER_IO_TIMEOUT)
}

pub(crate) fn read_response<S: IpcStream>(
    stream: &mut S,
    target: &str,
) -> Result<ResponseEnvelope, IpcError> {
    read_json_message(stream, target, SOCKET_IO_TIMEOUT)
}

fn read_json_message<T, S: IpcStream>(
    stream: &mut S,
    target: &str,
    timeout: Duration,
) -> Result<T, IpcError>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = read_bounded_message(stream, target, timeout)?;
    serde_json::from_slice(&bytes).map_err(|source| IpcError::Json { source })
}

pub(crate) fn write_json_message<T, S: IpcStream>(
    stream: &mut S,
    message: &T,
    target: &str,
) -> Result<(), IpcError>
where
    T: Serialize,
{
    write_json_message_with_timeout(stream, message, target, SOCKET_IO_TIMEOUT)
}

fn write_json_message_with_timeout<T, S: IpcStream>(
    stream: &mut S,
    message: &T,
    target: &str,
    timeout: Duration,
) -> Result<(), IpcError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(message).map_err(|source| IpcError::Json { source })?;
    if bytes.len() > MAX_IPC_MESSAGE_BYTES {
        return Err(IpcError::Protocol {
            message: format!(
                "payload exceeds maximum allowed size of {MAX_IPC_MESSAGE_BYTES} bytes"
            ),
        });
    }

    let mut frame = Vec::with_capacity(bytes.len() + 1);
    frame.extend_from_slice(&bytes);
    frame.push(b'\n');
    write_frame_with_deadline(stream, &frame, target, timeout)
}

pub(crate) fn configure_client_stream<S: IpcStream>(
    stream: &S,
    target: &str,
) -> Result<(), IpcError> {
    stream
        .set_read_timeout(Some(SOCKET_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            target: target.to_string(),
            source,
        })?;
    stream
        .set_write_timeout(Some(SOCKET_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            target: target.to_string(),
            source,
        })?;
    Ok(())
}

pub(crate) fn configure_server_stream<S: IpcStream>(
    stream: &S,
    target: &str,
) -> Result<(), IpcError> {
    stream
        .set_read_timeout(Some(SERVER_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            target: target.to_string(),
            source,
        })?;
    stream
        .set_write_timeout(Some(SERVER_IO_TIMEOUT))
        .map_err(|source| IpcError::Io {
            target: target.to_string(),
            source,
        })?;
    Ok(())
}

fn write_frame_with_deadline<S: IpcStream>(
    stream: &mut S,
    frame: &[u8],
    target: &str,
    timeout: Duration,
) -> Result<(), IpcError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    let mut offset = 0;

    while offset < frame.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(write_deadline_error(target, offset, frame.len()));
        }
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|source| IpcError::Io {
                target: target.to_string(),
                source,
            })?;

        match stream.write(&frame[offset..]) {
            Ok(0) => {
                return Err(IpcError::Io {
                    target: target.to_string(),
                    source: io::Error::new(
                        io::ErrorKind::WriteZero,
                        "IPC frame write returned zero bytes",
                    ),
                });
            }
            Ok(written) => offset += written,
            Err(source) if source.kind() == io::ErrorKind::Interrupted => {}
            Err(source) if source.kind() == io::ErrorKind::WouldBlock => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(write_deadline_error(target, offset, frame.len()));
                }
                thread::sleep(remaining.min(Duration::from_millis(1)));
            }
            Err(source) if source.kind() == io::ErrorKind::TimedOut => {
                return Err(write_deadline_error(target, offset, frame.len()));
            }
            Err(source) => {
                return Err(IpcError::Io {
                    target: target.to_string(),
                    source,
                });
            }
        }
    }

    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(write_deadline_error(target, offset, frame.len()));
    }
    stream
        .set_write_timeout(Some(remaining))
        .map_err(|source| IpcError::Io {
            target: target.to_string(),
            source,
        })?;
    match stream.flush() {
        Ok(()) => Ok(()),
        Err(source)
            if matches!(
                source.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ) =>
        {
            Err(write_deadline_error(target, offset, frame.len()))
        }
        Err(source) => Err(IpcError::Io {
            target: target.to_string(),
            source,
        }),
    }
}

fn write_deadline_error(target: &str, offset: usize, frame_len: usize) -> IpcError {
    IpcError::Io {
        target: target.to_string(),
        source: io::Error::new(
            io::ErrorKind::TimedOut,
            format!("IPC frame write deadline exceeded after {offset} of {frame_len} bytes"),
        ),
    }
}

fn read_bounded_message<S: IpcStream>(
    stream: &mut S,
    target: &str,
    timeout: Duration,
) -> Result<Vec<u8>, IpcError> {
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; FRAME_READ_CHUNK_BYTES];

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(IpcError::Protocol {
                message: String::from("IPC frame receive deadline exceeded"),
            });
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|source| IpcError::Io {
                target: target.to_string(),
                source,
            })?;

        let read = match stream.read(&mut chunk) {
            Ok(0) => {
                return Err(IpcError::Protocol {
                    message: String::from("unterminated IPC frame: expected LF delimiter"),
                });
            }
            Ok(read) => read,
            Err(source)
                if matches!(
                    source.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(IpcError::Protocol {
                    message: String::from("IPC frame receive deadline exceeded"),
                });
            }
            Err(source) => {
                return Err(IpcError::Io {
                    target: target.to_string(),
                    source,
                });
            }
        };

        let Some(delimiter) = chunk[..read].iter().position(|byte| *byte == b'\n') else {
            bytes.extend_from_slice(&chunk[..read]);
            if bytes.len() > MAX_IPC_MESSAGE_BYTES {
                return Err(IpcError::Protocol {
                    message: format!(
                        "payload exceeds maximum allowed size of {MAX_IPC_MESSAGE_BYTES} bytes"
                    ),
                });
            }
            continue;
        };

        bytes.extend_from_slice(&chunk[..delimiter]);
        if bytes.len() > MAX_IPC_MESSAGE_BYTES {
            return Err(IpcError::Protocol {
                message: format!(
                    "payload exceeds maximum allowed size of {MAX_IPC_MESSAGE_BYTES} bytes"
                ),
            });
        }
        return Ok(bytes);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{
        read_bounded_message, read_request, read_response, write_frame_with_deadline,
        write_json_message, write_response, MAX_IPC_MESSAGE_BYTES,
    };
    use crate::ipc::protocol::{Request, RequestEnvelope, ResponseEnvelope};
    use crate::ipc::socket::BoundControlSocket;
    use std::fs;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use crate::ipc::socket::ControlSocket;

    #[test]
    fn client_can_round_trip_request_and_response() {
        let temp = TempDir::new();
        let socket = ControlSocket {
            path: temp.path().join("run/control.sock"),
        };
        let listener = socket.bind_listener().expect("listener should bind");

        let server = thread::spawn(move || serve_ping(&listener));
        let response = socket
            .send_request(&RequestEnvelope::new(Request::Ping))
            .expect("client request should succeed");
        server.join().expect("server thread should finish");

        assert_eq!(response.kind_name(), "pong");
    }

    #[test]
    fn request_frame_does_not_require_client_eof() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        write_json_message(
            &mut writer,
            &RequestEnvelope::new(Request::Ping),
            "/tmp/control.sock",
        )
        .expect("request should encode");
        let started = Instant::now();
        let request = read_request(&mut reader, "/tmp/control.sock")
            .expect("LF should complete request without EOF");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            request.validate().expect("request should validate"),
            Request::Ping
        );
    }

    #[test]
    fn response_frame_does_not_require_server_eof() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        write_response(&mut writer, &ResponseEnvelope::pong(), "/tmp/control.sock")
            .expect("response should encode");
        let started = Instant::now();
        let response = read_response(&mut reader, "/tmp/control.sock")
            .expect("LF should complete response without EOF");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(response.kind_name(), "pong");
    }

    #[test]
    fn embedded_newline_is_json_escaped_and_not_a_frame_delimiter() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let request = RequestEnvelope::new(Request::SetOverride {
            monitor_id: Some(String::from("desk\nmonitor")),
            percent: 42,
            minutes: None,
        });
        write_json_message(&mut writer, &request, "/tmp/control.sock")
            .expect("request should encode");
        let decoded = read_request(&mut reader, "/tmp/control.sock")
            .expect("escaped newline should remain in one frame")
            .validate()
            .expect("request should validate");
        assert_eq!(decoded, request.request);
    }

    #[test]
    fn empty_frame_is_rejected() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        writer.write_all(b"\n").expect("frame should write");
        let error =
            read_request(&mut reader, "/tmp/control.sock").expect_err("empty frame must fail");
        assert!(matches!(error, super::IpcError::Json { .. }));
    }

    #[test]
    fn malformed_json_frame_is_rejected() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        writer
            .write_all(b"{\"request\":\n")
            .expect("frame should write");
        let error =
            read_request(&mut reader, "/tmp/control.sock").expect_err("malformed JSON must fail");
        assert!(matches!(error, super::IpcError::Json { .. }));
    }

    #[test]
    fn exact_payload_limit_excludes_delimiter() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        writer
            .write_all(&vec![b'x'; MAX_IPC_MESSAGE_BYTES])
            .expect("boundary payload should write");
        writer.write_all(b"\n").expect("delimiter should write");
        let payload =
            read_bounded_message(&mut reader, "/tmp/control.sock", Duration::from_secs(1))
                .expect("exact payload boundary should be accepted");
        assert_eq!(payload.len(), MAX_IPC_MESSAGE_BYTES);
    }

    #[test]
    fn payload_limit_is_enforced_with_delimiter_in_same_read() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        writer
            .write_all(&vec![b'x'; MAX_IPC_MESSAGE_BYTES + 1])
            .expect("oversize payload should write");
        writer.write_all(b"\n").expect("delimiter should write");
        let error = read_bounded_message(&mut reader, "/tmp/control.sock", Duration::from_secs(1))
            .expect_err("payload over the limit must fail");
        assert!(error.to_string().contains("exceeds"));
    }

    #[test]
    fn eof_before_delimiter_is_rejected() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        writer
            .write_all(br#"{"version":1}"#)
            .expect("partial frame should write");
        writer
            .shutdown(std::net::Shutdown::Write)
            .expect("writer should close");
        let error =
            read_request(&mut reader, "/tmp/control.sock").expect_err("EOF before LF must fail");
        assert!(error.to_string().contains("unterminated"));
    }

    #[test]
    fn oversize_frame_is_rejected_before_delimiter() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        writer
            .write_all(&vec![b'x'; MAX_IPC_MESSAGE_BYTES + 1])
            .expect("oversize frame should write");
        let error = read_request(&mut reader, "/tmp/control.sock")
            .expect_err("oversize frame must fail immediately");
        assert!(error.to_string().contains("exceeds"));
    }

    #[test]
    fn normal_frame_write_completes_and_decodes() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        write_json_message(
            &mut writer,
            &RequestEnvelope::new(Request::Ping),
            "/tmp/control.sock",
        )
        .expect("bounded frame should write");
        assert_eq!(
            read_request(&mut reader, "/tmp/control.sock")
                .expect("frame should decode")
                .validate()
                .expect("request should validate"),
            Request::Ping
        );
    }

    #[test]
    fn exact_max_payload_frame_writes_successfully() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let payload = vec![b'a'; MAX_IPC_MESSAGE_BYTES];
        let reader_thread = thread::spawn(move || {
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .expect("peer should read frame");
            bytes
        });
        let mut frame = payload;
        frame.push(b'\n');
        write_frame_with_deadline(
            &mut writer,
            &frame,
            "/tmp/control.sock",
            Duration::from_secs(1),
        )
        .expect("max-size payload plus delimiter should be writable");
        drop(writer);
        let bytes = reader_thread.join().expect("reader should finish");
        assert_eq!(bytes.len(), MAX_IPC_MESSAGE_BYTES + 1);
    }

    #[test]
    fn legal_maximum_near_frame_writes_through_production_writer() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let request = RequestEnvelope::new(Request::SetOverride {
            monitor_id: Some("x".repeat(65_000)),
            percent: 42,
            minutes: None,
        });
        let encoded = serde_json::to_vec(&request).expect("request should encode");
        assert!(encoded.len() <= MAX_IPC_MESSAGE_BYTES);
        let reader_thread = thread::spawn(move || {
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .expect("peer should read frame");
            bytes
        });
        write_json_message(&mut writer, &request, "/tmp/control.sock")
            .expect("legal near-maximum payload should write");
        drop(writer);
        let frame = reader_thread.join().expect("reader should finish");
        assert_eq!(frame.len(), encoded.len() + 1);
        assert_eq!(frame[encoded.len()], b'\n');
    }

    #[test]
    fn write_deadline_is_absolute_when_peer_reads_slowly() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        set_send_buffer_size(&writer, 1024);
        reader
            .set_read_timeout(Some(Duration::from_millis(10)))
            .expect("reader timeout should configure");
        let frame = vec![b'x'; 2 * 1024 * 1024];
        let stop_reader = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_reader_thread = std::sync::Arc::clone(&stop_reader);
        let reader_thread = thread::spawn(move || {
            let mut byte = [0_u8; 1];
            loop {
                match reader.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => {
                        if stop_reader_thread.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) && stop_reader_thread.load(std::sync::atomic::Ordering::Relaxed) =>
                    {
                        break;
                    }
                    Err(_) => break,
                }
            }
        });
        let started = Instant::now();
        let error = write_frame_with_deadline(
            &mut writer,
            &frame,
            "/tmp/control.sock",
            Duration::from_millis(100),
        )
        .expect_err("slow peer should exhaust the total write deadline");
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(error.to_string().contains("write deadline"));
        stop_reader.store(true, std::sync::atomic::Ordering::Relaxed);
        drop(writer);
        reader_thread.join().expect("reader should finish");
    }

    #[test]
    fn write_deadline_fails_when_peer_never_reads() {
        let (mut writer, _reader) = UnixStream::pair().expect("stream pair should work");
        set_send_buffer_size(&writer, 1024);
        let frame = vec![b'x'; 2 * 1024 * 1024];
        let started = Instant::now();
        let error = write_frame_with_deadline(
            &mut writer,
            &frame,
            "/tmp/control.sock",
            Duration::from_millis(100),
        )
        .expect_err("non-reading peer should exhaust the write deadline");
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(error.to_string().contains("write deadline"));
    }

    #[cfg(target_os = "linux")]
    fn set_send_buffer_size(stream: &UnixStream, size: i32) {
        let result = unsafe {
            libc::setsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                (&raw const size).cast(),
                std::mem::size_of::<i32>() as libc::socklen_t,
            )
        };
        assert_eq!(result, 0, "setsockopt(SO_SNDBUF) should succeed");
    }

    #[cfg(not(target_os = "linux"))]
    fn set_send_buffer_size(_stream: &UnixStream, _size: i32) {}

    #[test]
    fn slow_drip_cannot_extend_the_absolute_frame_deadline() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let writer_thread = thread::spawn(move || {
            for _ in 0..20 {
                if writer.write_all(b"x").is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        let started = Instant::now();
        let error = read_request(&mut reader, "/tmp/control.sock")
            .expect_err("slow drip without LF must hit the absolute deadline");
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(error.to_string().contains("deadline"));
        writer_thread.join().expect("writer thread should finish");
    }

    #[test]
    fn second_frame_is_not_processed_as_a_second_request() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        write_json_message(
            &mut writer,
            &RequestEnvelope::new(Request::Ping),
            "/tmp/control.sock",
        )
        .expect("first frame should write");
        write_json_message(
            &mut writer,
            &RequestEnvelope::new(Request::Status),
            "/tmp/control.sock",
        )
        .expect("second frame should write");

        let first = read_request(&mut reader, "/tmp/control.sock")
            .expect("first frame should decode")
            .validate()
            .expect("first request should validate");
        assert_eq!(first, Request::Ping);
    }

    #[test]
    fn writer_rejects_oversize_payload_before_writing() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let oversized = RequestEnvelope::new(Request::SetOverride {
            monitor_id: Some("x".repeat(MAX_IPC_MESSAGE_BYTES)),
            percent: 42,
            minutes: None,
        });
        let error = write_json_message(&mut writer, &oversized, "/tmp/control.sock")
            .expect_err("oversize payload must be rejected");
        assert!(error.to_string().contains("exceeds"));
        reader
            .set_read_timeout(Some(Duration::from_millis(20)))
            .expect("timeout should configure");
        let mut byte = [0_u8; 1];
        assert!(
            matches!(reader.read(&mut byte), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock || error.kind() == std::io::ErrorKind::TimedOut)
        );
    }

    #[test]
    fn oversized_request_is_rejected_without_unbounded_read() {
        let (mut writer, mut reader) = UnixStream::pair().expect("stream pair should work");
        let oversized = vec![b'x'; MAX_IPC_MESSAGE_BYTES + 1];

        let server = thread::spawn(move || {
            let error = read_request(&mut reader, "/tmp/control.sock")
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

    fn serve_ping(listener: &BoundControlSocket) {
        for _ in 0..20 {
            match listener.accept().expect("accept should work") {
                Some(mut stream) => {
                    let request = read_request(&mut stream, "/tmp/control.sock")
                        .expect("request should decode")
                        .validate()
                        .expect("request should validate");
                    assert_eq!(request, Request::Ping);
                    write_response(&mut stream, &ResponseEnvelope::pong(), "/tmp/control.sock")
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
