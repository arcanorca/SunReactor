#[cfg(target_os = "linux")]
use std::env;
use std::path::{Path, PathBuf};

pub const CONFIG_DIR: &str = "~/.config/sunreactor/";
pub const CONFIG_FILE: &str = "~/.config/sunreactor/config.toml";
pub const STATE_DIR: &str = "~/.local/state/sunreactor/";
pub const STATE_FILE: &str = "~/.local/state/sunreactor/runtime-state.json";
pub const CACHE_DIR: &str = "~/.cache/sunreactor/";
pub const SOCKET_DIR_TEMPLATE: &str = "/run/user/$UID/sunreactor/";
pub const SOCKET_PATH_TEMPLATE: &str = "/run/user/$UID/sunreactor/control.sock";

#[cfg(target_os = "linux")]
const APP_DIR_NAME: &str = "sunreactor";
const CONFIG_FILE_NAME: &str = "config.toml";
const STATE_FILE_NAME: &str = "runtime-state.json";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("HOME is not set; cannot resolve XDG fallback paths")]
    MissingHome,
    #[error("XDG_RUNTIME_DIR is not set and the current user id could not be resolved")]
    MissingRuntimeUid,
    #[error("{env_var} must be an absolute path, got {}", value.display())]
    InvalidAbsolutePath {
        env_var: &'static str,
        value: PathBuf,
    },
    #[cfg(target_os = "windows")]
    #[error("Windows security/SID query failed: {0}")]
    WindowsSecurity(String),
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    WindowsPath(#[from] crate::platform::windows::paths::WindowsPathError),
}

pub fn config_dir() -> Result<PathBuf, PathError> {
    #[cfg(target_os = "linux")]
    {
        resolve_user_dir("XDG_CONFIG_HOME", ".config").map(|base| base.join(APP_DIR_NAME))
    }
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::win_paths::config_dir().map_err(PathError::from)
    }
}

pub fn config_file() -> Result<PathBuf, PathError> {
    config_dir().map(|path| path.join(CONFIG_FILE_NAME))
}

pub fn state_dir() -> Result<PathBuf, PathError> {
    #[cfg(target_os = "linux")]
    {
        resolve_user_dir("XDG_STATE_HOME", ".local/state").map(|base| base.join(APP_DIR_NAME))
    }
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::win_paths::state_dir().map_err(PathError::from)
    }
}

pub fn state_file() -> Result<PathBuf, PathError> {
    state_dir().map(|path| path.join(STATE_FILE_NAME))
}

pub fn cache_dir() -> Result<PathBuf, PathError> {
    #[cfg(target_os = "linux")]
    {
        resolve_user_dir("XDG_CACHE_HOME", ".cache").map(|base| base.join(APP_DIR_NAME))
    }
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::win_paths::cache_dir().map_err(PathError::from)
    }
}

#[cfg(target_os = "linux")]
pub fn runtime_socket_dir() -> Result<PathBuf, PathError> {
    resolve_runtime_base_dir().map(|base| base.join(APP_DIR_NAME))
}

pub fn runtime_socket_path() -> Result<PathBuf, PathError> {
    #[cfg(target_os = "linux")]
    {
        runtime_socket_dir().map(|path| path.join("control.sock"))
    }
    #[cfg(target_os = "windows")]
    {
        AppPaths::from_environment().map(|paths| match paths.ipc_endpoint {
            IpcEndpoint::NamedPipe(pipe) => PathBuf::from(pipe),
            IpcEndpoint::UnixSocket(path) => path,
        })
    }
}

/// Represents an IPC communication endpoint.
///
/// On Linux/Unix, this is backed by a Unix-domain socket path in the user runtime directory.
/// On Windows (future port), this will be backed by a native Named Pipe identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcEndpoint {
    UnixSocket(PathBuf),
    #[allow(dead_code)]
    NamedPipe(String),
}

impl IpcEndpoint {
    #[must_use]
    pub fn display_target(&self) -> String {
        match self {
            Self::UnixSocket(path) => path.display().to_string(),
            Self::NamedPipe(pipe) => pipe.clone(),
        }
    }

    #[must_use]
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Self::UnixSocket(path) => Some(path.as_path()),
            Self::NamedPipe(_) => None,
        }
    }
}

/// Authoritative paths container for configuration, runtime state, and IPC endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub state_dir: PathBuf,
    pub state_file: PathBuf,
    pub cache_dir: PathBuf,
    pub ipc_endpoint: IpcEndpoint,
}

