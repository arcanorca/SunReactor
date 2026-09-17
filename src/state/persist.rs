use std::fs;
#[cfg(target_os = "linux")]
use std::fs::File;
use std::io;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;
use crate::state::types::{RuntimeState, StateError};

pub(crate) const RUNTIME_STATE_FILE_NAME: &str = "runtime-state.json";

impl RuntimeState {
    pub fn load() -> Result<Self, StateError> {
        let path = paths::state_file()?;
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Result<Self, StateError> {
        let raw = match fs::read(path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(StateError::Io {
                    path: path.to_path_buf(),
                    source: error,
                });
            }
        };

        let Ok(document) = serde_json::from_slice::<serde_json::Value>(&raw) else {
            repair_corrupt_state_file(path, &Self::default());
            return Ok(Self::default());
        };

        let Some(schema_value) = document.get("schema_version") else {
            // schema_version has existed since the first persisted state
            // format. An absent version is therefore not a legacy format.
            repair_corrupt_state_file(path, &Self::default());
            return Ok(Self::default());
        };
        let Some(found) = schema_value.as_u64() else {
            repair_corrupt_state_file(path, &Self::default());
            return Ok(Self::default());
        };
        if found != u64::from(crate::state::types::STATE_SCHEMA_VERSION) {
            return Err(StateError::UnsupportedSchemaVersion {
                path: path.to_path_buf(),
                found,
                supported: crate::state::types::STATE_SCHEMA_VERSION,
            });
        }

        let Ok(state) = serde_json::from_value::<RuntimeState>(document) else {
            repair_corrupt_state_file(path, &Self::default());
            return Ok(Self::default());
        };
        Ok(state.normalized())
    }

    pub fn save(&self) -> Result<PathBuf, StateError> {
        let path = paths::state_file()?;
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<PathBuf, StateError> {
        atomic_write_json(path, &self.clone().normalized())?;
        Ok(path.to_path_buf())
    }

    pub(crate) fn normalized_for_persistence(&self) -> Self {
        self.clone().normalized()
    }
}

pub(crate) fn atomic_write_json(path: &Path, state: &RuntimeState) -> Result<(), StateError> {
    let canonical_state = state.clone().normalized();
    let bytes = serde_json::to_vec_pretty(&canonical_state)
        .map_err(|source| StateError::Serialize { source })?;

    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::atomic_write_file(path, &bytes).map_err(|source| StateError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700);
            builder.create(parent).map_err(|source| StateError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
            let _ =
                fs::set_permissions(parent, std::os::unix::fs::PermissionsExt::from_mode(0o700));
        }

        let mut temp_file = tempfile::Builder::new()
            .prefix("sunreactor_state_")
            .suffix(".tmp")
            .tempfile_in(parent)
            .map_err(|source| StateError::Io {
                path: parent.to_path_buf(),
                source,
            })?;

        let _ = fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(0o600));

        temp_file
            .write_all(&bytes)
            .map_err(|source| StateError::Io {
                path: temp_file.path().to_path_buf(),
                source,
            })?;
        temp_file
            .write_all(b"\n")
            .map_err(|source| StateError::Io {
                path: temp_file.path().to_path_buf(),
                source,
            })?;
        temp_file
            .as_file()
            .sync_all()
            .map_err(|source| StateError::Io {
                path: temp_file.path().to_path_buf(),
                source,
            })?;

        temp_file.persist(path).map_err(|source| StateError::Io {
            path: path.to_path_buf(),
            source: source.error,
        })?;

        sync_parent_dir(path)?;

        Ok(())
    }
}

pub(crate) fn repair_corrupt_state_file(path: &Path, default_state: &RuntimeState) {
    if !path.exists() {
        return;
    }

    let mut backup_path = corrupt_backup_path(path);
    loop {
        match fs::rename(path, &backup_path) {
            Ok(()) => break,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                backup_path = corrupt_backup_path(path);
            }
            Err(_) => return,
        }
    }

    let _ = atomic_write_json(path, default_state);
}

