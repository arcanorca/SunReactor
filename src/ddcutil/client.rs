use std::time::Duration;

use crate::backends::{CommandError, CommandOutput, ProcessRunner};

use super::parser::{self, BrightnessValue, ParseError};
use super::{command, DdcutilCapabilities};

#[derive(Debug, Clone, Default)]
pub(crate) struct DdcutilProfile {
    pub version_string: String,
    pub supports_noconfig: bool,
    pub supports_noverify: bool,
    pub supports_terse: bool,
    pub supports_brief: bool,
}

impl From<DdcutilCapabilities> for DdcutilProfile {
    fn from(capabilities: DdcutilCapabilities) -> Self {
        Self {
            version_string: capabilities.version_string,
            supports_noconfig: capabilities.supports_noconfig,
            supports_noverify: capabilities.supports_noverify,
            supports_terse: capabilities.supports_terse,
            supports_brief: capabilities.supports_brief,
        }
    }
}

impl From<&DdcutilProfile> for DdcutilCapabilities {
    fn from(profile: &DdcutilProfile) -> Self {
        Self {
            version_string: profile.version_string.clone(),
            supports_noconfig: profile.supports_noconfig,
            supports_noverify: profile.supports_noverify,
            supports_terse: profile.supports_terse,
            supports_brief: profile.supports_brief,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DdcutilTimeouts {
    pub detect: Duration,
    pub capabilities: Duration,
    pub getvcp: Duration,
    pub setvcp: Duration,
}

impl Default for DdcutilTimeouts {
    fn default() -> Self {
        Self {
            detect: Duration::from_secs(25),
            capabilities: Duration::from_secs(12),
            getvcp: Duration::from_secs(8),
            setvcp: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DdcutilError {
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error("ddcutil failed: {detail}")]
    Failed {
        detail: String,
        output: CommandOutput,
    },
    #[error(transparent)]
    Parse(#[from] ParseError),
}

pub(crate) struct DdcutilClient<'a, R: ProcessRunner> {
    runner: &'a R,
    profile: DdcutilProfile,
    timeouts: DdcutilTimeouts,
}

impl<'a, R: ProcessRunner> DdcutilClient<'a, R> {
    pub(crate) fn probe(runner: &'a R, timeouts: DdcutilTimeouts) -> Self {
        Self {
            runner,
            profile: command::probe_capabilities(runner).into(),
            timeouts,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_profile(
        runner: &'a R,
        profile: DdcutilProfile,
        timeouts: DdcutilTimeouts,
    ) -> Self {
        Self {
            runner,
            profile,
            timeouts,
        }
    }

    pub(crate) fn detect(
        &self,
    ) -> Result<Vec<crate::discovery::model::RawDdcMonitor>, DdcutilError> {
        let args = command::build_detect_args(&(&self.profile).into());
        let output = self.execute(&args, self.timeouts.detect)?;
        Ok(parser::parse_ddc_detect(&output.stdout))
    }

    pub(crate) fn capabilities(&self, display: u32) -> Result<bool, DdcutilError> {
        let args = command::build_capabilities_args(&(&self.profile).into(), display);
        let output = self.execute(&args, self.timeouts.capabilities)?;
        Ok(parser::parse_brightness_vcp_support(&output.stdout))
    }

    pub(crate) fn get_brightness(&self, display: u32) -> Result<BrightnessValue, DdcutilError> {
        let args = command::build_getvcp_args(&(&self.profile).into(), display, "10");
        let output = self.execute(&args, self.timeouts.getvcp)?;
        Ok(parser::parse_getvcp_brightness(&output.stdout)?)
    }

    pub(crate) fn set_brightness(
        &self,
        selection_args: &[String],
        percent: u8,
    ) -> Result<CommandOutput, DdcutilError> {
        let args = command::build_setvcp_args(
            &(&self.profile).into(),
            selection_args,
            "10",
            &percent.to_string(),
        );
        self.execute(&args, self.timeouts.setvcp)
    }

    fn execute(&self, args: &[String], timeout: Duration) -> Result<CommandOutput, DdcutilError> {
        let output = self.runner.run("ddcutil", args, timeout)?;
        if output.success() {
            Ok(output)
        } else {
            let detail = first_non_empty_line(&output.stderr)
                .or_else(|| first_non_empty_line(&output.stdout))
                .unwrap_or_else(|| String::from("non-zero exit status"));
            Err(DdcutilError::Failed { detail, output })
        }
    }
}

fn first_non_empty_line(value: &str) -> Option<String> {
    value.lines().find_map(|line| {
        let line = line.trim();
        (!line.is_empty()).then(|| line.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::backends::testutil::FakeRunner;
    use crate::process::CommandError;

    use super::{DdcutilClient, DdcutilError, DdcutilProfile, DdcutilTimeouts};

    #[test]
    fn ubuntu_2204_style_profile_uses_only_supported_arguments() {
        let fixture = include_str!("../../tests/fixtures/ddcutil/msi_then_invalid_boe.txt");
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--version"], "ddcutil 1.2.2")
            .with_success("ddcutil", &["--help"], "--brief")
            .with_success("ddcutil", &["--brief", "detect"], fixture)
            .with_success(
                "ddcutil",
                &["--display", "1", "capabilities"],
                "Feature: 10 (Brightness)",
            )
            .with_success(
                "ddcutil",
                &["--brief", "--display", "1", "getvcp", "10"],
                "VCP code 0x10 (Brightness): current value = 44, max value = 100",
            );
        let client = DdcutilClient::probe(&runner, DdcutilTimeouts::default());

        assert_eq!(client.detect().expect("detect").len(), 1);
        assert!(client.capabilities(1).expect("capabilities"));
        assert_eq!(client.get_brightness(1).expect("getvcp").current, 44);
        assert_eq!(
            runner.calls(),
            vec![
                "ddcutil|--version",
                "ddcutil|--help",
                "ddcutil|--brief|detect",
                "ddcutil|--display|1|capabilities",
                "ddcutil|--brief|--display|1|getvcp|10",
            ]
        );
    }

    #[test]
    fn modern_profile_constructs_one_setvcp_command() {
        let runner = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "4",
                "setvcp",
                "10",
                "51",
            ],
            "",
        );
        let client = DdcutilClient::with_profile(
            &runner,
            DdcutilProfile {
                version_string: String::from("ddcutil 2.2.7"),
                supports_noconfig: true,
                supports_noverify: true,
                supports_terse: true,
                supports_brief: true,
            },
            DdcutilTimeouts::default(),
        );

        client
            .set_brightness(&[String::from("--bus"), String::from("4")], 51)
            .expect("setvcp");
        assert_eq!(
            runner.calls(),
            vec!["ddcutil|--noconfig|--noverify|--bus|4|setvcp|10|51"]
        );
    }

    #[test]
    fn permission_denied_is_a_typed_process_failure() {
        let runner = FakeRunner::new().with_output(
            "ddcutil",
            &["detect"],
            Some(1),
            "",
            "Permission denied opening /dev/i2c-4",
        );
        let client = portable_client(&runner);
        let error = client.detect().expect_err("detect must fail");

        assert!(matches!(error, DdcutilError::Failed { .. }));
        assert!(error.to_string().contains("Permission denied"));
    }

    #[test]
    fn timeout_is_preserved_as_a_typed_command_error() {
        let runner = FakeRunner::new().with_timeout(
            "ddcutil",
            &["detect"],
            Duration::from_secs(25),
            "bus stalled",
        );
        let client = portable_client(&runner);
        let error = client.detect().expect_err("detect must time out");

        assert!(matches!(
            error,
            DdcutilError::Command(CommandError::Timeout { .. })
        ));
    }

    #[test]
    fn unsupported_getvcp_and_unknown_option_are_distinct() {
        let unsupported_runner = FakeRunner::new().with_success(
            "ddcutil",
            &["--display", "1", "getvcp", "10"],
            "VCP code 0x10 is an unsupported feature",
        );
        let unsupported = portable_client(&unsupported_runner)
            .get_brightness(1)
            .expect_err("unsupported VCP must fail");
        assert!(matches!(unsupported, DdcutilError::Parse(_)));

        let option_runner = FakeRunner::new().with_output(
            "ddcutil",
            &["detect"],
            Some(2),
            "",
            "Unknown option --future-option",
        );
        let unknown = portable_client(&option_runner)
            .detect()
            .expect_err("unknown option must fail");
        assert!(matches!(unknown, DdcutilError::Failed { .. }));
    }

    fn portable_client(runner: &FakeRunner) -> DdcutilClient<'_, FakeRunner> {
        DdcutilClient::with_profile(
            runner,
            DdcutilProfile::default(),
            DdcutilTimeouts::default(),
        )
    }
}
