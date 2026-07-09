use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::DirBuilderExt;
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

        if let Ok(state) = serde_json::from_slice::<RuntimeState>(&raw) {
            Ok(state.normalized())
        } else {
            repair_corrupt_state_file(path, &Self::default());
            Ok(Self::default())
        }
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
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(parent).map_err(|source| StateError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        let _ = fs::set_permissions(parent, std::os::unix::fs::PermissionsExt::from_mode(0o700));
    }

    let bytes =
        serde_json::to_vec_pretty(state).map_err(|source| StateError::Serialize { source })?;

    let mut temp_file = tempfile::Builder::new()
        .prefix("sunreactor_state_")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|source| StateError::Io {
            path: parent.to_path_buf(),
            source,
        })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(0o600));
    }

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

pub(crate) fn repair_corrupt_state_file(path: &Path, default_state: &RuntimeState) {
    if !path.exists() {
        return;
    }

    let backup_path = corrupt_backup_path(path);
    if fs::rename(path, &backup_path).is_err() {
        return;
    }

    let _ = atomic_write_json(path, default_state);
}

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
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::backends::{BackendKind, FailureKind};
    use crate::state::transitions::{limit_step_size, should_skip_hysteresis};

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
                observed_at_epoch_s: 1_950,
                cloud_cover_percent: Some(81),
                smoothed_cloud_cover_percent: Some(79),
                temperature: Some(0.0),
                forecast: vec![],
            }),
            ..RuntimeState::default()
        };
        state.monitors.insert(
            String::from("desk"),
            MonitorRuntimeState {
                last_applied_percent: Some(58),
                last_applied_at_epoch_s: Some(1_900),
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
