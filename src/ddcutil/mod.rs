pub mod command;
pub mod parser;
pub mod version;

use crate::backends::ProcessRunner;
use std::sync::OnceLock;

thread_local! {
    static DDCUTIL_CAPABILITIES: std::cell::OnceCell<DdcutilCapabilities> = std::cell::OnceCell::new();
}

#[derive(Debug, Clone)]
pub(crate) struct DdcutilCapabilities {
    pub version_string: String,
    pub supports_noconfig: bool,
    pub supports_noverify: bool,
    pub supports_terse: bool,
    pub supports_brief: bool,
}

pub(crate) struct DdcContext<'a, R: ProcessRunner> {
    pub runner: &'a R,
}

impl<'a, R: ProcessRunner> DdcContext<'a, R> {
    pub(crate) fn new(runner: &'a R) -> Self {
        Self { runner }
    }

    pub(crate) fn capabilities(&self) -> DdcutilCapabilities {
        #[cfg(test)]
        return command::probe_capabilities(self.runner);

        #[cfg(not(test))]
        DDCUTIL_CAPABILITIES.with(|caps| {
            caps.get_or_init(|| command::probe_capabilities(self.runner)).clone()
        })
    }
}
