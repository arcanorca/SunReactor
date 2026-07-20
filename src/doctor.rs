use crate::ddcutil::version::DdcutilVersion;
use serde::Serialize;
use std::process::Command;

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

    use crate::backends::{RealProcessRunner, ProcessRunner, CommandError};
    use std::time::Duration;

    let runner = RealProcessRunner;

    // Check ddcutil version
    match runner.run("ddcutil", &["--version".to_string()], Duration::from_secs(5)) {
        Ok(output) if output.success() => {
            if let Some(version) = DdcutilVersion::parse(&output.stdout) {
                if version.major < 1 || (version.major == 1 && version.minor < 4) {
                    checks.push(CheckResult {
                        code: String::from("SR-DDCUTIL-VERSION-WARN"),
                        status: CheckStatus::Warn,
                        message: format!("ddcutil version {}.{}.{} is older than 1.4.0. Compatibility layer will be used.", version.major, version.minor, version.patch),
                    });
                } else {
                    checks.push(CheckResult {
                        code: String::from("SR-DDCUTIL-VERSION-OK"),
                        status: CheckStatus::Pass,
                        message: format!(
                            "ddcutil version {}.{}.{} is fully supported.",
                            version.major, version.minor, version.patch
                        ),
                    });
                }
            } else {
                checks.push(CheckResult {
                    code: String::from("SR-DDCUTIL-PARSE-WARN"),
                    status: CheckStatus::Warn,
                    message: String::from("Failed to parse ddcutil version."),
                });
            }
        }
        Ok(_) => {
            checks.push(CheckResult {
                code: String::from("SR-DDCUTIL-ERR"),
                status: CheckStatus::Error,
                message: String::from("ddcutil command failed."),
            });
        }
        Err(CommandError::Missing { .. }) => {
            checks.push(CheckResult {
                code: String::from("SR-DDCUTIL-MISSING-ERR"),
                status: CheckStatus::Error,
                message: String::from("ddcutil is not installed or not in PATH."),
            });
        }
        Err(err) => {
            checks.push(CheckResult {
                code: String::from("SR-DDCUTIL-ERR"),
                status: CheckStatus::Error,
                message: format!("ddcutil command error: {}", err),
            });
        }
    }

    // Check I2C group
    let has_i2c_group = if let Ok(output) = Command::new("groups").output() {
        String::from_utf8_lossy(&output.stdout).contains("i2c")
    } else {
        false
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
    if Command::new("sunreactord")
        .arg("--version")
        .output()
        .is_err()
    {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()));
        let mut found_next_to_cli = false;
        if let Some(dir) = exe_dir {
            if dir.join("sunreactord").exists() {
                found_next_to_cli = true;
            }
        }

        if found_next_to_cli {
            checks.push(CheckResult {
                code: String::from("SR-DAEMON-PATH-WARN"),
                status: CheckStatus::Warn,
                message: String::from("sunreactord is not in PATH, but was found in the same directory as sunreactorctl."),
            });
        } else {
            checks.push(CheckResult {
                code: String::from("SR-DAEMON-PATH-ERR"),
                status: CheckStatus::Error,
                message: String::from("sunreactord is not in PATH and not next to sunreactorctl."),
            });
        }
    } else {
        checks.push(CheckResult {
            code: String::from("SR-DAEMON-PATH-OK"),
            status: CheckStatus::Pass,
            message: String::from("sunreactord is in PATH."),
        });
    }

    // GLIBC Check (if applicable on Linux)
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("ldd").arg("--version").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
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
