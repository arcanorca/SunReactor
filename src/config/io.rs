use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::paths;

use super::{Config, ConfigError, ConfigReport, ConfigSource, DEFAULT_CONFIG_TEMPLATE};

pub fn load() -> Result<ConfigReport, ConfigError> {
    let path = paths::config_file()?;
    load_from_path(&path)
}

pub fn parse_text(raw: &str, path: &Path) -> Result<Config, ConfigError> {
    parse_str(raw, path)
}

pub fn render(config: &Config) -> Result<String, ConfigError> {
    toml::to_string_pretty(config).map_err(|source| ConfigError::Serialize { source })
}

pub fn save(config: &Config) -> Result<PathBuf, ConfigError> {
    let path = paths::config_file()?;
    save_to_path(config, &path)
}

pub fn save_to_path(config: &Config, path: &Path) -> Result<PathBuf, ConfigError> {
    config.validate()?;
    let rendered = render(config)?;
    write_rendered_to_path(path, &rendered)
}

pub fn save_raw(raw: &str) -> Result<PathBuf, ConfigError> {
    let path = paths::config_file()?;
    save_raw_to_path(raw, &path)
}

pub fn save_raw_to_path(raw: &str, path: &Path) -> Result<PathBuf, ConfigError> {
    parse_str(raw, path)?;
    write_rendered_to_path(path, raw)
}

pub fn write_default() -> Result<PathBuf, ConfigError> {
    let path = paths::config_file()?;
    write_default_to(&path)
}

pub(crate) fn load_from_path(path: &Path) -> Result<ConfigReport, ConfigError> {
    if path.exists() {
        let raw = fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let warnings = super::compat::compatibility_warnings(&raw);
        let config = parse_str(&raw, path)?;

        Ok(ConfigReport {
            path: path.to_path_buf(),
            source: ConfigSource::FilePresent,
            config,
            warnings,
        })
    } else {
        let config = Config::default();
        config.validate()?;

        Ok(ConfigReport {
            path: path.to_path_buf(),
            source: ConfigSource::Defaults,
            config,
            warnings: Vec::new(),
        })
    }
}

pub(crate) fn parse_str(raw: &str, path: &Path) -> Result<Config, ConfigError> {
    super::compat::reject_removed_legacy_fields(raw)?;
    let config: Config = toml::from_str(raw).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    config.validate()?;
    Ok(config)
}

pub(crate) fn write_default_to(path: &Path) -> Result<PathBuf, ConfigError> {
    if path.exists() {
        return Err(ConfigError::AlreadyExists(path.to_path_buf()));
    }
    write_rendered_to_path(path, DEFAULT_CONFIG_TEMPLATE)
}

fn write_rendered_to_path(path: &Path, rendered: &str) -> Result<PathBuf, ConfigError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut temp_file = tempfile::Builder::new()
        .prefix("sunreactor_config_")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|source| ConfigError::Io {
            path: parent.to_path_buf(),
            source,
        })?;

    use std::io::Write;
    temp_file
        .write_all(rendered.as_bytes())
        .map_err(|source| ConfigError::Io {
            path: temp_file.path().to_path_buf(),
            source,
        })?;
    temp_file
        .as_file()
        .sync_all()
        .map_err(|source| ConfigError::Io {
            path: temp_file.path().to_path_buf(),
            source,
        })?;

    temp_file.persist(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source: source.error,
    })?;

    sync_parent_dir(path)?;
    Ok(path.to_path_buf())
}

fn sync_parent_dir(path: &Path) -> Result<(), ConfigError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let directory = File::open(parent).map_err(|source| ConfigError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    directory.sync_all().map_err(|source| ConfigError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    Ok(())
}
