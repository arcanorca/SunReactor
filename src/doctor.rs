use std::fs::{self, OpenOptions};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::discovery::{self, BackendStatusKind, DiscoveryReport};
use crate::ipc::{ControlSocket, Request, RequestEnvelope, Response};
use crate::process::{CommandError, ProcessRunner, RealProcessRunner};

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<CheckResult>,
    pub overall_healthy: bool,
    pub blocking_errors: usize,
    pub i2c_access: I2cAccessState,
    pub backlight_access: BacklightAccessState,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub code: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckStatus {
    Pass,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum I2cAccessState {
    AccessGrantedByUaccess,
    AccessGrantedByI2cGroup,
    I2cGroupConfiguredButSessionStale,
    DeviceExistsButPermissionDenied,
    I2cDevModuleMissing,
    NoRelevantI2cDevice,
    DdcCiDisabledOrUnavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BacklightAccessState {
    SysfsWritable,
    BrightnessctlWorks,
    UdevOrLogindAccess,
    VideoGroupAccess,
    PermissionDenied,
    BackendUnavailable,
    Unknown,
}

pub fn run_diagnostics() -> DoctorReport {
    let runner = RealProcessRunner;
    let discovery = discovery::discover();
    run_diagnostics_with(
        &runner,
        &discovery,
        Path::new("/dev"),
        Path::new("/sys/class/i2c-dev"),
    )
}

fn run_diagnostics_with<R: ProcessRunner>(
    runner: &R,
    discovery: &DiscoveryReport,
    dev_root: &Path,
    i2c_sysfs_root: &Path,
) -> DoctorReport {
    let mut checks = Vec::new();

    let current_groups = current_group_names(runner);
    let i2c_access = classify_i2c_access(
        dev_root,
        i2c_sysfs_root,
        current_groups.as_deref(),
        configured_i2c_membership(),
        discovery,
    );
    checks.push(i2c_check(i2c_access));

    let backlight_access = classify_backlight_access(runner, discovery, current_groups.as_deref());
    checks.push(backlight_check(backlight_access));

    checks.push(backend_check(
        "SR-DDCUTIL",
        "external DDC/CI discovery",
        discovery.backends.ddcutil.status,
        &discovery.backends.ddcutil.message,
        discovery.summary.backlight_devices > 0,
    ));
    checks.push(backend_check(
        "SR-BACKLIGHT",
        "internal backlight discovery",
        discovery.backends.sysfs.status,
        &discovery.backends.sysfs.message,
        discovery.summary.ddc_monitors > 0,
    ));

    checks.push(match crate::config::load() {
        Ok(report) => CheckResult::pass(
            "SR-CONFIG-OK",
            format!(
                "Configuration is valid at {} ({} monitor(s)).",
                report.path.display(),
                report.config.monitors.len()
            ),
        ),
        Err(error) => CheckResult::error("SR-CONFIG-ERR", error.to_string()),
    });

    let daemon_program = daemon_executable();
    checks.push(
        match runner.run(
            &daemon_program,
            &[String::from("--help")],
            Duration::from_secs(3),
        ) {
            Ok(output) if output.success() => CheckResult::pass(
                "SR-DAEMON-EXEC-OK",
                format!("sunreactord launches from {daemon_program}."),
            ),
            Ok(output) => CheckResult::error(
                "SR-DAEMON-EXEC-ERR",
                format!("sunreactord launch returned {:?}.", output.exit_code),
            ),
            Err(CommandError::Missing { .. }) => {
                CheckResult::error("SR-DAEMON-EXEC-MISSING", "sunreactord is not in PATH.")
            }
            Err(error) => CheckResult::error("SR-DAEMON-EXEC-ERR", error.to_string()),
        },
    );

    checks.push(check_ipc());

    let blocking_errors = checks
        .iter()
        .filter(|check| check.status == CheckStatus::Error)
        .count();
    DoctorReport {
        checks,
        overall_healthy: blocking_errors == 0,
        blocking_errors,
        i2c_access,
        backlight_access,
    }
}

fn daemon_executable() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(|parent| parent.join("sunreactord")))
        .filter(|candidate| candidate.is_file())
        .map_or_else(
            || String::from("sunreactord"),
            |candidate| candidate.to_string_lossy().into_owned(),
        )
}

impl CheckResult {
    fn pass(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            status: CheckStatus::Pass,
            message: message.into(),
        }
    }

    fn warn(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            status: CheckStatus::Warn,
            message: message.into(),
        }
    }

    fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            status: CheckStatus::Error,
            message: message.into(),
        }
    }
}

