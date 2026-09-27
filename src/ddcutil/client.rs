use std::time::Duration;

use crate::backends::{CommandError, CommandOutput, ProcessRunner};

use super::parser::{self, BrightnessValue, ParseError};
use super::{command, DdcutilCapabilities};

#[derive(Debug, Clone, Copy)]
pub(crate) struct DdcutilTimeouts {
    pub detect: Duration,
    pub capabilities: Duration,
    pub getvcp: Duration,
}

impl Default for DdcutilTimeouts {
    fn default() -> Self {
        Self {
            detect: Duration::from_secs(25),
            capabilities: Duration::from_secs(12),
            getvcp: Duration::from_secs(8),
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
    capabilities: DdcutilCapabilities,
    timeouts: DdcutilTimeouts,
}

impl<'a, R: ProcessRunner> DdcutilClient<'a, R> {
    pub(crate) fn probe(runner: &'a R, timeouts: DdcutilTimeouts) -> Self {
        Self {
            runner,
            capabilities: command::probe_capabilities(runner),
            timeouts,
        }
    }

    pub(crate) fn run_with_compatible_arguments(
        runner: &R,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        let output = runner.run("ddcutil", args, timeout)?;
        if output.success() || !command::reports_unsupported_option(&output) {
            return Ok(output);
        }

        let capabilities = command::probe_capabilities(runner);
        let compatible_args = command::adapt_args_for_capabilities(args, &capabilities);
        if compatible_args == args {
            return Ok(output);
        }

        runner.run("ddcutil", &compatible_args, timeout)
    }

    pub(crate) fn detect_output(&self) -> Result<CommandOutput, CommandError> {
        let args = command::build_detect_args(&self.capabilities);
        run_with_capabilities(self.runner, &args, self.timeouts.detect, &self.capabilities)
    }

    pub(crate) fn capabilities_output_for_selection(
        &self,
        selection_args: &[String],
    ) -> Result<CommandOutput, CommandError> {
        let args =
            command::build_capabilities_args_for_selection(&self.capabilities, selection_args);
        run_with_capabilities(
            self.runner,
            &args,
            self.timeouts.capabilities,
            &self.capabilities,
        )
    }

    pub(crate) fn get_brightness_for_selection(
        &self,
        selection_args: &[String],
    ) -> Result<BrightnessValue, DdcutilError> {
        let args =
            command::build_getvcp_args_for_selection(&self.capabilities, selection_args, "10");
        let output = self.execute(&args, self.timeouts.getvcp)?;
        Ok(parser::parse_getvcp_brightness(&output.stdout)?)
    }

    fn execute(&self, args: &[String], timeout: Duration) -> Result<CommandOutput, DdcutilError> {
        let output = run_with_capabilities(self.runner, args, timeout, &self.capabilities)?;
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

fn run_with_capabilities<R: ProcessRunner>(
    runner: &R,
    args: &[String],
    timeout: Duration,
    capabilities: &DdcutilCapabilities,
) -> Result<CommandOutput, CommandError> {
    let output = runner.run("ddcutil", args, timeout)?;
    if output.success() || !command::reports_unsupported_option(&output) {
        return Ok(output);
    }

    let compatible_args = command::adapt_args_for_capabilities(args, capabilities);
    if compatible_args == args {
        return Ok(output);
    }

    runner.run("ddcutil", &compatible_args, timeout)
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

    use super::{DdcutilClient, DdcutilError, DdcutilTimeouts};

    #[test]
    fn ubuntu_2204_style_profile_uses_only_supported_arguments() {
        let fixture = include_str!("../../tests/fixtures/ddcutil/msi_then_invalid_boe.txt");
        let runner = FakeRunner::new()
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

        assert!(client.detect_output().expect("detect").success());
        assert!(client
            .capabilities_output_for_selection(&[String::from("--display"), String::from("1")])
            .expect("capabilities")
            .success());
        assert_eq!(
            client
                .get_brightness_for_selection(&[String::from("--display"), String::from("1")])
                .expect("getvcp")
                .current,
            44
        );
        assert_eq!(
            runner.calls(),
            vec![
                "ddcutil|--help",
                "ddcutil|--brief|detect",
                "ddcutil|--display|1|capabilities",
                "ddcutil|--brief|--display|1|getvcp|10",
            ]
        );
    }

    #[test]
    fn legacy_profile_retries_only_with_supported_ddcutil_arguments() {
        let detect_args = ["--noconfig", "--terse", "detect"];
        let fixture = include_str!("../../tests/fixtures/ddcutil/msi_then_invalid_boe.txt");
        let runner = FakeRunner::new()
            .with_output(
                "ddcutil",
                &detect_args,
                Some(2),
                "",
                "Unknown option --noconfig",
            )
            .with_success("ddcutil", &["--help"], "--brief")
            .with_success("ddcutil", &["--brief", "detect"], fixture);

        let output = DdcutilClient::run_with_compatible_arguments(
            &runner,
            &detect_args.map(String::from),
            Duration::from_secs(25),
        )
        .expect("compatible retry");

        assert!(output.success());
        assert_eq!(
            runner.calls(),
            vec![
                "ddcutil|--noconfig|--terse|detect",
                "ddcutil|--help",
                "ddcutil|--brief|detect",
            ]
        );
    }

    #[test]
    fn real_ddcutil_errors_do_not_probe_or_retry_compatibility() {
        let args = [
            String::from("--noconfig"),
            String::from("--terse"),
            String::from("detect"),
        ];
        let runner = FakeRunner::new().with_output(
            "ddcutil",
            &["--noconfig", "--terse", "detect"],
            Some(1),
            "",
            "Permission denied opening /dev/i2c-4",
        );

        let output =
            DdcutilClient::run_with_compatible_arguments(&runner, &args, Duration::from_secs(25))
                .expect("operational failure remains an output");

        assert!(!output.success());
        assert_eq!(runner.calls(), vec!["ddcutil|--noconfig|--terse|detect"]);
    }

    #[test]
    fn getvcp_permission_denial_is_preserved_as_a_typed_failure() {
        let args = ["--display", "1", "getvcp", "10"];
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "")
            .with_output(
                "ddcutil",
                &args,
                Some(1),
                "",
                "Permission denied opening /dev/i2c-4",
            );
        let client = DdcutilClient::probe(&runner, DdcutilTimeouts::default());

        let error = client
            .get_brightness_for_selection(&[String::from("--display"), String::from("1")])
            .expect_err("permission denial must remain an error");
        assert!(matches!(error, DdcutilError::Failed { .. }));
        assert!(error.to_string().contains("Permission denied"));
    }

    #[test]
    fn getvcp_timeout_is_preserved_as_a_typed_command_error() {
        let args = ["--display", "1", "getvcp", "10"];
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "")
            .with_timeout("ddcutil", &args, Duration::from_secs(8), "bus stalled");
        let client = DdcutilClient::probe(&runner, DdcutilTimeouts::default());

        let error = client
            .get_brightness_for_selection(&[String::from("--display"), String::from("1")])
            .expect_err("getvcp must time out");
        assert!(matches!(
            error,
            DdcutilError::Command(CommandError::Timeout { .. })
        ));
    }

    #[test]
    fn unsupported_brightness_vcp_is_distinct_from_malformed_output() {
        let args = ["--display", "1", "getvcp", "10"];
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "")
            .with_success("ddcutil", &args, "VCP code 0x10 is an unsupported feature");
        let client = DdcutilClient::probe(&runner, DdcutilTimeouts::default());

        let error = client
            .get_brightness_for_selection(&[String::from("--display"), String::from("1")])
            .expect_err("unsupported VCP must fail");
        assert!(matches!(error, DdcutilError::Parse(_)));
    }
}
