use crate::backends::BackendKind;
use crate::config::TemperatureUnit;
use crate::paths::PathError;
use crate::solar::LunarPhase;
use serde::{Deserialize, Serialize};
use std::io;

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

/// Longest error message carried on the wire. Decode errors echo
/// attacker-controlled text (e.g. unknown variant names); without a cap a
/// near-limit request produced an error reply over the frame limit that could
/// not be written, leaving the client with a silent hang-up.
const MAX_ERROR_MESSAGE_BYTES: usize = 2048;

fn truncate_error_message(mut message: String) -> String {
    if message.len() > MAX_ERROR_MESSAGE_BYTES {
        let mut end = MAX_ERROR_MESSAGE_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
        message.push_str("...");
    }
    message
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
    SetMonitorLimits {
        monitor_id: String,
        min_pct: u8,
        max_pct: u8,
    },
    SetCloudDimming {
        percent: u8,
    },
    ClearOverride {
        monitor_id: Option<String>,
        global: bool,
    },
    ReloadConfig,
    RefreshWeather,
    Ping,
    RunOnce {
        force: bool,
    },
    ClearExternalHold {
        #[serde(default)]
        monitor: Option<String>,
    },
    ExternalBrightnessChange,
    /// Summaries of every theme in the daemon's registry.
    ListThemes,
    /// One fully resolved theme (id or alias).
    GetTheme {
        id: String,
    },
    /// Today's policy curve; `monitor_id: null` means every monitor.
    DayCurve {
        #[serde(default)]
        monitor_id: Option<String>,
    },
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
            Self::SetMonitorLimits { .. } => "set_monitor_limits",
            Self::SetCloudDimming { .. } => "set_cloud_dimming",
            Self::ClearOverride { .. } => "clear_override",
            Self::ReloadConfig => "reload_config",
            Self::RefreshWeather => "refresh_weather",
            Self::Ping => "ping",
            Self::RunOnce { .. } => "run_once",
            Self::ClearExternalHold { .. } => "clear_external_hold",
            Self::ExternalBrightnessChange => "external_brightness_change",
            Self::ListThemes => "list_themes",
            Self::GetTheme { .. } => "get_theme",
            Self::DayCurve { .. } => "day_curve",
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

    #[must_use]
    pub fn themes(themes: Vec<ThemeSummary>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Themes { themes },
        }
    }

    #[must_use]
    pub fn theme(theme: ThemeStatus) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Theme { theme },
        }
    }

    #[must_use]
    pub fn day_curve(day_curve: DayCurveResponse) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::DayCurve { day_curve },
        }
    }

    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response: Response::Error {
                code,
                message: truncate_error_message(message.into()),
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
            Response::Themes { .. } => "themes",
            Response::Theme { .. } => "theme",
            Response::DayCurve { .. } => "day_curve",
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
// Keep the status payload inline: this preserves the existing public response
// shape and avoids changing the IPC response construction API just for layout.
#[allow(clippy::large_enum_variant)] // inline status payload keeps the wire/API shape stable
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
    Themes {
        themes: Vec<ThemeSummary>,
    },
    Theme {
        theme: ThemeStatus,
    },
    DayCurve {
        day_curve: DayCurveResponse,
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
    /// Active TUI theme and palette hex codes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeStatus>,
    /// Set when the configured `tui.theme` could not be used as-is (unknown
    /// id, or its user theme file is invalid) and `theme` is a fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_warning: Option<String>,
    /// Preferred weather temperature unit (`"celsius"` or `"fahrenheit"`).
    /// `None` only when an older daemon predates this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_unit: Option<TemperatureUnit>,
}

/// A resolved theme as sent in `status.theme` and the `theme` response.
///
/// The flat colour fields predate the theme engine and stay for older
/// clients; everything after `text_muted` is optional on the wire so older
/// daemons (which omit it) still parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeStatus {
    pub name: String,
    pub accent: String,
    pub secondary_accent: String,
    pub bg: String,
    pub fg: String,
    pub text_muted: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<ThemePaletteStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<ThemeWidgetStatus>,
}