fn classify_i2c_access(
    dev_root: &Path,
    i2c_sysfs_root: &Path,
    current_groups: Option<&[String]>,
    configured_i2c: bool,
    discovery: &DiscoveryReport,
) -> I2cAccessState {
    let Ok(entries) = fs::read_dir(dev_root) else {
        return I2cAccessState::Unknown;
    };
    let devices = entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("i2c-"))
        .collect::<Vec<_>>();
    if devices.is_empty() {
        return if i2c_sysfs_root.exists() {
            I2cAccessState::NoRelevantI2cDevice
        } else {
            I2cAccessState::I2cDevModuleMissing
        };
    }

    for device in &devices {
        if OpenOptions::new()
            .read(true)
            .write(true)
            .open(device.path())
            .is_ok()
        {
            let granted_by_i2c_group = current_groups
                .is_some_and(|groups| groups.iter().any(|group| group == "i2c"))
                && device
                    .metadata()
                    .ok()
                    .and_then(|metadata| group_name_for_gid(metadata.gid()))
                    .is_some_and(|group| group == "i2c");
            if discovery.ddc_monitors.is_empty()
                && discovery.backends.ddcutil.status == BackendStatusKind::Ok
            {
                return I2cAccessState::DdcCiDisabledOrUnavailable;
            }
            return if granted_by_i2c_group {
                I2cAccessState::AccessGrantedByI2cGroup
            } else {
                I2cAccessState::AccessGrantedByUaccess
            };
        }
    }

    if configured_i2c
        && !current_groups.is_some_and(|groups| groups.iter().any(|group| group == "i2c"))
    {
        I2cAccessState::I2cGroupConfiguredButSessionStale
    } else {
        I2cAccessState::DeviceExistsButPermissionDenied
    }
}

fn classify_backlight_access<R: ProcessRunner>(
    runner: &R,
    discovery: &DiscoveryReport,
    current_groups: Option<&[String]>,
) -> BacklightAccessState {
    let Some(device) = discovery.backlight_devices.first() else {
        return BacklightAccessState::BackendUnavailable;
    };
    let brightness = Path::new(&device.sysfs_path).join("brightness");
    if OpenOptions::new().write(true).open(&brightness).is_ok() {
        if current_groups.is_some_and(|groups| groups.iter().any(|group| group == "video")) {
            return BacklightAccessState::VideoGroupAccess;
        }
        return BacklightAccessState::SysfsWritable;
    }

    let args = vec![
        String::from("--device"),
        device.device_name.clone(),
        String::from("get"),
    ];
    match runner.run("brightnessctl", &args, Duration::from_secs(3)) {
        Ok(output) if output.success() => BacklightAccessState::BrightnessctlWorks,
        Ok(output)
            if format!("{}\n{}", output.stdout, output.stderr)
                .to_ascii_lowercase()
                .contains("permission denied") =>
        {
            BacklightAccessState::PermissionDenied
        }
        Err(CommandError::Missing { .. }) => BacklightAccessState::BackendUnavailable,
        _ => BacklightAccessState::Unknown,
    }
}

fn i2c_check(state: I2cAccessState) -> CheckResult {
    match state {
        I2cAccessState::AccessGrantedByUaccess => CheckResult::pass(
            "SR-I2C-UACCESS-OK",
            "I2C read/write access is already granted without relying on the i2c group.",
        ),
        I2cAccessState::AccessGrantedByI2cGroup => {
            CheckResult::pass("SR-I2C-GROUP-OK", "I2C access is granted by the i2c group.")
        }
        I2cAccessState::I2cGroupConfiguredButSessionStale => CheckResult::error(
            "SR-I2C-SESSION-STALE",
            "The account is configured for i2c but this session is stale; log out and back in or reboot before hardware verification.",
        ),
        I2cAccessState::DeviceExistsButPermissionDenied => CheckResult::error(
            "SR-I2C-PERMISSION-DENIED",
            "I2C devices exist but cannot be opened read/write. Review udev/logind rules; add the user to i2c only after confirming group ownership.",
        ),
        I2cAccessState::I2cDevModuleMissing => CheckResult::warn(
            "SR-I2C-DEV-MISSING",
            "No I2C character devices or i2c-dev sysfs class were found; the i2c-dev module may be unavailable.",
        ),
        I2cAccessState::NoRelevantI2cDevice => CheckResult::warn(
            "SR-I2C-NO-DEVICE",
            "The i2c-dev subsystem exists but exposes no relevant character devices.",
        ),
        I2cAccessState::DdcCiDisabledOrUnavailable => CheckResult::warn(
            "SR-DDC-CI-UNAVAILABLE",
            "I2C access works, but ddcutil found no valid DDC/CI display; check monitor settings, cabling, docks, and GPU support.",
        ),
        I2cAccessState::Unknown => {
            CheckResult::warn("SR-I2C-UNKNOWN", "I2C access could not be classified.")
        }
    }
}

