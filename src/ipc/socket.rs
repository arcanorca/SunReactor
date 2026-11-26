#![cfg(target_os = "linux")]

use super::protocol::{IpcError, RequestEnvelope, ResponseEnvelope};
use super::transport::{
    configure_client_stream, configure_server_stream, read_response, write_json_message,
};
use crate::paths::{self, PathError};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

const SOCKET_DIR_MODE: u32 = 0o700;
const SOCKET_FILE_MODE: u32 = 0o600;
const MAX_UNIX_SOCKET_PATH_LEN: usize = 107;

#[derive(Debug, Clone, Default)]
pub struct ControlSocket {
    pub path: PathBuf,
}

impl ControlSocket {
    pub fn from_runtime() -> Result<Self, PathError> {
        Ok(Self {
            path: paths::runtime_socket_path()?,
        })
    }

    #[must_use]
    pub fn display_target(&self) -> String {
        self.path.display().to_string()
    }

    /// Takes the exclusive single-instance lock (`<socket>.lock`, `flock`) for
    /// the daemon's lifetime. The kernel drops it with the process, so a crash
    /// never leaves a stale lock. Fails with [`IpcError::SocketInUse`] while
    /// another daemon holds it.
    pub fn acquire_instance_lock(&self) -> Result<InstanceLock, IpcError> {
        validate_socket_path_length(&self.path)?;
        let lock_path = instance_lock_path(&self.path);
        if let Some(parent) = lock_path.parent() {
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(SOCKET_DIR_MODE);
            builder.create(parent).map_err(|source| IpcError::Io {
                target: parent.display().to_string(),
                source,
            })?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(SOCKET_FILE_MODE)
            .open(&lock_path)
            .map_err(|source| IpcError::Io {
                target: lock_path.display().to_string(),
                source,
            })?;
        // A just-released lock can still look held for a moment: a previous
        // daemon may be finishing its exit, or (in-process) another thread's
        // fork has briefly inherited the descriptor before its exec closes it.
        // Retry for a short, bounded grace before declaring a live instance.
        let deadline = std::time::Instant::now() + INSTANCE_LOCK_GRACE;
        loop {
            match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return Ok(InstanceLock { _file: file }),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Err(IpcError::SocketInUse {
                            target: self.path.display().to_string(),
                        });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                Err(error) => {
                    return Err(IpcError::Io {
                        target: lock_path.display().to_string(),
                        source: io::Error::from(error),
                    })
                }
            }
        }
    }

    pub fn bind_listener(&self) -> Result<BoundControlSocket, IpcError> {
        self.bind_listener_with_lock(None)
    }

    /// Binds the listener, holding `lock` (taken early by the caller, before
    /// any state is touched) or acquiring the instance lock itself.
    pub fn bind_listener_with_lock(
        &self,
        lock: Option<InstanceLock>,
    ) -> Result<BoundControlSocket, IpcError> {
        validate_socket_path_length(&self.path)?;
        let instance_lock = match lock {
            Some(lock) => lock,
            None => self.acquire_instance_lock()?,
        };
        prepare_socket_path(&self.path)?;

        let listener = UnixListener::bind(&self.path).map_err(|source| {
            if source.kind() == io::ErrorKind::AddrInUse {
                IpcError::SocketInUse {
                    target: self.path.display().to_string(),
                }
            } else {
                IpcError::Io {
                    target: self.path.display().to_string(),
                    source,
                }
            }
        })?;
        listener
            .set_nonblocking(true)
            .map_err(|source| IpcError::Io {
                target: self.path.display().to_string(),
                source,
            })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &self.path,
                std::fs::Permissions::from_mode(SOCKET_FILE_MODE),
            );
        }

        let identity = fs::symlink_metadata(&self.path)
            .ok()
            .map(|metadata| (metadata.dev(), metadata.ino()));

        Ok(BoundControlSocket {
            listener,
            path: self.path.clone(),
            identity,
            _instance_lock: instance_lock,
        })
    }

    pub fn send_request(&self, request: &RequestEnvelope) -> Result<ResponseEnvelope, IpcError> {
        validate_socket_path_length(&self.path)?;
        let target = self.path.display().to_string();
        let mut stream = match UnixStream::connect(&self.path) {
            Ok(stream) => stream,
            Err(source)
                if matches!(
                    source.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                return Err(IpcError::Unavailable {
                    target,
                    message: source.to_string(),
                });
            }
            Err(source) => {
                return Err(IpcError::Io { target, source });
            }
        };

        configure_client_stream(&stream, &target)?;
        write_json_message(&mut stream, request, &target)?;
        read_response(&mut stream, &target)?.validate()
    }

    #[must_use]
    pub fn endpoint(&self) -> paths::IpcEndpoint {
        paths::IpcEndpoint::UnixSocket(self.path.clone())
    }
}