#[cfg(target_os = "linux")]
pub(crate) fn sync_parent_dir(path: &Path) -> Result<(), StateError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let directory = File::open(parent).map_err(|source| StateError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    directory.sync_all().map_err(|source| StateError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub(crate) fn corrupt_backup_path(path: &Path) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = format!(
        "{}.corrupt-{unique}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(RUNTIME_STATE_FILE_NAME)
    );
    path.with_file_name(file_name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::backends::{BackendKind, FailureKind};

    use crate::state::persist::RUNTIME_STATE_FILE_NAME;
    use crate::state::{
        FailureBackoffState, ManualOverrideState, MonitorRuntimeState, RuntimeState,
        WeatherSnapshotMetadata,
    };

    #[test]
    fn cold_start_returns_default_state_when_file_is_missing() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);

        let state = RuntimeState::load_from_path(&path).expect("missing state file should load");

        assert_eq!(state, RuntimeState::default());
    }

    #[test]
    fn loads_valid_persisted_state() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);
        let mut state = RuntimeState {
            suspend_until_epoch_s: Some(2_000),
            manual_override: Some(ManualOverrideState {
                global_percent: Some(48),
                global_expires_at_epoch_s: Some(2_050),
                targets: BTreeMap::from([
                    (String::from("desk"), 64),
                    (String::from("internal"), 33),
                ]),
                expires_at_epoch_s: Some(2_100),
            }),
            weather: Some(WeatherSnapshotMetadata {
                provider: String::from("openweather"),
                fetched_at_epoch_s: 1_950,
                valid_at_epoch_s: 1_900,
                source_kind: crate::weather::WeatherSourceKind::Forecast,
                cloud_cover_percent: Some(81),
                smoothed_cloud_cover_percent: Some(79),
                temperature: Some(0.0),
                condition: crate::weather::WeatherCondition::Cloudy,
                condition_description: Some(String::from("overcast clouds")),
                day_phase: Some(crate::weather::WeatherDayPhase::Day),
                forecast: vec![],
                ..Default::default()
            }),
            ..RuntimeState::default()
        };
        state.monitors.insert(
            String::from("desk"),
            MonitorRuntimeState {
                last_applied_percent: Some(58),
                last_applied_at_epoch_s: Some(1_900),
                last_integrity_check_at_epoch_s: None,
                backoff: Some(FailureBackoffState {
                    backend: BackendKind::Ddc,
                    failure_kind: FailureKind::Persistent,
                    consecutive_failures: 2,
                    suppress_until_epoch_s: Some(1_930),
                }),
            },
        );

        state.save_to_path(&path).expect("state should save");
        let loaded = RuntimeState::load_from_path(&path).expect("state should load");

        assert_eq!(loaded, state);
    }

    #[test]
    fn legacy_weather_timestamp_maps_to_fetch_time_and_unknown_validity() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);
        let legacy = br#"{
  "schema_version": 1,
  "monitors": {},
  "weather": {
    "provider": "openweather",
    "observed_at_epoch_s": 1800000000,
    "cloud_cover_percent": 55,
    "smoothed_cloud_cover_percent": 50,
    "temperature": 12.0,
    "forecast": []
  }
}
"#;
        fs::write(&path, legacy).expect("legacy state should write");

        let state = RuntimeState::load_from_path(&path).expect("legacy state should load");
        let weather = state
            .weather
            .as_ref()
            .expect("legacy weather should survive load");
        assert_eq!(weather.fetched_at_epoch_s, 1_800_000_000);
        assert_eq!(weather.valid_at_epoch_s, 0);
        assert_eq!(
            weather.source_kind,
            crate::weather::WeatherSourceKind::Unknown
        );
        assert_eq!(weather.condition, crate::weather::WeatherCondition::Unknown);
        assert_eq!(weather.condition_description, None);
        assert_eq!(weather.day_phase, None);

        state.save_to_path(&path).expect("legacy state should save");
        let canonical = fs::read_to_string(&path).expect("canonical state should read");
        assert!(canonical.contains("fetched_at_epoch_s"));
        assert!(!canonical.contains("observed_at_epoch_s"));
        let reloaded = RuntimeState::load_from_path(&path).expect("canonical state should load");
        assert_eq!(
            reloaded
                .weather
                .expect("weather should remain")
                .fetched_at_epoch_s,
            1_800_000_000
        );
    }

    #[test]
    fn future_schema_is_rejected_without_touching_the_file() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);
        let original = br#"{"schema_version":2,"future":{"new":true},"monitors":{}}
