use crate::backends::BackendKind;
use crate::paths::PathError;
use crate::solar::LunarPhase;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub version: u32,
    #[serde(flatten)]
    pub request: Request,
}

impl RequestEnvelope {
    #[must_use]
    pub fn new(request: Request) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request,
        }
    }

    #[must_use]
    pub fn run_once(force: bool) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request: Request::RunOnce { force },
        }
    }

    pub fn validate(self) -> Result<Request, Box<ResponseEnvelope>> {
        if self.version != PROTOCOL_VERSION {
            return Err(Box::new(ResponseEnvelope::error(
                ErrorCode::UnsupportedVersion,
                format!(
                    "unsupported IPC protocol version {}; expected {}",
                    self.version, PROTOCOL_VERSION
                ),
            )));
        }

        Ok(self.request)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case")]
pub enum Request {
    Status,
    Suspend {
        #[serde(default)]
        minutes: Option<u64>,
    },
    Resume,
    IdleDim,
    IdleWake,
    SetOverride {
        monitor_id: Option<String>,
        percent: u8,
        minutes: Option<u64>,
    },
    ClearOverride {
        monitor_id: Option<String>,
        global: bool,
    },
    ReloadConfig,
    Ping,
    RunOnce {
        force: bool,
    },
    ExternalBrightnessChange,
}

impl Request {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Suspend { .. } => "suspend",
            Self::Resume => "resume",
            Self::IdleDim => "idle_dim",
            Self::IdleWake => "idle_wake",
            Self::SetOverride { .. } => "set_override",
            Self::ClearOverride { .. } => "clear_override",
            Self::ReloadConfig => "reload_config",
            Self::Ping => "ping",
            Self::RunOnce { .. } => "run_once",
            Self::ExternalBrightnessChange => "external_brightness_change",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub version: u32,
    #[serde(flatten)]
    pub response: Response,
}

impl ResponseEnvelope {
    #[must_use]
    pub fn pong() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Pong {
                message: String::from("pong"),
            },
        }
    }

    pub fn ack(message: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Ack {
                message: message.into(),
            },
        }
    }

    #[must_use]
    pub fn status(status: StatusResponse) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Status { status },
        }
    }

    pub fn run_once(run_once: RunOnceResponse, message: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::RunOnce {
                run_once,
                message: message.into(),
            },
        }
    }

    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Error {
                code,
                message: message.into(),
            },
        }
    }

    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        match &self.response {
            Response::Pong { .. } => "pong",
            Response::Ack { .. } => "ack",
            Response::Status { .. } => "status",
            Response::RunOnce { .. } => "run_once",
            Response::Error { .. } => "error",
        }
    }

    pub fn validate(self) -> Result<Self, IpcError> {
        if self.version != PROTOCOL_VERSION {
            return Err(IpcError::Protocol {
                message: format!(
                    "unsupported IPC protocol version {}; expected {}",
                    self.version, PROTOCOL_VERSION
                ),
            });
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum Response {
    Pong {
        message: String,
    },
    Ack {
        message: String,
    },
    Status {
        status: StatusResponse,
    },
    RunOnce {
        run_once: RunOnceResponse,
        message: String,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    UnsupportedVersion,
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusResponse {
    pub daemon_alive: bool,
    pub config_path: String,
    pub tick_seconds: u64,
    pub dry_run: bool,
    pub suspended: bool,
    pub desktop_idle_dimmed: bool,
    pub suspend_until_epoch_s: Option<u64>,
    pub manual_override_active: bool,
    pub per_monitor_override_until_epoch_s: Option<u64>,
    pub global_override_percent: Option<u8>,
    pub global_override_until_epoch_s: Option<u64>,
    pub configured_monitors: u32,
    pub stateful_monitors: u32,
    pub weather: Option<WeatherStatus>,
    pub monitors: Vec<MonitorStatus>,
    pub solar_elevation: Option<f64>,
    pub now_epoch_s: u64,
    pub sunrise_epoch_s: Option<u64>,
    pub sunset_epoch_s: Option<u64>,
    /// Current lunar phase calculated offline from the synodic cycle.
    /// `None` only when the daemon predates this field (backward compat).
    #[serde(default)]
    pub lunar_phase: Option<LunarPhase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorStatus {
    pub logical_id: String,
    pub backend: BackendKind,
    pub enabled: bool,
    pub override_percent: Option<u8>,
    pub last_applied_percent: Option<u8>,
    pub last_applied_at_epoch_s: Option<u64>,
    pub backoff_until_epoch_s: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherStatus {
    pub enabled: bool,
    pub active: bool,
    pub stale: bool,
    pub provider: Option<String>,
    pub observed_at_epoch_s: Option<u64>,
    pub last_refresh_attempt_epoch_s: Option<u64>,
    pub next_refresh_at_epoch_s: Option<u64>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub cloud_cover_percent: Option<u8>,
    pub temperature: Option<f32>,
    pub forecast: Vec<crate::state::ForecastPoint>,
    pub multiplier: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunOnceResponse {
    pub tick_duration_ms: u64,
    pub monitors_evaluated: u32,
    pub writes_attempted: u32,
    pub writes_skipped: u32,
    pub writes_succeeded: u32,
    pub writes_failed: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("failed to access {}: {}", path.display(), source)]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to encode or decode IPC message: {source}")]
    Json {
        #[source]
        source: serde_json::Error,
    },
    #[error("{message}")]
    Protocol { message: String },
    #[error("daemon is not reachable via {}: {message}", path.display())]
    Unavailable { path: PathBuf, message: String },
    #[error("refusing to replace {}: {message}", path.display())]
    UnsafeSocketPath { path: PathBuf, message: String },
    #[error("another daemon appears to be listening on {}", path.display())]
    SocketInUse { path: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, Request, RequestEnvelope, ResponseEnvelope};

    #[test]
    fn request_validation_rejects_wrong_protocol_version() {
        let response = RequestEnvelope {
            version: 99,
            request: Request::Status,
        }
        .validate()
        .expect_err("unsupported protocol version must fail");

        assert_eq!(response.kind_name(), "error");
        assert_eq!(
            *response,
            ResponseEnvelope::error(
                ErrorCode::UnsupportedVersion,
                "unsupported IPC protocol version 99; expected 1",
            )
        );
    }
}
