use std::time::Duration;

use crate::config::{MonitorConfig, MonitorSelector};
use crate::ddcutil::{DdcutilClient, DdcutilError, DdcutilTimeouts};

use super::{
    clamp_percent, command_failure, map_command_error, BackendError, BackendKind, BackendWrite,
    ProcessRunner, RealProcessRunner,
};

const DEFAULT_DDC_ADDRESS: u16 = 0x37;

pub fn apply(monitor: &MonitorConfig, percent: u8) -> Result<BackendWrite, BackendError> {
    apply_with_runner(
        &RealProcessRunner,
        monitor,
        percent,
        Duration::from_secs(10),
    )
}

pub(crate) fn apply_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
) -> Result<BackendWrite, BackendError> {
    let percent = clamp_percent(percent);
    let selection = build_selector(&monitor.selector)?;
    let client = DdcutilClient::probe(
        runner,
        DdcutilTimeouts {
            setvcp: timeout,
            ..DdcutilTimeouts::default()
        },
    );

    let mut attempts = 0u8;

    loop {
        attempts += 1;
        #[cfg(unix)]
        let _lock = {
            let uid = unsafe { libc::geteuid() };
            let lock_path = format!("/run/user/{}/sunreactor_i2c.lock", uid);
            let lock_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .ok();

            if let Some(ref f) = lock_file {
                use rustix::fs::{flock, FlockOperation};
                let _ = flock(f, FlockOperation::LockExclusive);
            }
            lock_file // keep open until block ends
        };

        match client.set_brightness(&selection.args, percent) {
            Ok(_) => {
                return Ok(BackendWrite {
                    backend: BackendKind::Ddc,
                    applied_percent: percent,
                    attempts,
                    detail: format!("applied via ddcutil using {}", selection.description),
                });
            }
            Err(DdcutilError::Failed { output, .. }) => {
                let error = command_failure(BackendKind::Ddc, "ddcutil", &output);
                if attempts == 1 && should_retry(&error) {
                    continue;
                }
                return Err(error.with_attempts(attempts));
            }
            Err(DdcutilError::Command(error)) => {
                let error = map_command_error(BackendKind::Ddc, error);
                if attempts == 1 && should_retry(&error) {
                    continue;
                }
                return Err(error.with_attempts(attempts));
            }
            Err(DdcutilError::Parse(error)) => {
                return Err(BackendError::Io {
                    backend: BackendKind::Ddc,
                    program: String::from("ddcutil"),
                    message: error.to_string(),
                    attempts,
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DdcSelection {
    args: Vec<String>,
    description: String,
}

fn build_selector(selector: &MonitorSelector) -> Result<DdcSelection, BackendError> {
    let serial = normalized(&selector.serial);
    let model = normalized(&selector.model);
    let edid = normalized(&selector.edid);
    let connector = normalized(&selector.connector);
    let sysfs_path = normalized(&selector.sysfs_path);

    if sysfs_path.is_some() {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Ddc,
            field: "sysfs_path",
            message: String::from("sysfs_path only applies to backlight devices"),
        });
    }

    if let Some(address) = selector.ddc_address {
        if address != DEFAULT_DDC_ADDRESS {
            return Err(BackendError::InvalidSelector {
                backend: BackendKind::Ddc,
                field: "ddc_address",
                message: format!(
                    "only the standard DDC/CI slave address {DEFAULT_DDC_ADDRESS} is supported"
                ),
            });
        }
    }

    if let Some(edid) = edid {
        validate_edid(&edid)?;
        return Ok(DdcSelection {
            args: vec![String::from("--edid"), edid],
            description: String::from("EDID"),
        });
    }

    if let Some(serial) = serial {
        let mut args = vec![String::from("--sn"), serial.clone()];
        let mut description = format!("serial `{serial}`");
        if let Some(model) = model {
            args.push(String::from("--model"));
            args.push(model.clone());
            description.push_str(&format!(" and model `{model}`"));
        }

        return Ok(DdcSelection { args, description });
    }

    if let Some(bus) = selector.ddc_bus {
        return Ok(DdcSelection {
            args: vec![String::from("--bus"), bus.to_string()],
            description: format!("bus {bus}"),
        });
    }

    if let Some(model) = model {
        let args = vec![String::from("--model"), model.clone()];
        let description = format!("model `{model}`");

        return Ok(DdcSelection { args, description });
    }

    if connector.is_some() {
        return Err(BackendError::MissingSelector {
            backend: BackendKind::Ddc,
            expected:
                "serial, model, edid, or ddc_bus; connector alone is not stable enough for apply",
        });
    }

    Err(BackendError::MissingSelector {
        backend: BackendKind::Ddc,
        expected: "serial, model, edid, or ddc_bus",
    })
}

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn validate_edid(edid: &str) -> Result<(), BackendError> {
    if edid.len() != 256 || !edid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(BackendError::InvalidSelector {
            backend: BackendKind::Ddc,
            field: "edid",
            message: String::from("expected exactly 256 hexadecimal characters"),
        });
    }

    Ok(())
}

fn should_retry(error: &BackendError) -> bool {
    matches!(
        error,
        BackendError::CommandTimeout { .. }
            | BackendError::CommandFailed {
                transient: true,
                ..
            }
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::backends::testutil::FakeRunner;
    use crate::config::{MonitorConfig, MonitorSelector};

    use super::apply_with_runner;

    #[test]
    fn retries_once_for_transient_ddc_failures_and_uses_stable_selectors() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: Some(String::from("card1-DP-1")),
            serial: Some(String::from("ABC123")),
            model: Some(String::from("U2720Q")),
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(7),
            ddc_address: Some(0x37),
        });
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "--noconfig --noverify")
            .with_output(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--sn",
                    "ABC123",
                    "--model",
                    "U2720Q",
                    "setvcp",
                    "10",
                    "64",
                ],
                Some(1),
                "",
                "Device or resource busy",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--sn",
                    "ABC123",
                    "--model",
                    "U2720Q",
                    "setvcp",
                    "10",
                    "64",
                ],
                "",
            );

        let result = apply_with_runner(&runner, &monitor, 64, Duration::from_secs(10))
            .expect("ddc write should succeed");

        assert_eq!(result.applied_percent, 64);
        assert_eq!(result.attempts, 2);

        let calls = runner.calls();
        assert!(calls
            .iter()
            .any(|c| c.contains("|--sn|ABC123|--model|U2720Q|setvcp|10|64")));
    }

    #[test]
    fn falls_back_to_bus_when_no_stable_ddc_selector_exists() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: Some(String::from("card1-DP-3")),
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(9),
            ddc_address: None,
        });
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "--noconfig --noverify")
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "9",
                    "setvcp",
                    "10",
                    "42",
                ],
                "",
            );

        let result = apply_with_runner(&runner, &monitor, 42, Duration::from_secs(10))
            .expect("ddc write should succeed");

        assert_eq!(result.attempts, 1);
        assert!(result.detail.contains("bus 9"));
    }

    #[test]
    fn rejects_connector_only_selector() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: Some(String::from("card1-DP-1")),
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: None,
            ddc_address: None,
        });
        let runner = FakeRunner::new();

        let error = apply_with_runner(&runner, &monitor, 50, Duration::from_secs(4))
            .expect_err("connector-only selector must fail");

        assert!(error
            .to_string()
            .contains("connector alone is not stable enough"));
    }

    #[test]
    fn retries_timeouts_only_once() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: None,
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(6),
            ddc_address: None,
        });
        let runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "--noconfig --noverify")
            .with_timeout(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "6",
                    "setvcp",
                    "10",
                    "55",
                ],
                Duration::from_secs(4),
                "timed out",
            )
            .with_timeout(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "6",
                    "setvcp",
                    "10",
                    "55",
                ],
                Duration::from_secs(4),
                "timed out again",
            );

        let error = apply_with_runner(&runner, &monitor, 55, Duration::from_secs(10))
            .expect_err("second timeout should fail");

        assert!(error.to_string().contains("timed out"));
    }

    fn ddc_monitor(selector: MonitorSelector) -> MonitorConfig {
        MonitorConfig {
            logical_id: String::from("desk"),
            backend: crate::backends::BackendKind::Ddc,
            enabled: true,
            min_pct: 0,
            max_pct: 100,
            gain: 1.0,
            transition_gamma: 1.4,
            milestone_adjustments: Vec::new(),
            selector,
        }
    }
}
