use crate::backends::{BackendKind, FailureKind};
use crate::paths::PathError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

pub(crate) const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default)]
pub struct DaemonState {
    pub last_applied_percent: Option<u8>,
    pub last_reason: Option<String>,
    pub discovered_targets: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeState {
    pub schema_version: u32,
    pub monitors: BTreeMap<String, MonitorRuntimeState>,
    pub suspend_indefinite: bool,
    pub suspend_until_epoch_s: Option<u64>,
    pub desktop_idle_dimmed: bool,
    pub manual_override: Option<ManualOverrideState>,
    pub weather: Option<WeatherSnapshotMetadata>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveControlState {
    pub suspended: bool,
    pub suspend_indefinite: bool,
    pub suspend_until_epoch_s: Option<u64>,
    pub desktop_idle_dimmed: bool,
    pub manual_override_active: bool,
    pub per_monitor_override_until_epoch_s: Option<u64>,
    pub global_override_percent: Option<u8>,
    pub global_override_until_epoch_s: Option<u64>,
    pub(crate) per_monitor_overrides: BTreeMap<String, u8>,
}
impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            monitors: BTreeMap::new(),
            suspend_indefinite: false,
            suspend_until_epoch_s: None,
            desktop_idle_dimmed: false,
            manual_override: None,
            weather: None,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MonitorRuntimeState {
    pub last_applied_percent: Option<u8>,
    pub last_applied_at_epoch_s: Option<u64>,
    pub backoff: Option<FailureBackoffState>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManualOverrideState {
    pub global_percent: Option<u8>,
    pub global_expires_at_epoch_s: Option<u64>,
    pub targets: BTreeMap<String, u8>,
    pub expires_at_epoch_s: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub dt_epoch_s: u64,
    pub cloud_cover_percent: u8,
    pub temperature: f32,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherSnapshotMetadata {
    pub provider: String,
    pub observed_at_epoch_s: u64,
    pub cloud_cover_percent: Option<u8>,
    pub smoothed_cloud_cover_percent: Option<u8>,
    pub temperature: Option<f32>,
    pub forecast: Vec<ForecastPoint>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FailureBackoffState {
    pub backend: BackendKind,
    pub failure_kind: FailureKind,
    pub consecutive_failures: u32,
    pub suppress_until_epoch_s: Option<u64>,
}
impl Default for FailureBackoffState {
    fn default() -> Self {
        Self {
            backend: BackendKind::Backlight,
            failure_kind: FailureKind::Transient,
            consecutive_failures: 0,
            suppress_until_epoch_s: None,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("failed to access {}: {}", path.display(), source)]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to serialize runtime state: {source}")]
    Serialize {
        #[source]
        source: serde_json::Error,
    },
}
