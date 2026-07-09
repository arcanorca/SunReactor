use std::io;
use std::path::PathBuf;

use crate::paths::PathError;

use super::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    Defaults,
    FilePresent,
}

#[derive(Debug, Clone)]
pub struct ConfigReport {
    pub path: PathBuf,
    pub source: ConfigSource,
    pub config: Config,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file already exists at {}", .0.display())]
    AlreadyExists(PathBuf),
    #[error("failed to read or write {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse {}: {source}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to render config TOML: {source}")]
    Serialize {
        #[source]
        source: toml::ser::Error,
    },
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("config validation failed{}", format_validation_errors(.0))]
    Validation(Vec<ValidationError>),
}

fn format_validation_errors(errors: &[ValidationError]) -> String {
    let mut message = String::new();
    for error in errors {
        message.push_str(&format!("\n- {error}"));
    }
    message
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {message}")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub(crate) fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}
