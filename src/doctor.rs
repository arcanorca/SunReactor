use crate::backends::{CommandError, ProcessRunner, RealProcessRunner};
use crate::ddcutil::client::{DdcutilClient, DdcutilTimeouts};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<CheckResult>,
    pub overall_healthy: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub code: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Error,
}

pub fn run_diagnostics() -> DoctorReport {
    let mut checks = Vec::new();
    let runner = RealProcessRunner;

    // Check ddcutil capabilities and run detect
    let profile = DdcutilClient::<RealProcessRunner>::probe_profile(&runner);
    if !profile.version_string.is_empty() {
        if profile.supports_noconfig && profile.supports_terse && profile.supports_noverify {
            checks.push(CheckResult {
                code: String::from("SR-DDCUTIL-CAPS-OK"),
                status: CheckStatus::Pass,
                message: format!(
                    "ddcutil is installed ({}) and supports all required flags.",
                    profile.version_string
                ),
            });
        } else {
            checks.push(CheckResult {
                code: String::from("SR-DDCUTIL-CAPS-WARN"),
                status: CheckStatus::Warn,
                message: format!(
                    "ddcutil is installed ({}) but lacks some capabilities (noconfig: {}, terse: {}, noverify: {}). Compatibility mode active.",
                    profile.version_string, profile.supports_noconfig, profile.supports_terse, profile.supports_noverify
                ),
            });
        }

        let client = DdcutilClient::new(RealProcessRunner, profile, DdcutilTimeouts::default());
        match client.detect() {
            Ok(monitors) => {
                if monitors.is_empty() {
                    checks.push(CheckResult {
                        code: String::from("SR-DDCUTIL-DETECT-WARN"),
                        status: CheckStatus::Warn,
                        message: String::from(
                            "ddcutil detect ran successfully, but no DDC/CI monitors were found.",
                        ),
                    });
                } else {
                    checks.push(CheckResult {
                        code: String::from("SR-DDCUTIL-DETECT-OK"),
                        status: CheckStatus::Pass,
                        message: format!("ddcutil detect found {} monitor(s).", monitors.len()),
                    });
                }
            }
            Err(e) => {
                checks.push(CheckResult {
                    code: String::from("SR-DDCUTIL-DETECT-ERR"),
                    status: CheckStatus::Error,
                    message: format!("ddcutil detect failed: {}", e),
                });
            }
        }
    } else {
        match RealProcessRunner.run(
            "ddcutil",
            &["--version".to_string()],
            Duration::from_secs(2),
        ) {
            Err(CommandError::Missing { .. }) => {
                checks.push(CheckResult {
                    code: String::from("SR-DDCUTIL-MISSING-ERR"),
                    status: CheckStatus::Error,
                    message: String::from("ddcutil is not installed or not in PATH."),
                });
            }
            Err(e) => {
                checks.push(CheckResult {
                    code: String::from("SR-DDCUTIL-ERR"),
                    status: CheckStatus::Error,
                    message: format!("ddcutil execution error: {}", e),
                });
            }
            Ok(_) => {
                checks.push(CheckResult {
                    code: String::from("SR-DDCUTIL-PARSE-WARN"),
                    status: CheckStatus::Warn,
                    message: String::from(
                        "Failed to parse ddcutil capabilities or version string.",
                    ),
                });
            }
        }
    }

    // Check I2C group
    let has_i2c_group = match runner.run("groups", &[], Duration::from_secs(2)) {
        Ok(output) => output.stdout.contains("i2c"),
        Err(_) => false,
    };

    // Check access to /dev/i2c-*
    let mut i2c_access = false;
    let mut i2c_exists = false;
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("i2c-") {
                i2c_exists = true;
                if std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(entry.path())
                    .is_ok()
                {
                    i2c_access = true;
                    break;
                }
            }
        }
    }

    if !i2c_exists {
        checks.push(CheckResult {
            code: String::from("SR-I2C-MISSING-WARN"),
            status: CheckStatus::Warn,
            message: String::from("No /dev/i2c-* devices found. Ensure i2c-dev module is loaded."),
        });
    } else if !i2c_access && !has_i2c_group {
        checks.push(CheckResult {
            code: String::from("SR-I2C-PERMS-ERR"),
            status: CheckStatus::Error,
            message: String::from(
                "User does not have read/write access to /dev/i2c-* and is not in the i2c group.",
            ),
        });
    } else {
        checks.push(CheckResult {
            code: String::from("SR-I2C-PERMS-OK"),
            status: CheckStatus::Pass,
            message: String::from("I2C permissions are sufficient."),
        });
    }

    // Check daemon path
    if runner
        .run(
            "sunreactord",
            &["--version".to_string()],
            Duration::from_secs(2),
        )
        .is_err()
    {
        checks.push(CheckResult {
            code: String::from("SR-DAEMON-PATH-WARN"),
            status: CheckStatus::Warn,
            message: String::from(
                "sunreactord is not in PATH. Ensure ~/.cargo/bin or ~/.local/bin is in PATH.",
            ),
        });
    } else {
        checks.push(CheckResult {
            code: String::from("SR-DAEMON-PATH-OK"),
            status: CheckStatus::Pass,
            message: String::from("sunreactord is in PATH."),
        });
    }

    // Check IPC
    if let Ok(output) = runner.run(
        "sunreactorctl",
        &["status".to_string()],
        Duration::from_secs(2),
    ) {
        if output.success() {
            checks.push(CheckResult {
                code: String::from("SR-IPC-OK"),
                status: CheckStatus::Pass,
                message: String::from("IPC with sunreactord is functioning normally."),
            });
        } else {
            checks.push(CheckResult {
                code: String::from("SR-IPC-WARN"),
                status: CheckStatus::Warn,
                message: String::from(
                    "sunreactord does not appear to be running or IPC is failing.",
                ),
            });
        }
    }

    // GLIBC Check (if applicable on Linux)
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = runner.run("ldd", &["--version".to_string()], Duration::from_secs(2)) {
            let stdout = output.stdout;
            if let Some(line) = stdout.lines().next() {
                if let Some(version_str) = line.split_whitespace().last() {
                    let parts: Vec<&str> = version_str.split('.').collect();
                    if parts.len() >= 2 {
                        if let (Ok(major), Ok(minor)) =
                            (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                        {
                            if major < 2 || (major == 2 && minor < 31) {
                                checks.push(CheckResult {
                                    code: String::from("SR-GLIBC-WARN"),
                                    status: CheckStatus::Warn,
                                    message: format!("GLIBC version {}.{} is older than 2.31, some features may not work.", major, minor),
                                });
                            } else {
                                checks.push(CheckResult {
                                    code: String::from("SR-GLIBC-OK"),
                                    status: CheckStatus::Pass,
                                    message: format!(
                                        "GLIBC version {}.{} is supported.",
                                        major, minor
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let overall_healthy = checks.iter().all(|c| c.status != CheckStatus::Error);

    DoctorReport {
        checks,
        overall_healthy,
    }
}