impl ThemeStatus {
    #[must_use]
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        let palette = &theme.palette;
        let widget = &theme.widget;
        Self {
            name: theme.name.clone(),
            accent: palette.accent.to_hex(),
            secondary_accent: palette.secondary_accent.to_hex(),
            bg: palette.bg.to_hex(),
            fg: palette.fg.to_hex(),
            text_muted: palette.text_muted.to_hex(),
            id: Some(theme.id.clone()),
            family: Some(theme.family.as_str().to_owned()),
            glyph_style: Some(theme.tui.glyph_style.as_str().to_owned()),
            palette: Some(ThemePaletteStatus {
                bg: palette.bg.to_hex(),
                fg: palette.fg.to_hex(),
                accent: palette.accent.to_hex(),
                secondary_accent: palette.secondary_accent.to_hex(),
                border_active: palette.border_active.to_hex(),
                border_inactive: palette.border_inactive.to_hex(),
                text_muted: palette.text_muted.to_hex(),
                success: palette.success.to_hex(),
                warning: palette.warning.to_hex(),
                error: palette.error.to_hex(),
            }),
            widget: Some(ThemeWidgetStatus {
                renderer: widget.renderer.as_str().to_owned(),
                font: widget.font.as_str().to_owned(),
                pixel_scale: widget.pixel_scale,
                shades: widget.shades.iter().map(|shade| shade.to_hex()).collect(),
                density: widget.density.as_str().to_owned(),
                border: widget.border.as_str().to_owned(),
                radius: widget.radius,
                dither: widget.dither.as_str().to_owned(),
                glow: widget.glow,
                modules: widget
                    .modules
                    .iter()
                    .map(|module| module.as_str().to_owned())
                    .collect(),
            }),
        }
    }
}

/// All ten palette colours as `#rrggbb`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemePaletteStatus {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub secondary_accent: String,
    pub border_active: String,
    pub border_inactive: String,
    pub text_muted: String,
    pub success: String,
    pub warning: String,
    pub error: String,
}

/// Widget tokens with family defaults applied. Enum-like values travel as
/// strings so a newer daemon's values never break an older client's parse.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeWidgetStatus {
    pub renderer: String,
    pub font: String,
    pub pixel_scale: u8,
    pub shades: Vec<String>,
    pub density: String,
    pub border: String,
    pub radius: u8,
    pub dither: String,
    pub glow: f64,
    pub modules: Vec<String>,
}

/// One entry of the `themes` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub family: String,
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub secondary_accent: String,
}

impl ThemeSummary {
    #[must_use]
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            id: theme.id.clone(),
            name: theme.name.clone(),
            family: theme.family.as_str().to_owned(),
            bg: theme.palette.bg.to_hex(),
            fg: theme.palette.fg.to_hex(),
            accent: theme.palette.accent.to_hex(),
            secondary_accent: theme.palette.secondary_accent.to_hex(),
        }
    }
}

/// Payload of the `day_curve` response. All `t` values are Unix seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayCurveResponse {
    /// Local date (`YYYY-MM-DD`) in the configured timezone.
    pub date: String,
    pub step_minutes: u32,
    pub generated_at_epoch_s: u64,
    /// Weather multiplier applied to `pct`; `null` when weather is inactive.
    pub weather_multiplier: Option<f64>,
    pub sun: DayCurveSun,
    pub monitors: Vec<DayCurveMonitor>,
    /// Why `milestones` are empty (e.g. polar day/night), when they are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestones_error: Option<String>,
}

