use std::time::Duration;
use crate::backends::{ProcessRunner, CommandError, CommandOutput};

#[derive(Debug, Clone)]
pub struct DdcutilProfile {
    pub version_string: String,
    pub supports_noconfig: bool,
    pub supports_noverify: bool,
    pub supports_terse: bool,
    pub supports_brief: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DdcutilTimeouts {
    pub detect: Duration,
    pub capabilities: Duration,
    pub getvcp: Duration,
    pub setvcp: Duration,
}

impl Default for DdcutilTimeouts {
    fn default() -> Self {
        Self {
            detect: Duration::from_secs(12),
            capabilities: Duration::from_secs(15),
            getvcp: Duration::from_secs(4),
            setvcp: Duration::from_secs(10),
        }
    }
}

pub struct DdcutilClient<R: ProcessRunner> {
    runner: R,
    profile: DdcutilProfile,
    timeouts: DdcutilTimeouts,
}

#[derive(Debug, thiserror::Error)]
pub enum DdcutilError {
    #[error("command execution failed: {0}")]
    Command(#[from] CommandError),
    #[error("ddcutil returned non-zero exit code {0}: {1}")]
    ExecutionFailed(i32, String),
    #[error("ddcutil was terminated by a signal")]
    Terminated,
    #[error("parse error: {0}")]
    Parse(String),
}

impl<R: ProcessRunner> DdcutilClient<R> {
    pub fn new(runner: R, profile: DdcutilProfile, timeouts: DdcutilTimeouts) -> Self {
        Self { runner, profile, timeouts }
    }

    pub fn probe_profile(runner: &R) -> DdcutilProfile {
        let caps = crate::ddcutil::command::probe_capabilities(runner);
        DdcutilProfile {
            version_string: caps.version_string,
            supports_noconfig: caps.supports_noconfig,
            supports_noverify: caps.supports_noverify,
            supports_terse: caps.supports_terse,
            supports_brief: caps.supports_brief,
        }
    }

    pub fn profile(&self) -> &DdcutilProfile {
        &self.profile
    }

    pub fn runner(&self) -> &R {
        &self.runner
    }

    pub fn timeouts(&self) -> &DdcutilTimeouts {
        &self.timeouts
    }

    fn execute(&self, args: &[String], timeout: Duration) -> Result<CommandOutput, DdcutilError> {
        let output = self.runner.run("ddcutil", args, timeout)?;
        if !output.success() {
            if let Some(code) = output.exit_code {
                return Err(DdcutilError::ExecutionFailed(code, output.stderr.clone()));
            } else {
                return Err(DdcutilError::Terminated);
            }
        }
        Ok(output)
    }

    pub fn detect(&self) -> Result<Vec<crate::discovery::model::RawDdcMonitor>, DdcutilError> {
        let caps = crate::ddcutil::DdcutilCapabilities {
            version_string: self.profile.version_string.clone(),
            supports_noconfig: self.profile.supports_noconfig,
            supports_noverify: self.profile.supports_noverify,
            supports_terse: self.profile.supports_terse,
            supports_brief: self.profile.supports_brief,
        };
        let args = crate::ddcutil::command::build_detect_args(&caps);
        let output = self.execute(&args, self.timeouts.detect)?;
        Ok(crate::ddcutil::parser::parse_ddc_detect(&output.stdout))
    }

    pub fn capabilities(&self, display: u32) -> Result<bool, DdcutilError> {
        let caps = crate::ddcutil::DdcutilCapabilities {
            version_string: self.profile.version_string.clone(),
            supports_noconfig: self.profile.supports_noconfig,
            supports_noverify: self.profile.supports_noverify,
            supports_terse: self.profile.supports_terse,
            supports_brief: self.profile.supports_brief,
        };
        let args = crate::ddcutil::command::build_capabilities_args(&caps, display);
        let output = self.execute(&args, self.timeouts.capabilities)?;
        Ok(crate::ddcutil::parser::parse_brightness_vcp_support(&output.stdout))
    }

    pub fn getvcp_brightness(&self, display: u32) -> Result<bool, DdcutilError> {
        let caps = crate::ddcutil::DdcutilCapabilities {
            version_string: self.profile.version_string.clone(),
            supports_noconfig: self.profile.supports_noconfig,
            supports_noverify: self.profile.supports_noverify,
            supports_terse: self.profile.supports_terse,
            supports_brief: self.profile.supports_brief,
        };
        let args = crate::ddcutil::command::build_getvcp_args(&caps, display, "10");
        let output = self.execute(&args, self.timeouts.getvcp)?;
        Ok(crate::ddcutil::parser::parse_getvcp_brightness(&output.stdout))
    }

    pub fn setvcp(&self, selection_args: &[String], vcp: &str, value: &str) -> Result<(), DdcutilError> {
        let caps = crate::ddcutil::DdcutilCapabilities {
            version_string: self.profile.version_string.clone(),
            supports_noconfig: self.profile.supports_noconfig,
            supports_noverify: self.profile.supports_noverify,
            supports_terse: self.profile.supports_terse,
            supports_brief: self.profile.supports_brief,
        };
        let args = crate::ddcutil::command::build_setvcp_args(&caps, selection_args, vcp, value);
        self.execute(&args, self.timeouts.setvcp)?;
        Ok(())
    }
}
