pub mod client;
pub mod command;
pub mod parser;
pub mod version;

pub(crate) use client::{DdcutilClient, DdcutilError, DdcutilTimeouts};

#[derive(Debug, Clone)]
pub(crate) struct DdcutilCapabilities {
    pub version_string: String,
    pub supports_noconfig: bool,
    pub supports_noverify: bool,
    pub supports_terse: bool,
    pub supports_brief: bool,
}
