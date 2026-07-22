use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::backends::BackendKind;
use crate::discovery::DiscoveryReport;

use super::{parse_text, save_raw_to_path, ConfigError, MonitorConfig, DEFAULT_CONFIG_TEMPLATE};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveredApplyResult {
    pub path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub added_monitor_ids: Vec<String>,
    pub unchanged: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveredApplyError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("failed to access {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("configuration health verification failed; the previous config was restored: {0}")]
    Verification(String),
}

/// Adds viable discovery candidates and verifies the daemon before committing
/// the transaction. Read-only discovery remains a separate operation.
pub fn apply_discovered_transaction<F>(
    path: &Path,
    report: &DiscoveryReport,
    mut verify: F,
) -> Result<DiscoveredApplyResult, DiscoveredApplyError>
where
    F: FnMut() -> Result<(), String>,
{
    let existed = path.exists();
    let original = if existed {
        fs::read_to_string(path).map_err(|source| DiscoveredApplyError::Io {
            path: path.to_path_buf(),
            source,
        })?
    } else {
        DEFAULT_CONFIG_TEMPLATE.to_owned()
    };
    let current = parse_text(&original, path)?;
    let candidates = parse_text(&report.config_snippet, Path::new("<discovery>"))?.monitors;

    let existing_identities = current
        .monitors
        .iter()
        .map(monitor_identity)
        .collect::<HashSet<_>>();
    let mut used_logical_ids = current
        .monitors
        .iter()
        .map(|monitor| monitor.logical_id.clone())
        .collect::<HashSet<_>>();
    let mut selected = Vec::new();
    let mut selected_identities = HashSet::new();

    for mut candidate in candidates {
        let identity = monitor_identity(&candidate);
        if existing_identities.contains(&identity) || !selected_identities.insert(identity) {
            continue;
        }
        candidate.logical_id = allocate_logical_id(&candidate.logical_id, &mut used_logical_ids);
        selected.push(candidate);
    }

    if selected.is_empty() {
        return Ok(DiscoveredApplyResult {
            path: path.to_path_buf(),
            backup_path: None,
            added_monitor_ids: Vec::new(),
            unchanged: true,
        });
    }

    let appended = render_monitor_blocks(&selected)?;
    let updated = format!(
        "{}\n\n# Added by `sunreactorctl discover --apply`.\n{}",
        original.trim_end(),
        appended.trim()
    );
    parse_text(&updated, path)?;

    let backup_path = existed.then(|| backup_path(path));
    if let Some(backup) = &backup_path {
        save_raw_to_path(&original, backup)?;
    }
    save_raw_to_path(&updated, path)?;

    if let Err(message) = verify() {
        if existed {
            save_raw_to_path(&original, path)?;
        } else {
            fs::remove_file(path).map_err(|source| DiscoveredApplyError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let rollback_message = verify().err();
        let detail = rollback_message.map_or(message.clone(), |rollback_error| {
            format!("{message}; restoring the daemon state also failed: {rollback_error}")
        });
        return Err(DiscoveredApplyError::Verification(detail));
    }

    Ok(DiscoveredApplyResult {
        path: path.to_path_buf(),
        backup_path,
        added_monitor_ids: selected
            .into_iter()
            .map(|monitor| monitor.logical_id)
            .collect(),
        unchanged: false,
    })
}

#[derive(Serialize)]
struct MonitorBlocks<'a> {
    monitors: &'a [MonitorConfig],
}

fn render_monitor_blocks(monitors: &[MonitorConfig]) -> Result<String, ConfigError> {
    toml::to_string_pretty(&MonitorBlocks { monitors })
        .map_err(|source| ConfigError::Serialize { source })
}

fn monitor_identity(monitor: &MonitorConfig) -> String {
    let selector = &monitor.selector;
    match monitor.backend {
        BackendKind::Backlight => format!(
            "backlight:{}",
            normalize(selector.sysfs_path.as_deref()).unwrap_or(&monitor.logical_id)
        ),
        BackendKind::Ddc => {
            if let Some(edid) = normalize(selector.edid.as_deref()) {
                return format!("ddc:edid:{edid}");
            }
            if let Some(serial) = normalize(selector.serial.as_deref()) {
                return format!(
                    "ddc:serial:{}:{}",
                    normalize(selector.model.as_deref()).unwrap_or(""),
                    serial
                );
            }
            if let Some(bus) = selector.ddc_bus {
                return format!(
                    "ddc:bus:{}:{}",
                    bus,
                    normalize(selector.model.as_deref()).unwrap_or("")
                );
            }
            format!(
                "ddc:connector:{}:{}",
                normalize(selector.connector.as_deref()).unwrap_or(""),
                normalize(selector.model.as_deref()).unwrap_or(&monitor.logical_id)
            )
        }
    }
}

fn normalize(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn allocate_logical_id(base: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.to_owned()) {
        return base.to_owned();
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must find an unused logical id")
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".bak");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::apply_discovered_transaction;
    use crate::config::{load_from_path, DEFAULT_CONFIG_TEMPLATE};
    use crate::discovery::{
        BackendStatus, BackendStatusKind, DiscoveryBackends, DiscoveryReport, DiscoverySummary,
    };
    use std::fs;

    #[test]
    fn apply_is_idempotent_and_preserves_manual_text() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        let original = DEFAULT_CONFIG_TEMPLATE
            .replace("[weather]", "# user's comment must survive\n[weather]");
        fs::write(&path, original).expect("seed config");
        let report = report_with_snippet(backlight_snippet());

        let first = apply_discovered_transaction(&path, &report, || Ok(())).expect("first apply");
        let second = apply_discovered_transaction(&path, &report, || Ok(())).expect("second apply");

        assert_eq!(first.added_monitor_ids, vec!["internal"]);
        assert!(first
            .backup_path
            .as_ref()
            .is_some_and(|backup| backup.exists()));
        assert!(second.unchanged);
        assert_eq!(
            load_from_path(&path)
                .expect("valid config")
                .config
                .monitors
                .len(),
            1
        );
        assert!(fs::read_to_string(path)
            .expect("read")
            .contains("user's comment must survive"));
    }

    #[test]
    fn verification_failure_restores_original_config() {
        use std::cell::Cell;

        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        fs::write(&path, DEFAULT_CONFIG_TEMPLATE).expect("seed config");
        let original = fs::read_to_string(&path).expect("read original");

        let verification_attempts = Cell::new(0);
        let error =
            apply_discovered_transaction(&path, &report_with_snippet(backlight_snippet()), || {
                verification_attempts.set(verification_attempts.get() + 1);
                Err(String::from("daemon rejected reload"))
            })
            .expect_err("verification should fail");

        assert!(error.to_string().contains("previous config was restored"));
        assert_eq!(fs::read_to_string(path).expect("read restored"), original);
        assert_eq!(verification_attempts.get(), 2);
    }

    #[test]
    fn invalid_discovery_snippet_is_never_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        fs::write(&path, DEFAULT_CONFIG_TEMPLATE).expect("seed config");
        let original = fs::read_to_string(&path).expect("read original");
        let invalid = "[[monitors]]\nlogical_id = \"bad\"\nbackend = \"backlight\"\nenabled = true\nmin_pct = 90\nmax_pct = 10\nsysfs_path = \"/sys/class/backlight/test\"\n";

        let result = apply_discovered_transaction(&path, &report_with_snippet(invalid), || Ok(()));

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(path).expect("read unchanged"), original);
    }

    fn backlight_snippet() -> &'static str {
        "[[monitors]]\nlogical_id = \"internal\"\nbackend = \"backlight\"\nenabled = true\nmin_pct = 15\nmax_pct = 60\ngain = 1.0\nsysfs_path = \"/sys/class/backlight/intel_backlight\"\n"
    }

    fn report_with_snippet(snippet: &str) -> DiscoveryReport {
        let status = BackendStatus {
            backend: String::from("test"),
            status: BackendStatusKind::Ok,
            available: true,
            message: String::new(),
            guidance: None,
        };
        DiscoveryReport {
            summary: DiscoverySummary {
                ddc_monitors: 0,
                backlight_devices: 1,
                viable_targets: 1,
            },
            backends: DiscoveryBackends {
                ddcutil: status.clone(),
                brightnessctl: status.clone(),
                sysfs: status,
            },
            ddc_monitors: Vec::new(),
            backlight_devices: Vec::new(),
            notes: Vec::new(),
            config_snippet: snippet.to_owned(),
        }
    }
}