"#;
        fs::write(&path, original).expect("future state should write");

        let error = RuntimeState::load_from_path(&path).expect_err("future state must reject");

        assert!(matches!(
            error,
            crate::state::StateError::UnsupportedSchemaVersion {
                found: 2,
                supported: 1,
                ..
            }
        ));
        assert_eq!(fs::read(&path).expect("state should remain"), original);
        assert_eq!(corrupt_backup_count(temp.path()), 0);
    }

    #[test]
    fn missing_schema_is_treated_as_corrupt_not_as_current_state() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);
        let original = br#"{"monitors":{}}
"#;
        fs::write(&path, original).expect("unversioned state should write");

        assert_eq!(
            RuntimeState::load_from_path(&path).expect("unversioned state should recover"),
            RuntimeState::default()
        );
        assert_eq!(
            fs::read(first_corrupt_backup(temp.path())).expect("backup should be readable"),
            original
        );
        assert_eq!(
            serde_json::from_slice::<RuntimeState>(&fs::read(&path).expect("default should exist"))
                .expect("repaired state should be valid")
                .schema_version,
            crate::state::types::STATE_SCHEMA_VERSION
        );
    }

    #[test]
    fn corrupted_state_file_falls_back_to_default_and_repairs_file() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);
        fs::create_dir_all(temp.path()).expect("temp dir should exist");
        fs::write(&path, "{\"schema_version\":1,\"monitors\":")
            .expect("corrupt state should write");

        let state = RuntimeState::load_from_path(&path).expect("corrupt state should recover");

        assert_eq!(state, RuntimeState::default());
        let repaired = RuntimeState::load_from_path(&path).expect("repaired state should load");
        assert_eq!(repaired, RuntimeState::default());

        let backup_count = fs::read_dir(temp.path())
            .expect("temp dir should be readable")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("runtime-state.json.corrupt-")
            })
            .count();
        assert_eq!(backup_count, 1);
    }

    #[test]
    fn save_canonicalizes_schema_version_and_round_trips_complete_json() {
        let temp = TempDir::new();
        let path = temp.path().join(RUNTIME_STATE_FILE_NAME);
        let state = RuntimeState {
            schema_version: 999,
            suspend_until_epoch_s: Some(2_000),
            ..RuntimeState::default()
        };

        state.save_to_path(&path).expect("state should save");

        let bytes = fs::read(&path).expect("saved state should exist");
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("json should be complete");
        assert_eq!(
            value["schema_version"],
            crate::state::types::STATE_SCHEMA_VERSION
        );
        assert_eq!(
            RuntimeState::load_from_path(&path).expect("state should load"),
            state.normalized()
        );
    }

    fn corrupt_backup_count(path: &Path) -> usize {
        fs::read_dir(path)
            .expect("temp dir should be readable")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("runtime-state.json.corrupt-")
            })
            .count()
    }

    fn first_corrupt_backup(path: &Path) -> PathBuf {
        fs::read_dir(path)
            .expect("temp dir should be readable")
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("runtime-state.json.corrupt-")
            })
            .expect("corrupt backup should exist")
            .path()
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
            let path = std::env::temp_dir().join(format!("sunreactor-state-test-{unique}"));
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