/// Exclusive single-instance guard; released when dropped or the process dies.
/// How long a contended instance lock is retried before giving up.
const INSTANCE_LOCK_GRACE: std::time::Duration = std::time::Duration::from_millis(500);

#[derive(Debug)]
pub struct InstanceLock {
    _file: fs::File,
}

fn instance_lock_path(socket_path: &Path) -> PathBuf {
    let mut name = socket_path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".lock");
    socket_path.with_file_name(name)
}

#[derive(Debug)]
pub struct BoundControlSocket {
    listener: UnixListener,
    path: PathBuf,
    /// `(dev, ino)` of the socket file we bound, so Drop never unlinks a
    /// socket another process created at the same path.
    identity: Option<(u64, u64)>,
    /// Declared last: released after the socket file is removed.
    _instance_lock: InstanceLock,
}

impl BoundControlSocket {
    #[must_use]
    pub fn endpoint(&self) -> paths::IpcEndpoint {
        paths::IpcEndpoint::UnixSocket(self.path.clone())
    }

    #[must_use]
    pub fn display_target(&self) -> String {
        self.path.display().to_string()
    }

    pub fn accept(&self) -> Result<Option<UnixStream>, IpcError> {
        let target = self.path.display().to_string();
        match self.listener.accept() {
            Ok((stream, _)) => {
                #[cfg(unix)]
                {
                    use rustix::net::sockopt::get_socket_peercred;
                    use std::os::unix::io::AsFd;

                    match get_socket_peercred(stream.as_fd()) {
                        Ok(creds) => {
                            // SAFETY: geteuid has no preconditions and cannot fail.
                            let current_euid = unsafe { libc::geteuid() };
                            if creds.uid.as_raw() != current_euid {
                                tracing::warn!(
                                    "rejected unauthorized IPC connection from uid {}",
                                    creds.uid.as_raw()
                                );
                                return Err(IpcError::Protocol {
                                    message: format!(
                                        "Rejecting connection from unauthorized UID: {}",
                                        creds.uid.as_raw()
                                    ),
                                });
                            }
                        }
                        Err(e) => {
                            return Err(IpcError::Protocol {
                                message: format!("Failed to read SO_PEERCRED: {e}"),
                            });
                        }
                    }
                }

                configure_server_stream(&stream, &target)?;
                Ok(Some(stream))
            }
            Err(source) if source.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(source) => Err(IpcError::Io { target, source }),
        }
    }
}

impl Drop for BoundControlSocket {
    fn drop(&mut self) {
        let still_ours = match (self.identity, fs::symlink_metadata(&self.path)) {
            (Some(bound), Ok(current)) => bound == (current.dev(), current.ino()),
            _ => false,
        };
        if still_ours {
            fs::remove_file(&self.path).ok();
        }
    }
}

impl std::os::unix::io::AsRawFd for BoundControlSocket {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.listener.as_raw_fd()
    }
}

fn validate_socket_path_length(path: &Path) -> Result<(), IpcError> {
    let bytes_len = path.as_os_str().as_bytes().len();
    if bytes_len > MAX_UNIX_SOCKET_PATH_LEN {
        return Err(IpcError::UnsafeSocketPath {
            target: path.display().to_string(),
            message: format!(
                "IPC socket path exceeds maximum Unix domain socket length ({MAX_UNIX_SOCKET_PATH_LEN} bytes, got {bytes_len})"
            ),
        });
    }
    Ok(())
}