/// Sun events of the local day; `null` where the sun never crosses the
/// threshold (polar edge cases).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCurveSun {
    pub dawn: Option<i64>,
    pub sunrise: Option<i64>,
    pub solar_noon: Option<i64>,
    pub sunset: Option<i64>,
    pub dusk: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCurveMonitor {
    pub monitor_id: String,
    pub min_pct: u8,
    pub max_pct: u8,
    pub samples: Vec<DayCurveSample>,
    pub milestones: Vec<DayCurveMilestone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCurveSample {
    pub t: i64,
    /// Target with the current weather multiplier applied.
    pub pct: u8,
    /// Target from the solar policy alone.
    pub pct_no_weather: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCurveMilestone {
    /// Stable machine id (`rise_start`, `rise_50`, ... `night_floor`).
    pub id: String,
    /// English label as shown by the TUI.
    pub label: String,
    pub t: i64,
    pub pct: u8,
}

impl DayCurveResponse {
    #[must_use]
    pub fn from_curve(curve: &crate::policy::preview::DayCurve, generated_at_epoch_s: u64) -> Self {
        let t = |at: chrono::DateTime<chrono::Utc>| at.timestamp();
        Self {
            date: curve.date.format("%Y-%m-%d").to_string(),
            step_minutes: curve.step_minutes,
            generated_at_epoch_s,
            weather_multiplier: curve.weather_multiplier,
            sun: DayCurveSun {
                dawn: curve.sun.dawn.map(t),
                sunrise: curve.sun.sunrise.map(t),
                solar_noon: curve.sun.solar_noon.map(t),
                sunset: curve.sun.sunset.map(t),
                dusk: curve.sun.dusk.map(t),
            },
            monitors: curve
                .monitors
                .iter()
                .map(|monitor| DayCurveMonitor {
                    monitor_id: monitor.logical_id.clone(),
                    min_pct: monitor.min_pct,
                    max_pct: monitor.max_pct,
                    samples: monitor
                        .samples
                        .iter()
                        .map(|sample| DayCurveSample {
                            t: t(sample.at_utc),
                            pct: sample.percent,
                            pct_no_weather: sample.percent_no_weather,
                        })
                        .collect(),
                    milestones: monitor
                        .milestones
                        .iter()
                        .map(|milestone| DayCurveMilestone {
                            id: milestone.milestone.as_str().to_owned(),
                            label: milestone.milestone.label().to_owned(),
                            t: milestone.adjusted_time_local.timestamp(),
                            pct: milestone.target_percent,
                        })
                        .collect(),
                })
                .collect(),
            milestones_error: curve.milestones_error.as_ref().map(ToString::to_string),
        }
    }
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
    /// Runtime topology classification, when a capability snapshot exists.
    #[serde(default)]
    pub topology: Option<String>,
    #[serde(default)]
    pub min_pct: Option<u8>,
    #[serde(default)]
    pub max_pct: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_hold: Option<ExternalHoldStatus>,
    /// Hardware write accounting; omitted by older daemons.
    #[serde(default)]
    pub writes: Option<WriteBudgetStatus>,
    /// Another controller keeps fighting this monitor; automatic writes are
    /// backed off. Omitted by older daemons.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub contested: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contested_until_epoch_s: Option<u64>,
    /// Earliest epoch at which a rate-capped manual write will be retried.
    /// Omitted by older daemons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_deferred_until_epoch_s: Option<u64>,
}

/// Per-monitor hardware write accounting (DDC writes wear the monitor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBudgetStatus {
    /// Hardware write attempts since the daemon started.
    #[serde(default)]
    pub writes_total: u64,
    /// Budgeted writes inside the current rolling window.
    #[serde(default)]
    pub window_used: u32,
    /// Writes allowed per window.
    #[serde(default)]
    pub window_max: u32,
    /// Length of the rolling window in seconds.
    #[serde(default)]
    pub window_seconds: u64,
    /// Automatic writes are currently deferred because the budget is spent.
    #[serde(default)]
    pub deferred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalHoldStatus {
    pub remaining_s: u64,
    pub observed_percent: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherStatus {
    pub enabled: bool,
    /// Maximum cloud dimming in percent; omitted by older daemons and then
    /// deserialised as the default value for wire compatibility.
    #[serde(default)]
    pub cloud_dimming_percent: u8,
    #[serde(default)]
    pub state: crate::weather::WeatherState,
    pub active: bool,
    pub stale: bool,
    pub provider: Option<String>,
    pub fetched_at_epoch_s: Option<u64>,
    pub valid_at_epoch_s: Option<u64>,
    pub source_kind: Option<crate::weather::WeatherSourceKind>,
    pub last_refresh_attempt_epoch_s: Option<u64>,
    pub next_refresh_at_epoch_s: Option<u64>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub cloud_cover_percent: Option<u8>,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub condition: crate::weather::WeatherCondition,
    #[serde(default)]
    pub condition_description: Option<String>,
    #[serde(default)]
    pub day_phase: Option<crate::weather::WeatherDayPhase>,
    pub forecast: Vec<crate::state::ForecastPoint>,
    pub multiplier: Option<f64>,
    pub details: crate::state::WeatherDetails,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunOnceResponse {
    pub tick_duration_ms: u64,
    pub monitors_evaluated: u32,
    pub writes_attempted: u32,
    pub writes_skipped: u32,
    pub writes_succeeded: u32,
    pub writes_failed: u32,
    /// True when the daemon only queued the apply: it replies before the
    /// hardware write, so the counters above carry no real results.
    #[serde(default)]
    pub queued: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("failed to access {target}: {source}")]
    Io {
        target: String,
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
    #[error("daemon is not reachable via {target}: {message}")]
    Unavailable { target: String, message: String },
    #[error("refusing to replace {target}: {message}")]
    UnsafeSocketPath { target: String, message: String },
    #[error("another daemon appears to be listening on {target}")]
    SocketInUse { target: String },
}

#[cfg(test)]
mod tests {
    use super::{
        DayCurveMilestone, DayCurveMonitor, DayCurveResponse, DayCurveSample, DayCurveSun,
        ErrorCode, ExternalHoldStatus, MonitorStatus, Request, RequestEnvelope, Response,
        ResponseEnvelope, RunOnceResponse, StatusResponse, TemperatureUnit, ThemePaletteStatus,
        ThemeStatus, ThemeSummary, ThemeWidgetStatus, WeatherStatus, WriteBudgetStatus,
    };

    fn every_request_variant() -> Vec<Request> {
        vec![
            Request::Status,
            Request::Suspend { minutes: None },
            Request::Suspend { minutes: Some(15) },
            Request::Resume,
            Request::IdleDim,
            Request::IdleWake,
            Request::SetOverride {
                monitor_id: Some(String::from("desk")),
                percent: 40,
                minutes: Some(10),
            },
            Request::SetOverride {
                monitor_id: None,
                percent: 0,
                minutes: None,
            },
            Request::SetMonitorLimits {
                monitor_id: String::from("desk"),
                min_pct: 5,
                max_pct: 95,
            },
            Request::SetCloudDimming { percent: 20 },
            Request::ClearOverride {
                monitor_id: Some(String::from("desk")),
                global: false,
            },
            Request::ReloadConfig,
            Request::RefreshWeather,
            Request::Ping,
            Request::RunOnce { force: true },
            Request::ClearExternalHold { monitor: None },
            Request::ExternalBrightnessChange,
            Request::ListThemes,
            Request::GetTheme {
                id: String::from("nord"),
            },
            Request::DayCurve {
                monitor_id: Some(String::from("desk")),
            },
        ]
    }

    fn sample_status_response() -> StatusResponse {
        StatusResponse {
            daemon_alive: true,
            config_path: String::from("/c.toml"),
            tick_seconds: 60,
            dry_run: false,
            suspended: true,
            desktop_idle_dimmed: false,
            suspend_until_epoch_s: Some(7),
            manual_override_active: false,
            per_monitor_override_until_epoch_s: None,
            global_override_percent: Some(30),
            global_override_until_epoch_s: None,
            configured_monitors: 1,
            stateful_monitors: 1,
            weather: Some(WeatherStatus::default()),
            monitors: vec![MonitorStatus {
                logical_id: String::from("desk"),
                backend: crate::backends::BackendKind::Ddc,
                enabled: true,
                override_percent: None,
                last_applied_percent: Some(50),
                last_applied_at_epoch_s: Some(3),
                backoff_until_epoch_s: None,
                topology: Some(String::from("ddc")),
                min_pct: Some(1),
                max_pct: Some(99),
                external_hold: Some(ExternalHoldStatus {
                    remaining_s: 60,
                    observed_percent: 22,
                }),
                writes: Some(WriteBudgetStatus {
                    writes_total: 17,
                    window_used: 3,
                    window_max: 12,
                    window_seconds: 600,
                    deferred: false,
                }),
                contested: false,
                contested_until_epoch_s: None,
                manual_deferred_until_epoch_s: None,
            }],
            solar_elevation: Some(12.5),
            now_epoch_s: 100,
            sunrise_epoch_s: Some(1),
            sunset_epoch_s: None,
            lunar_phase: Some(crate::solar::LunarPhase::FullMoon),
            theme: None,
            theme_warning: Some(String::from("fallback")),
            temperature_unit: Some(TemperatureUnit::Fahrenheit),
        }
    }

    fn every_response_variant() -> Vec<ResponseEnvelope> {
        let status = sample_status_response();
        vec![
            ResponseEnvelope::pong(),
            ResponseEnvelope::ack("done"),
            ResponseEnvelope::status(status),
            ResponseEnvelope::run_once(
                RunOnceResponse {
                    tick_duration_ms: 0,
                    monitors_evaluated: 0,
                    writes_attempted: 0,
                    writes_skipped: 0,
                    writes_succeeded: 0,
                    writes_failed: 0,
                    queued: true,
                },
                "daemon tick queued",
            ),
            ResponseEnvelope::themes(vec![ThemeSummary {
                id: String::from("nord"),
                name: String::from("Nord"),
                family: String::from("terminal"),
                bg: String::from("#000000"),
                fg: String::from("#ffffff"),
                accent: String::from("#111111"),
                secondary_accent: String::from("#222222"),
            }]),
            ResponseEnvelope::theme(ThemeStatus {
                name: String::from("Amber"),
                accent: String::from("#ffb000"),
                secondary_accent: String::from("#dc8c00"),
                bg: String::from("#0f0a05"),
                fg: String::from("#ffd296"),
                text_muted: String::from("#b47832"),
                id: Some(String::from("amber")),
                family: None,
                glyph_style: None,
                palette: Some(ThemePaletteStatus::default()),
                widget: Some(ThemeWidgetStatus::default()),
            }),
            ResponseEnvelope::day_curve(DayCurveResponse {
                date: String::from("2026-10-13"),
                step_minutes: 10,
                generated_at_epoch_s: 5,
                weather_multiplier: None,
                sun: DayCurveSun::default(),
                monitors: Vec::new(),
                milestones_error: Some(String::from("polar night")),
            }),
            ResponseEnvelope::error(ErrorCode::InvalidRequest, "bad"),
            ResponseEnvelope::error(ErrorCode::UnsupportedVersion, "old"),
            ResponseEnvelope::error(ErrorCode::InternalError, "boom"),
        ]
    }

    /// Exhaustive on purpose: adding a `Request` variant fails to compile
    /// here until the snapshot below covers it.
    fn request_variant_index(request: &Request) -> usize {
        match request {
            Request::Status => 0,
            Request::Suspend { .. } => 1,
            Request::Resume => 2,
            Request::IdleDim => 3,
            Request::IdleWake => 4,
            Request::SetOverride { .. } => 5,
            Request::SetMonitorLimits { .. } => 6,
            Request::SetCloudDimming { .. } => 7,
            Request::ClearOverride { .. } => 8,
            Request::ReloadConfig => 9,
            Request::RefreshWeather => 10,
            Request::Ping => 11,
            Request::RunOnce { .. } => 12,
            Request::ClearExternalHold { .. } => 13,
            Request::ExternalBrightnessChange => 14,
            Request::ListThemes => 15,
            Request::GetTheme { .. } => 16,
            Request::DayCurve { .. } => 17,
        }
    }

    fn response_variant_index(response: &Response) -> usize {
        match response {
            Response::Pong { .. } => 0,
            Response::Ack { .. } => 1,
            Response::Status { .. } => 2,
            Response::RunOnce { .. } => 3,
            Response::Themes { .. } => 4,
            Response::Theme { .. } => 5,
            Response::DayCurve { .. } => 6,
            Response::Error { .. } => 7,
        }
    }

    #[test]
    fn every_request_variant_has_a_byte_identical_wire_snapshot() {
        let expected = [
            r#"{"version":1,"request":"status"}"#,
            r#"{"version":1,"request":"suspend","minutes":null}"#,
            r#"{"version":1,"request":"suspend","minutes":15}"#,
            r#"{"version":1,"request":"resume"}"#,
            r#"{"version":1,"request":"idle_dim"}"#,
            r#"{"version":1,"request":"idle_wake"}"#,
            r#"{"version":1,"request":"set_override","monitor_id":"desk","percent":40,"minutes":10}"#,
            r#"{"version":1,"request":"set_override","monitor_id":null,"percent":0,"minutes":null}"#,
            r#"{"version":1,"request":"set_monitor_limits","monitor_id":"desk","min_pct":5,"max_pct":95}"#,
            r#"{"version":1,"request":"set_cloud_dimming","percent":20}"#,
            r#"{"version":1,"request":"clear_override","monitor_id":"desk","global":false}"#,
            r#"{"version":1,"request":"reload_config"}"#,
            r#"{"version":1,"request":"refresh_weather"}"#,
            r#"{"version":1,"request":"ping"}"#,
            r#"{"version":1,"request":"run_once","force":true}"#,
            r#"{"version":1,"request":"clear_external_hold","monitor":null}"#,
            r#"{"version":1,"request":"external_brightness_change"}"#,
            r#"{"version":1,"request":"list_themes"}"#,
            r#"{"version":1,"request":"get_theme","id":"nord"}"#,
            r#"{"version":1,"request":"day_curve","monitor_id":"desk"}"#,
        ];
        let requests = every_request_variant();
        assert_eq!(requests.len(), expected.len());
        let mut covered = [false; 18];
        for (request, wire) in requests.into_iter().zip(expected) {
            covered[request_variant_index(&request)] = true;
            let envelope = RequestEnvelope::new(request);
            assert_eq!(serde_json::to_string(&envelope).unwrap(), wire);
            let parsed: RequestEnvelope = serde_json::from_str(wire).unwrap();
            assert_eq!(parsed, envelope);
        }
        assert!(
            covered.iter().all(|seen| *seen),
            "uncovered request variant"
        );
    }

    #[test]
    fn every_response_variant_has_a_byte_identical_wire_snapshot() {
        let expected = [
            r#"{"version":1,"response":"pong","message":"pong"}"#,
            r#"{"version":1,"response":"ack","message":"done"}"#,
            r#"{"version":1,"response":"status","status":{"daemon_alive":true,"config_path":"/c.toml","tick_seconds":60,"dry_run":false,"suspended":true,"desktop_idle_dimmed":false,"suspend_until_epoch_s":7,"manual_override_active":false,"per_monitor_override_until_epoch_s":null,"global_override_percent":30,"global_override_until_epoch_s":null,"configured_monitors":1,"stateful_monitors":1,"weather":{"enabled":false,"cloud_dimming_percent":0,"state":"disabled","active":false,"stale":false,"provider":null,"fetched_at_epoch_s":null,"valid_at_epoch_s":null,"source_kind":null,"last_refresh_attempt_epoch_s":null,"next_refresh_at_epoch_s":null,"consecutive_failures":0,"last_error":null,"cloud_cover_percent":null,"temperature":null,"condition":"unknown","condition_description":null,"day_phase":null,"forecast":[],"multiplier":null,"details":{"feels_like":null,"humidity_percent":null,"pressure_hpa":null,"wind_speed_mps":null,"wind_direction_deg":null,"visibility_m":null,"precipitation_percent":null,"air_quality":null}},"monitors":[{"logical_id":"desk","backend":"ddc","enabled":true,"override_percent":null,"last_applied_percent":50,"last_applied_at_epoch_s":3,"backoff_until_epoch_s":null,"topology":"ddc","min_pct":1,"max_pct":99,"external_hold":{"remaining_s":60,"observed_percent":22},"writes":{"writes_total":17,"window_used":3,"window_max":12,"window_seconds":600,"deferred":false}}],"solar_elevation":12.5,"now_epoch_s":100,"sunrise_epoch_s":1,"sunset_epoch_s":null,"lunar_phase":"full_moon","theme_warning":"fallback","temperature_unit":"fahrenheit"}}"#,
            r#"{"version":1,"response":"run_once","run_once":{"tick_duration_ms":0,"monitors_evaluated":0,"writes_attempted":0,"writes_skipped":0,"writes_succeeded":0,"writes_failed":0,"queued":true},"message":"daemon tick queued"}"#,
            r##"{"version":1,"response":"themes","themes":[{"id":"nord","name":"Nord","family":"terminal","bg":"#000000","fg":"#ffffff","accent":"#111111","secondary_accent":"#222222"}]}"##,
            r##"{"version":1,"response":"theme","theme":{"name":"Amber","accent":"#ffb000","secondary_accent":"#dc8c00","bg":"#0f0a05","fg":"#ffd296","text_muted":"#b47832","id":"amber","palette":{"bg":"","fg":"","accent":"","secondary_accent":"","border_active":"","border_inactive":"","text_muted":"","success":"","warning":"","error":""},"widget":{"renderer":"","font":"","pixel_scale":0,"shades":[],"density":"","border":"","radius":0,"dither":"","glow":0.0,"modules":[]}}}"##,
            r#"{"version":1,"response":"day_curve","day_curve":{"date":"2026-10-13","step_minutes":10,"generated_at_epoch_s":5,"weather_multiplier":null,"sun":{"dawn":null,"sunrise":null,"solar_noon":null,"sunset":null,"dusk":null},"monitors":[],"milestones_error":"polar night"}}"#,
            r#"{"version":1,"response":"error","code":"invalid_request","message":"bad"}"#,
            r#"{"version":1,"response":"error","code":"unsupported_version","message":"old"}"#,
            r#"{"version":1,"response":"error","code":"internal_error","message":"boom"}"#,
        ];
        let responses = every_response_variant();
        assert_eq!(responses.len(), expected.len());
        let mut covered = [false; 8];
        for (envelope, wire) in responses.into_iter().zip(expected) {
            covered[response_variant_index(&envelope.response)] = true;
            assert_eq!(serde_json::to_string(&envelope).unwrap(), wire);
            let parsed: ResponseEnvelope = serde_json::from_str(wire).unwrap();
            assert_eq!(parsed, envelope);
        }
        assert!(
            covered.iter().all(|seen| *seen),
            "uncovered response variant"
        );
    }

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

    #[test]
    fn theme_and_curve_requests_use_the_documented_wire_shape() {
        use serde_json::json;

        let cases = [
            (
                Request::ListThemes,
                json!({"version": 1, "request": "list_themes"}),
            ),
            (
                Request::GetTheme {
                    id: String::from("nord"),
                },
                json!({"version": 1, "request": "get_theme", "id": "nord"}),
            ),
            (
                Request::DayCurve { monitor_id: None },
                json!({"version": 1, "request": "day_curve", "monitor_id": null}),
            ),
            (
                Request::SetCloudDimming { percent: 35 },
                json!({"version": 1, "request": "set_cloud_dimming", "percent": 35}),
            ),
            (
                Request::ClearExternalHold { monitor: None },
                json!({"version": 1, "request": "clear_external_hold", "monitor": null}),
            ),
            (
                Request::ClearExternalHold {
                    monitor: Some(String::from("desk")),
                },
                json!({"version": 1, "request": "clear_external_hold", "monitor": "desk"}),
            ),
        ];
        for (request, wire) in cases {
            let envelope = RequestEnvelope::new(request.clone());
            assert_eq!(serde_json::to_value(&envelope).unwrap(), wire);
            let parsed: RequestEnvelope = serde_json::from_value(wire).unwrap();
            assert_eq!(parsed.request, request);
        }
        // monitor_id may be omitted entirely.
        let parsed: RequestEnvelope =
            serde_json::from_str(r#"{"version":1,"request":"day_curve"}"#).unwrap();
        assert_eq!(parsed.request, Request::DayCurve { monitor_id: None });
        let parsed: RequestEnvelope =
            serde_json::from_str(r#"{"version":1,"request":"clear_external_hold"}"#).unwrap();
        assert_eq!(parsed.request, Request::ClearExternalHold { monitor: None });
        let parsed: RequestEnvelope =
            serde_json::from_str(r#"{"version":1,"request":"day_curve","monitor_id":"mi"}"#)
                .unwrap();
        assert_eq!(
            parsed.request,
            Request::DayCurve {
                monitor_id: Some(String::from("mi"))
            }
        );
    }

    #[test]
    fn theme_status_round_trips_with_every_documented_key() {
        let registry = crate::theme::Registry::builtin();
        let status = ThemeStatus::from_theme(registry.get("handheld").unwrap());
        let value = serde_json::to_value(ResponseEnvelope::theme(status.clone())).unwrap();
        assert_eq!(value["response"], "theme");
        let theme = &value["theme"];
        for key in [
            "name",
            "accent",
            "secondary_accent",
            "bg",
            "fg",
            "text_muted",
            "id",
            "family",
            "glyph_style",
            "palette",
            "widget",
        ] {
            assert!(!theme[key].is_null(), "missing {key}");
        }
        for key in crate::theme::Palette::KEYS {
            assert!(theme["palette"][key].as_str().unwrap().starts_with('#'));
        }
        for key in [
            "renderer",
            "font",
            "pixel_scale",
            "shades",
            "density",
            "border",
            "radius",
            "dither",
            "glow",
            "modules",
        ] {
            assert!(!theme["widget"][key].is_null(), "missing widget.{key}");
        }
        assert_eq!(theme["widget"]["glow"], serde_json::json!(0.0));
        let parsed: ResponseEnvelope = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.response, Response::Theme { theme: status });
    }

    #[test]
    fn old_status_json_without_theme_engine_fields_still_parses() {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)] // fields exist only to require the legacy keys during deserialization
        struct LegacyThemeStatus {
            name: String,
            accent: String,
            secondary_accent: String,
            bg: String,
            fg: String,
            text_muted: String,
        }
        // Shape emitted by a daemon before the theme engine (flat theme only).
        let old = r##"{
            "version": 1, "response": "status",
            "status": {
                "daemon_alive": true, "config_path": "/c.toml", "tick_seconds": 60,
                "dry_run": false, "suspended": false, "desktop_idle_dimmed": false,
                "suspend_until_epoch_s": null, "manual_override_active": false,
                "per_monitor_override_until_epoch_s": null, "global_override_percent": null,
                "global_override_until_epoch_s": null, "configured_monitors": 0,
                "stateful_monitors": 0, "weather": null, "monitors": [],
                "solar_elevation": null, "now_epoch_s": 1, "sunrise_epoch_s": null,
                "sunset_epoch_s": null,
                "theme": {"name": "Amber", "accent": "#ffb000", "secondary_accent": "#dc8c00",
                          "bg": "#0f0a05", "fg": "#ffd296", "text_muted": "#b47832"}
            }
        }"##;
        let parsed: ResponseEnvelope = serde_json::from_str(old).unwrap();
        let Response::Status { status } = parsed.response else {
            panic!("status expected");
        };
        assert!(status.temperature_unit.is_none());
        let theme = status.theme.unwrap();
        assert_eq!(theme.name, "Amber");
        assert!(theme.id.is_none() && theme.palette.is_none() && theme.widget.is_none());
        // Re-serialising does not invent the new keys.
        let value = serde_json::to_value(&theme).unwrap();
        assert!(value.get("palette").is_none() && value.get("id").is_none());

        // And an old client struct (flat fields only) ignores the new keys.
        let registry = crate::theme::Registry::builtin();
        let new =
            serde_json::to_string(&ThemeStatus::from_theme(registry.get("nord").unwrap())).unwrap();
        let legacy: LegacyThemeStatus = serde_json::from_str(&new).unwrap();
        assert_eq!(legacy.name, "Nord");
    }

    #[test]
    fn run_once_response_without_queued_field_parses_as_not_queued() {
        let raw = serde_json::json!({
            "tick_duration_ms": 12, "monitors_evaluated": 1, "writes_attempted": 1,
            "writes_skipped": 0, "writes_succeeded": 1, "writes_failed": 0
        });
        let parsed: super::RunOnceResponse = serde_json::from_value(raw).expect("legacy payload");
        assert!(!parsed.queued);
    }

    #[test]
    fn day_curve_response_round_trips() {
        let response = ResponseEnvelope::day_curve(DayCurveResponse {
            date: String::from("2026-10-13"),
            step_minutes: 10,
            generated_at_epoch_s: 5,
            weather_multiplier: Some(0.88),
            sun: DayCurveSun {
                dawn: None,
                sunrise: Some(1),
                solar_noon: Some(2),
                sunset: Some(3),
                dusk: None,
            },
            monitors: vec![DayCurveMonitor {
                monitor_id: String::from("mi-monitor"),
                min_pct: 1,
                max_pct: 50,
                samples: vec![DayCurveSample {
                    t: 0,
                    pct: 3,
                    pct_no_weather: 3,
                }],
                milestones: vec![DayCurveMilestone {
                    id: String::from("rise_50"),
                    label: String::from("Rise 50%"),
                    t: 0,
                    pct: 34,
                }],
            }],
            milestones_error: None,
        });
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["response"], "day_curve");
        assert!(value["day_curve"]["sun"]["dawn"].is_null());
        assert!(value["day_curve"].get("milestones_error").is_none());
        assert_eq!(response.kind_name(), "day_curve");
        let parsed: ResponseEnvelope = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, response);

        let themes = ResponseEnvelope::themes(vec![ThemeSummary::from_theme(
            crate::theme::Registry::builtin().get("dracula").unwrap(),
        )]);
        let value = serde_json::to_value(&themes).unwrap();
        assert_eq!(value["themes"][0]["family"], "terminal");
        assert_eq!(
            serde_json::from_value::<ResponseEnvelope>(value).unwrap(),
            themes
        );
    }

    #[test]
    fn old_monitor_status_json_without_skip_fields_still_parses() {
        let raw = serde_json::json!({
            "logical_id": "desk",
            "backend": "ddc",
            "enabled": true,
            "override_percent": null,
            "last_applied_percent": 50,
            "last_applied_at_epoch_s": 3,
            "backoff_until_epoch_s": null
        });
        let parsed: MonitorStatus = serde_json::from_value(raw).expect("legacy monitor status");
        assert!(!parsed.contested);
        assert_eq!(parsed.contested_until_epoch_s, None);
        assert_eq!(parsed.manual_deferred_until_epoch_s, None);
    }
}
