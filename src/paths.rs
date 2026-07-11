use std::env;
use std::path::{Path, PathBuf};

pub const CONFIG_DIR: &str = "~/.config/sunreactor/";
pub const CONFIG_FILE: &str = "~/.config/sunreactor/config.toml";
pub const STATE_DIR: &str = "~/.local/state/sunreactor/";
pub const STATE_FILE: &str = "~/.local/state/sunreactor/runtime-state.json";
pub const CACHE_DIR: &str = "~/.cache/sunreactor/";
pub const SOCKET_DIR_TEMPLATE: &str = "/run/user/$UID/sunreactor/";
pub const SOCKET_PATH_TEMPLATE: &str = "/run/user/$UID/sunreactor/control.sock";

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
}

pub fn config_dir() -> Result<PathBuf, PathError> {
    resolve_user_dir("XDG_CONFIG_HOME", ".config").map(|base| base.join(APP_DIR_NAME))
}

pub fn config_file() -> Result<PathBuf, PathError> {
    config_dir().map(|path| path.join(CONFIG_FILE_NAME))
}

pub fn state_dir() -> Result<PathBuf, PathError> {
    resolve_user_dir("XDG_STATE_HOME", ".local/state").map(|base| base.join(APP_DIR_NAME))
}

pub fn state_file() -> Result<PathBuf, PathError> {
    state_dir().map(|path| path.join(STATE_FILE_NAME))
}

pub fn cache_dir() -> Result<PathBuf, PathError> {
    resolve_user_dir("XDG_CACHE_HOME", ".cache").map(|base| base.join(APP_DIR_NAME))
}

pub fn runtime_socket_dir() -> Result<PathBuf, PathError> {
    resolve_runtime_base_dir().map(|base| base.join(APP_DIR_NAME))
}

pub fn runtime_socket_path() -> Result<PathBuf, PathError> {
    runtime_socket_dir().map(|path| path.join("control.sock"))
}

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

fn home_dir() -> Result<PathBuf, PathError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or(PathError::MissingHome)
}

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

fn current_uid() -> Option<u32> {
    let uid = unsafe { libc::geteuid() };
    if uid == u32::MAX {
        None
    } else {
        Some(uid)
    }
}