fn prepare_socket_path(path: &Path) -> Result<(), IpcError> {
    validate_socket_path_length(path)?;
    let target = path.display().to_string();
    if let Some(parent) = path.parent() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(SOCKET_DIR_MODE);
        builder.create(parent).map_err(|source| IpcError::Io {
            target: parent.display().to_string(),
            source,
        })?;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(SOCKET_DIR_MODE));
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(IpcError::Io { target, source });
        }
    };

    if !metadata.file_type().is_socket() {
        return Err(IpcError::UnsafeSocketPath {
            target,
            message: String::from("path exists but is not a Unix socket"),
        });
    }

    match UnixStream::connect(path) {
        Ok(_) => Err(IpcError::SocketInUse { target }),
        Err(source)
            if matches!(
                source.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(path).map_err(|remove_error| IpcError::Io {
                target,
                source: remove_error,
            })?;
            Ok(())
        }
        Err(source) => Err(IpcError::UnsafeSocketPath {
            target,
            message: source.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::{IpcError, Request, RequestEnvelope};
    use super::ControlSocket;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn bind_listener_replaces_stale_socket_and_sets_restrictive_permissions() {
        let temp = TempDir::new();
        let socket_path = temp.path().join("run/control.sock");

        if let Some(parent) = socket_path.parent() {
            fs::create_dir_all(parent).expect("socket dir should exist");
        }
        let stale_listener = UnixListener::bind(&socket_path).expect("stale socket should bind");
        drop(stale_listener);

        let socket = ControlSocket {
            path: socket_path.clone(),
        };
        let bound = socket
            .bind_listener()
            .expect("stale socket should be replaced");

        let socket_mode = fs::metadata(&socket_path)
            .expect("socket metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = fs::metadata(socket_path.parent().expect("socket dir"))
            .expect("socket dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(socket_mode, 0o600);
        assert_eq!(dir_mode, 0o700);

        drop(bound);
        assert!(!socket_path.exists());
    }

    #[test]
    fn second_daemon_bind_rejected_when_active() {
        let temp = TempDir::new();
        let socket_path = temp.path().join("run/control.sock");
        let socket = ControlSocket {
            path: socket_path.clone(),
        };

        let bound = socket.bind_listener().expect("first bind should succeed");
        assert_eq!(
            bound.endpoint(),
            crate::paths::IpcEndpoint::UnixSocket(socket_path.clone())
        );

        let second_socket = ControlSocket {
            path: socket_path.clone(),
        };
        let error = second_socket
            .bind_listener()
            .expect_err("second bind must be rejected while first daemon is active");

        assert!(matches!(error, super::IpcError::SocketInUse { .. }));
        drop(bound);
    }

    #[test]
    fn instance_lock_blocks_second_holder_even_without_a_live_listener() {
        let temp = TempDir::new();
        let socket = ControlSocket {
            path: temp.path().join("run/control.sock"),
        };
        let first = socket.acquire_instance_lock().expect("first lock");
        let error = socket
            .acquire_instance_lock()
            .expect_err("second lock must be refused");
        assert!(matches!(error, super::IpcError::SocketInUse { .. }));
        // bind_listener acquires the same lock, so it is refused as well.
        assert!(matches!(
            socket.bind_listener().expect_err("bind must be refused"),
            super::IpcError::SocketInUse { .. }
        ));
        drop(first);
        socket.acquire_instance_lock().expect("lock is free again");
    }

    #[test]
    fn drop_leaves_a_socket_replaced_by_another_process() {
        let temp = TempDir::new();
        let socket_path = temp.path().join("run/control.sock");
        let socket = ControlSocket {
            path: socket_path.clone(),
        };
        let bound = socket.bind_listener().expect("bind");
        // Another process replaced the path while we were running.
        fs::remove_file(&socket_path).expect("unlink ours");
        let _other = UnixListener::bind(&socket_path).expect("foreign socket");
        drop(bound);
        assert!(
            socket_path.exists(),
            "a socket we did not bind must not be unlinked"
        );
    }

    #[test]
    fn unsafe_non_socket_path_rejected_and_preserved() {
        let temp = TempDir::new();
        let regular_file = temp.path().join("not-a-socket.txt");
        fs::write(&regular_file, "important user data\n").expect("regular file write");

        let socket = ControlSocket {
            path: regular_file.clone(),
        };
        let error = socket
            .bind_listener()
            .expect_err("binding over regular file must be rejected");

        assert!(matches!(error, super::IpcError::UnsafeSocketPath { .. }));
        assert!(regular_file.exists(), "regular file must NOT be deleted");
        assert_eq!(
            fs::read_to_string(&regular_file).expect("read"),
            "important user data\n"
        );
    }

    #[test]
    fn excessively_long_socket_path_fails_safely() {
        let long_path = PathBuf::from(format!("/run/user/1000/{}/control.sock", "a".repeat(120)));
        let socket = ControlSocket {
            path: long_path.clone(),
        };
        let bind_err = socket
            .bind_listener()
            .expect_err("excessively long path must fail");
        assert!(matches!(bind_err, IpcError::UnsafeSocketPath { .. }));
        assert!(bind_err
            .to_string()
            .contains("exceeds maximum Unix domain socket length"));

        let request = RequestEnvelope {
            version: 1,
            request: Request::Status,
        };
        let send_err = socket
            .send_request(&request)
            .expect_err("excessively long send must fail");
        assert!(matches!(send_err, IpcError::UnsafeSocketPath { .. }));
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