fn backlight_check(state: BacklightAccessState) -> CheckResult {
    match state {
        BacklightAccessState::SysfsWritable => {
            CheckResult::pass("SR-BACKLIGHT-SYSFS-OK", "Selected backlight sysfs control is writable.")
        }
        BacklightAccessState::BrightnessctlWorks => CheckResult::pass(
            "SR-BACKLIGHTCTL-OK",
            "brightnessctl can read the selected backlight without a brightness write.",
        ),
        BacklightAccessState::VideoGroupAccess => CheckResult::pass(
            "SR-BACKLIGHT-VIDEO-OK",
            "Selected backlight sysfs control is writable in a session with video group access.",
        ),
        BacklightAccessState::UdevOrLogindAccess => CheckResult::pass(
            "SR-BACKLIGHT-UACCESS-OK",
            "Selected backlight is available through session device access.",
        ),
        BacklightAccessState::PermissionDenied => CheckResult::error(
            "SR-BACKLIGHT-PERMISSION-DENIED",
            "The selected backlight backend reports permission denied; inspect udev/logind policy before changing groups.",
        ),
        BacklightAccessState::BackendUnavailable => CheckResult::warn(
            "SR-BACKLIGHT-UNAVAILABLE",
            "No internal backlight backend is available.",
        ),
        BacklightAccessState::Unknown => CheckResult::warn(
            "SR-BACKLIGHT-UNKNOWN",
            "The selected backlight exists but access could not be classified.",
        ),
    }
}

fn backend_check(
    code: &str,
    name: &str,
    status: BackendStatusKind,
    detail: &str,
    alternative_available: bool,
) -> CheckResult {
    match status {
        BackendStatusKind::Ok => CheckResult::pass(format!("{code}-OK"), detail),
        BackendStatusKind::Missing | BackendStatusKind::Unavailable if alternative_available => {
            CheckResult::warn(format!("{code}-OPTIONAL"), format!("{name}: {detail}"))
        }
        BackendStatusKind::Missing | BackendStatusKind::Unavailable => {
            CheckResult::warn(format!("{code}-UNAVAILABLE"), format!("{name}: {detail}"))
        }
        BackendStatusKind::Timeout | BackendStatusKind::Error => {
            CheckResult::warn(format!("{code}-WARN"), format!("{name}: {detail}"))
        }
    }
}

fn check_ipc() -> CheckResult {
    let socket = match ControlSocket::from_runtime() {
        Ok(socket) => socket,
        Err(error) => return CheckResult::error("SR-IPC-PATH-ERR", error.to_string()),
    };
    match socket.send_request(&RequestEnvelope::new(Request::Ping)) {
        Ok(response) if matches!(response.response, Response::Pong { .. }) => {
            CheckResult::pass("SR-IPC-OK", "Daemon IPC responded to ping.")
        }
        Ok(response) => CheckResult::error(
            "SR-IPC-PROTOCOL-ERR",
            format!("Unexpected IPC response: {}.", response.kind_name()),
        ),
        Err(error) => CheckResult::error("SR-IPC-ERR", error.to_string()),
    }
}

fn current_group_names<R: ProcessRunner>(runner: &R) -> Option<Vec<String>> {
    let output = runner
        .run("id", &[String::from("-nG")], Duration::from_secs(2))
        .ok()?;
    output.success().then(|| {
        output
            .stdout
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    })
}

fn configured_i2c_membership() -> bool {
    let Some(user) = std::env::var_os("USER") else {
        return false;
    };
    let user = user.to_string_lossy();
    fs::read_to_string("/etc/group").ok().is_some_and(|groups| {
        groups.lines().any(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            fields.first() == Some(&"i2c")
                && fields
                    .get(3)
                    .is_some_and(|members| members.split(',').any(|member| member == user))
        })
    })
}

fn group_name_for_gid(gid: u32) -> Option<String> {
    fs::read_to_string("/etc/group")
        .ok()?
        .lines()
        .find_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            fields.next()?;
            let candidate = fields.next()?.parse::<u32>().ok()?;
            (candidate == gid).then(|| name.to_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::{backlight_check, i2c_check, BacklightAccessState, CheckStatus, I2cAccessState};

    #[test]
    fn stale_i2c_session_is_blocking_and_explicit() {
        let check = i2c_check(I2cAccessState::I2cGroupConfiguredButSessionStale);
        assert_eq!(check.status, CheckStatus::Error);
        assert!(check.message.contains("log out and back in"));
    }

    #[test]
    fn no_ddc_display_is_not_misreported_as_permission_denied() {
        let check = i2c_check(I2cAccessState::DdcCiDisabledOrUnavailable);
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.message.contains("I2C access works"));
    }

    #[test]
    fn backlight_permission_denied_is_blocking() {
        let check = backlight_check(BacklightAccessState::PermissionDenied);
        assert_eq!(check.status, CheckStatus::Error);
    }
}