impl AppPaths {
    pub fn from_environment() -> Result<Self, PathError> {
        let cfg_dir = config_dir()?;
        let cfg_file = cfg_dir.join(CONFIG_FILE_NAME);
        let st_dir = state_dir()?;
        let st_file = st_dir.join(STATE_FILE_NAME);
        let c_dir = cache_dir()?;

        #[cfg(target_os = "linux")]
        {
            let socket_path = runtime_socket_path()?;
            Ok(Self {
                config_dir: cfg_dir,
                config_file: cfg_file,
                state_dir: st_dir,
                state_file: st_file,
                cache_dir: c_dir,
                ipc_endpoint: IpcEndpoint::UnixSocket(socket_path),
            })
        }

        #[cfg(target_os = "windows")]
        {
            let pipe_name = crate::platform::windows::pipe::default_windows_pipe_name()?;
            Ok(Self {
                config_dir: cfg_dir,
                config_file: cfg_file,
                state_dir: st_dir,
                state_file: st_file,
                cache_dir: c_dir,
                ipc_endpoint: IpcEndpoint::NamedPipe(pipe_name),
            })
        }
    }

    #[must_use]
    pub fn custom(
        config_dir: PathBuf,
        state_dir: PathBuf,
        cache_dir: PathBuf,
        ipc_endpoint: IpcEndpoint,
    ) -> Self {
        let config_file = config_dir.join(CONFIG_FILE_NAME);
        let state_file = state_dir.join(STATE_FILE_NAME);
        Self {
            config_dir,
            config_file,
            state_dir,
            state_file,
            cache_dir,
            ipc_endpoint,
        }
    }
}

#[cfg(target_os = "linux")]
fn resolve_user_dir(env_var: &'static str, fallback_suffix: &str) -> Result<PathBuf, PathError> {
    match env::var_os(env_var) {
        Some(value) if !value.is_empty() => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                Ok(path)
            } else {
                Err(PathError::InvalidAbsolutePath {
                    env_var,
                    value: path,
                })
            }
        }
        _ => home_dir().map(|path| path.join(fallback_suffix)),
    }
}

#[cfg(target_os = "linux")]
fn home_dir() -> Result<PathBuf, PathError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or(PathError::MissingHome)
}

#[cfg(target_os = "linux")]
fn resolve_runtime_base_dir() -> Result<PathBuf, PathError> {
    match env::var_os("XDG_RUNTIME_DIR") {
        Some(value) if !value.is_empty() => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                Ok(path)
            } else {
                Err(PathError::InvalidAbsolutePath {
                    env_var: "XDG_RUNTIME_DIR",
                    value: path,
                })
            }
        }
        _ => current_uid()
            .map(|uid| Path::new("/run/user").join(uid.to_string()))
            .ok_or(PathError::MissingRuntimeUid),
    }
}

#[cfg(target_os = "linux")]
fn current_uid() -> Option<u32> {
    let uid = unsafe { libc::geteuid() };
    if uid == u32::MAX {
        None
    } else {
        Some(uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_endpoint_displays_target() {
        let endpoint =
            IpcEndpoint::UnixSocket(PathBuf::from("/run/user/1000/sunreactor/control.sock"));
        assert_eq!(
            endpoint.display_target(),
            "/run/user/1000/sunreactor/control.sock"
        );
        assert_eq!(
            endpoint.as_path(),
            Some(Path::new("/run/user/1000/sunreactor/control.sock"))
        );

        let pipe_endpoint = IpcEndpoint::NamedPipe(String::from(r"\\.\pipe\sunreactor"));
        assert_eq!(pipe_endpoint.display_target(), r"\\.\pipe\sunreactor");
        assert_eq!(pipe_endpoint.as_path(), None);
    }

    #[test]
    fn app_paths_custom_construction() {
        let endpoint = IpcEndpoint::UnixSocket(PathBuf::from("/tmp/test/control.sock"));
        let paths = AppPaths::custom(
            PathBuf::from("/tmp/config"),
            PathBuf::from("/tmp/state"),
            PathBuf::from("/tmp/cache"),
            endpoint.clone(),
        );

        assert_eq!(paths.config_dir, Path::new("/tmp/config"));
        assert_eq!(paths.config_file, Path::new("/tmp/config/config.toml"));
        assert_eq!(paths.state_dir, Path::new("/tmp/state"));
        assert_eq!(paths.state_file, Path::new("/tmp/state/runtime-state.json"));
        assert_eq!(paths.cache_dir, Path::new("/tmp/cache"));
        assert_eq!(paths.ipc_endpoint, endpoint);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn app_paths_from_environment_succeeds_when_home_is_set() {
        if std::env::var_os("HOME").is_some() {
            let paths = AppPaths::from_environment().expect("paths should resolve");
            assert!(paths.config_file.ends_with("config.toml"));
            assert!(paths.state_file.ends_with("runtime-state.json"));
            assert!(matches!(paths.ipc_endpoint, IpcEndpoint::UnixSocket(_)));
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn app_paths_from_environment_succeeds_on_windows() {
        let paths = AppPaths::from_environment().expect("paths should resolve");
        assert!(paths.config_file.ends_with("config.toml"));
        assert!(paths.state_file.ends_with("runtime-state.json"));
        assert!(matches!(paths.ipc_endpoint, IpcEndpoint::NamedPipe(_)));
    }
}
