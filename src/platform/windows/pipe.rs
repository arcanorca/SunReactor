use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_BROKEN_PIPE,
    ERROR_FILE_NOT_FOUND, ERROR_IO_PENDING, ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
    GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE,
    FILE_FLAG_OVERLAPPED, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, WaitNamedPipeW, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject};
use windows_sys::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};

use super::security::{
    create_pipe_security_attributes, get_current_user_sid, hash_sid, AutoHandle,
};
use crate::ipc::protocol::{IpcError, RequestEnvelope, ResponseEnvelope};
use crate::ipc::transport::{
    configure_client_stream, configure_server_stream, read_response, write_json_message,
};
use crate::paths::{IpcEndpoint, PathError};

const PIPE_PREFIX: &str = r"\\.\pipe\SunReactor\";
const MAX_IPC_MESSAGE_BYTES: usize = 64 * 1024;
const CLIENT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

fn to_wide_null(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// Derives the canonical per-user Named Pipe path.
///
/// Format: `\\.\pipe\SunReactor\control-{sid_hash}`
pub fn default_windows_pipe_name() -> Result<String, PathError> {
    if let Some(custom) = std::env::var_os("SUNREACTOR_TEST_PIPE_NAME") {
        if !custom.is_empty() {
            let name = custom.to_string_lossy().to_string();
            if name.starts_with(r"\\.\pipe\") {
                return Ok(name);
            }
            return Ok(format!("{PIPE_PREFIX}{name}"));
        }
    }

    let sid = get_current_user_sid().map_err(PathError::WindowsSecurity)?;
    let hash = hash_sid(&sid);
    Ok(format!("{PIPE_PREFIX}control-{hash}"))
}

#[derive(Debug, Clone)]
pub struct ControlSocket {
    pub endpoint: IpcEndpoint,
}

impl ControlSocket {
    pub fn from_runtime() -> Result<Self, PathError> {
        let name = default_windows_pipe_name()?;
        Ok(Self {
            endpoint: IpcEndpoint::NamedPipe(name),
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> IpcEndpoint {
        self.endpoint.clone()
    }

    #[must_use]
    pub fn display_target(&self) -> String {
        self.endpoint.display_target()
    }

    pub fn bind_listener(&self) -> Result<BoundControlSocket, IpcError> {
        let pipe_name = self.display_target();
        let wide_name = to_wide_null(&pipe_name);

        let sid = get_current_user_sid().map_err(|e| IpcError::Protocol {
            message: format!("failed to determine user SID for pipe DACL: {e}"),
        })?;

        let (mut sa, _sd_guard) =
            create_pipe_security_attributes(&sid).map_err(|e| IpcError::Protocol {
                message: format!("failed to construct secure DACL: {e}"),
            })?;

        let open_mode = PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED;
        let pipe_mode =
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;

        let handle = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                open_mode,
                pipe_mode,
                1, // Only 1 instance permitted: enforces single-instance server
                MAX_IPC_MESSAGE_BYTES as u32,
                MAX_IPC_MESSAGE_BYTES as u32,
                0,
                (&raw mut sa).cast_const(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let err = unsafe { GetLastError() };
            if err == ERROR_ACCESS_DENIED || err == ERROR_ALREADY_EXISTS || err == ERROR_PIPE_BUSY {
                return Err(IpcError::SocketInUse { target: pipe_name });
            }
            return Err(IpcError::Io {
                target: pipe_name,
                source: io::Error::from_raw_os_error(err as i32),
            });
        }

        let event_handle = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        if event_handle.is_null() || event_handle == INVALID_HANDLE_VALUE {
            let err = unsafe { GetLastError() };
            unsafe { CloseHandle(handle) };
            return Err(IpcError::Io {
                target: pipe_name,
                source: io::Error::from_raw_os_error(err as i32),
            });
        }

        Ok(BoundControlSocket {
            pipe_handle: AutoHandle(handle),
            event_handle: AutoHandle(event_handle),
            endpoint: self.endpoint.clone(),
            connected: AtomicBool::new(false),
            connect_pending: AtomicBool::new(false),
            overlapped: Mutex::new(unsafe { std::mem::zeroed() }),
        })
    }

    pub fn send_request(&self, request: &RequestEnvelope) -> Result<ResponseEnvelope, IpcError> {
        let pipe_name = self.display_target();
        let wide_name = to_wide_null(&pipe_name);
        let deadline = Instant::now() + CLIENT_CONNECT_TIMEOUT;

        let mut handle = INVALID_HANDLE_VALUE;
        while Instant::now() < deadline {
            handle = unsafe {
                CreateFileW(
                    wide_name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };

            if handle != INVALID_HANDLE_VALUE {
                break;
            }

            let err = unsafe { GetLastError() };
            if err == ERROR_FILE_NOT_FOUND {
                return Err(IpcError::Unavailable {
                    target: pipe_name,
                    message: String::from("SunReactor daemon is not running"),
                });
            }

            if err == ERROR_PIPE_BUSY {
                let remaining_ms = deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .min(250) as u32;
                unsafe { WaitNamedPipeW(wide_name.as_ptr(), remaining_ms) };
                continue;
            }

            return Err(IpcError::Io {
                target: pipe_name,
                source: io::Error::from_raw_os_error(err as i32),
            });
        }

        if handle == INVALID_HANDLE_VALUE {
            return Err(IpcError::Unavailable {
                target: pipe_name,
                message: String::from("connection timed out waiting for server pipe instance"),
            });
        }

        let mut stream = NamedPipeStream::new_client(handle, pipe_name.clone());
        configure_client_stream(&stream, &pipe_name)?;
        write_json_message(&mut stream, request, &pipe_name)?;
        read_response(&mut stream, &pipe_name)?.validate()
    }
}

pub struct BoundControlSocket {
    pipe_handle: AutoHandle,
    event_handle: AutoHandle,
    endpoint: IpcEndpoint,
    connected: AtomicBool,
    connect_pending: AtomicBool,
    overlapped: Mutex<OVERLAPPED>,
}

impl std::fmt::Debug for BoundControlSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundControlSocket")
            .field("endpoint", &self.endpoint)
            .field("connected", &self.connected)
            .finish_non_exhaustive()
    }
}

unsafe impl Send for BoundControlSocket {}
unsafe impl Sync for BoundControlSocket {}

impl BoundControlSocket {
    #[must_use]
    pub fn endpoint(&self) -> IpcEndpoint {
        self.endpoint.clone()
    }

    #[must_use]
    pub fn display_target(&self) -> String {
        self.endpoint.display_target()
    }

    /// Checks for a pending client connection without blocking.
    ///
    /// If a client is connected, returns an `IpcStream` wrapper.
    pub fn accept(&self) -> Result<Option<NamedPipeStream>, IpcError> {
        let pipe_name = self.display_target();

        // If a previous connection was active, disconnect it first so the pipe can accept again.
        if self.connected.load(Ordering::SeqCst) {
            unsafe {
                DisconnectNamedPipe(self.pipe_handle.0);
                ResetEvent(self.event_handle.0);
            }
            self.connected.store(false, Ordering::SeqCst);
            self.connect_pending.store(false, Ordering::SeqCst);
        }

        let mut ov = self.overlapped.lock().unwrap();

        if !self.connect_pending.load(Ordering::SeqCst) {
            unsafe {
                ov.hEvent = self.event_handle.0;
                let ok = ConnectNamedPipe(self.pipe_handle.0, &raw mut *ov);
                if ok != 0 {
                    self.connected.store(true, Ordering::SeqCst);
                    let stream = NamedPipeStream::new_server(self.pipe_handle.0, pipe_name.clone());
                    let _ = configure_server_stream(&stream, &pipe_name);
                    return Ok(Some(stream));
                }

                let err = GetLastError();
                if err == ERROR_PIPE_CONNECTED {
                    self.connected.store(true, Ordering::SeqCst);
                    let stream = NamedPipeStream::new_server(self.pipe_handle.0, pipe_name.clone());
                    let _ = configure_server_stream(&stream, &pipe_name);
                    return Ok(Some(stream));
                }

                if err == ERROR_IO_PENDING {
                    self.connect_pending.store(true, Ordering::SeqCst);
                } else {
                    return Err(IpcError::Io {
                        target: pipe_name,
                        source: io::Error::from_raw_os_error(err as i32),
                    });
                }
            }
        }

        // Check if overlapped connection has completed (poll with timeout 0)
        let wait_res = unsafe { WaitForSingleObject(self.event_handle.0, 0) };
        if wait_res == WAIT_OBJECT_0 {
            let mut transferred = 0;
            let ok = unsafe {
                GetOverlappedResult(
                    self.pipe_handle.0,
                    (&raw const *ov).cast_mut(),
                    &raw mut transferred,
                    0,
                )
            };
            if ok != 0 || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED {
                self.connected.store(true, Ordering::SeqCst);
                self.connect_pending.store(false, Ordering::SeqCst);
                let stream = NamedPipeStream::new_server(self.pipe_handle.0, pipe_name.clone());
                let _ = configure_server_stream(&stream, &pipe_name);
                return Ok(Some(stream));
            }
        }

        Ok(None)
    }

    /// Bounded wait for incoming IPC connection.
    pub fn wait_for_ipc_or_sleep(&self, duration: Duration) {
        if self.connected.load(Ordering::SeqCst) {
            return;
        }

        let mut ov = self.overlapped.lock().unwrap();
        if !self.connect_pending.load(Ordering::SeqCst) {
            unsafe {
                ov.hEvent = self.event_handle.0;
                let ok = ConnectNamedPipe(self.pipe_handle.0, &raw mut *ov);
                if ok != 0 || GetLastError() == ERROR_PIPE_CONNECTED {
                    self.connected.store(true, Ordering::SeqCst);
                    return;
                }
                if GetLastError() == ERROR_IO_PENDING {
                    self.connect_pending.store(true, Ordering::SeqCst);
                }
            }
        }

        let timeout_ms = duration.as_millis().min(u128::from(u32::MAX)) as u32;
        unsafe {
            WaitForSingleObject(self.event_handle.0, timeout_ms);
        }
    }
}

impl Drop for BoundControlSocket {
    fn drop(&mut self) {
        if self.connect_pending.load(Ordering::SeqCst) {
            unsafe {
                CancelIo(self.pipe_handle.0);
            }
        }
        if self.connected.load(Ordering::SeqCst) {
            unsafe {
                DisconnectNamedPipe(self.pipe_handle.0);
            }
        }
    }
}

pub struct NamedPipeStream {
    handle: HANDLE,
    owned: bool,
    #[allow(dead_code)]
    target: String,
}

unsafe impl Send for NamedPipeStream {}
unsafe impl Sync for NamedPipeStream {}

impl NamedPipeStream {
    #[must_use]
    pub fn new_client(handle: HANDLE, target: String) -> Self {
        Self {
            handle,
            owned: true,
            target,
        }
    }

    #[must_use]
    pub fn new_server(handle: HANDLE, target: String) -> Self {
        Self {
            handle,
            owned: false,
            target,
        }
    }
}

impl Drop for NamedPipeStream {
    fn drop(&mut self) {
        if self.owned && !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

impl crate::ipc::transport::IpcStream for NamedPipeStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        let _ = timeout;
        Ok(())
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        let _ = timeout;
        Ok(())
    }
}

impl Read for NamedPipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut bytes_read: u32 = 0;
        let ok = unsafe {
            ReadFile(
                self.handle,
                buf.as_mut_ptr().cast(),
                buf.len() as u32,
                &raw mut bytes_read,
                std::ptr::null_mut(),
            )
        };

        if ok != 0 {
            return Ok(bytes_read as usize);
        }

        let err = unsafe { GetLastError() };
        if err == ERROR_BROKEN_PIPE || err == ERROR_NO_DATA {
            // Standard EOF signal on Windows named pipes
            return Ok(0);
        }

        Err(io::Error::from_raw_os_error(err as i32))
    }
}

impl Write for NamedPipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes_written: u32 = 0;
        let ok = unsafe {
            WriteFile(
                self.handle,
                buf.as_ptr().cast(),
                buf.len() as u32,
                &raw mut bytes_written,
                std::ptr::null_mut(),
            )
        };

        if ok != 0 {
            return Ok(bytes_written as usize);
        }

        let err = unsafe { GetLastError() };
        Err(io::Error::from_raw_os_error(err as i32))
    }

    fn flush(&mut self) -> io::Result<()> {
        let ok = unsafe { FlushFileBuffers(self.handle) };
        if ok != 0 {
            Ok(())
        } else {
            let err = unsafe { GetLastError() };
            if err == ERROR_BROKEN_PIPE || err == ERROR_NO_DATA {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(err as i32))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::{Request, RequestEnvelope, ResponseEnvelope};

    #[test]
    fn single_instance_rejection_on_duplicate_bind() {
        let pipe_name = format!(
            r"\\.\pipe\SunReactor\test-single-inst-{}",
            std::process::id()
        );
        let socket1 = ControlSocket {
            endpoint: IpcEndpoint::NamedPipe(pipe_name.clone()),
        };
        let listener1 = socket1.bind_listener().expect("bind listener 1");

        let socket2 = ControlSocket {
            endpoint: IpcEndpoint::NamedPipe(pipe_name.clone()),
        };
        let err2 = socket2.bind_listener().expect_err("second bind must fail");

        match err2 {
            IpcError::SocketInUse { target } => {
                assert!(target.contains("test-single-inst"));
            }
            other => panic!("expected SocketInUse, got {other:?}"),
        }

        drop(listener1);
    }

    #[test]
    fn pipe_roundtrip_request_and_response() {
        let pipe_name = format!(r"\\.\pipe\SunReactor\test-roundtrip-{}", std::process::id());
        let socket = ControlSocket {
            endpoint: IpcEndpoint::NamedPipe(pipe_name.clone()),
        };
        let listener = socket.bind_listener().expect("bind listener");

        let target = listener.display_target();
        let server_thread = std::thread::spawn(move || {
            let start = Instant::now();
            loop {
                if let Ok(Some(mut stream)) = listener.accept() {
                    let req = crate::ipc::transport::read_request(&mut stream, &target)
                        .expect("read request");
                    assert_eq!(req.request, Request::Ping);
                    let resp = ResponseEnvelope::pong();
                    crate::ipc::transport::write_response(&mut stream, &resp, &target)
                        .expect("write response");
                    break;
                }
                if start.elapsed() > Duration::from_secs(5) {
                    panic!("server timed out waiting for client");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });

        let client_socket = ControlSocket {
            endpoint: IpcEndpoint::NamedPipe(pipe_name),
        };
        let resp = client_socket
            .send_request(&RequestEnvelope::new(Request::Ping))
            .expect("client ping");

        assert_eq!(resp, ResponseEnvelope::pong());
        server_thread.join().expect("server thread finish");
    }
}
