pub mod client;
pub mod command;
pub mod parser;
pub mod version;

pub use client::{DdcutilClient, DdcutilProfile, DdcutilTimeouts, DdcutilError};

// We keep DdcutilCapabilities around because command.rs uses it, but we can deprecate it later.
#[derive(Debug, Clone)]
pub(crate) struct DdcutilCapabilities {
    pub version_string: String,
    pub supports_noconfig: bool,
    pub supports_noverify: bool,
    pub supports_terse: bool,
    pub supports_brief: bool,
}

