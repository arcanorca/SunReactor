use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use crate::config::{MonitorConfig, MonitorSelector};

use super::{
    clamp_percent, command_failure, map_command_error, BackendError, BackendKind, BackendWrite,
    CommandOutput, ProcessRunner, RealProcessRunner,
};

const DDC_BRIGHTNESS_VCP_CODE: &str = "10";
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
    apply_with_runner_in(runner, monitor, percent, timeout, drm_root())
}

fn apply_with_runner_in<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    percent: u8,
    timeout: Duration,
    drm: &Path,
) -> Result<BackendWrite, BackendError> {
    let percent = clamp_percent(percent);
    let identity_selection = build_selector(&monitor.selector)?;
    let bus_selection = verified_bus_selection(&identity_selection, &monitor.selector, drm);
    let mut use_bus = bus_selection.is_some();

    let mut attempts = 0u8;

    loop {
        attempts += 1;
        let selection = match (&bus_selection, use_bus) {
            (Some(selection), true) => selection,
            _ => &identity_selection,
        };
        let args = build_setvcp_args(selection, percent);

        #[cfg(unix)]
        let _lock = {
            let uid = unsafe { libc::geteuid() };
            let lock_path = format!("/run/user/{uid}/sunreactor_i2c.lock");
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

        let run_result = runner.run("ddcutil", &args, timeout);
        let run_result = match run_result {
            Ok(output) if !output.success() && is_unsupported_noconfig_error(&output) => {
                let fallback = filter_unsupported_noconfig(&args);
                runner.run("ddcutil", &fallback, timeout)
            }
            other => other,
        };

        match run_result {
            Ok(output) if output.success() => {
                return Ok(BackendWrite {
                    backend: BackendKind::Ddc,
                    applied_percent: percent,
                    attempts,
                    detail: format!("applied via ddcutil using {}", selection.description),
                });
            }
            Ok(output) => {
                let error = command_failure(BackendKind::Ddc, "ddcutil", &output);
                if attempts == 1 && (use_bus || should_retry(&error)) {
                    // A bus that stopped answering falls back to ddcutil's
                    // own identity matching for the retry.
                    use_bus = false;
                    continue;
                }
                return Err(error.with_attempts(attempts));
            }
            Err(error) => {
                let error = map_command_error(BackendKind::Ddc, error);
                if attempts == 1 && (use_bus || should_retry(&error)) {
                    use_bus = false;
                    continue;
                }
                return Err(error.with_attempts(attempts));
            }
        }
    }
}

pub(crate) fn read_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
) -> Result<super::BackendObservation, BackendError> {
    read_with_runner_in(runner, monitor, timeout, drm_root())
}

fn read_with_runner_in<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
    drm: &Path,
) -> Result<super::BackendObservation, BackendError> {
    let identity_selection = build_selector(&monitor.selector)?;
    let read = |selection: &DdcSelectorPlan| {
        let mut args = vec![String::from("--noconfig"), String::from("--terse")];
        args.extend(selection.args.iter().cloned());
        args.push(String::from("getvcp"));
        args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
        match runner.run("ddcutil", &args, timeout) {
            Ok(output) if !output.success() && is_unsupported_noconfig_error(&output) => {
                let fallback = filter_unsupported_noconfig(&args);
                runner.run("ddcutil", &fallback, timeout)
            }
            other => other,
        }
    };
    let bus_output = verified_bus_selection(&identity_selection, &monitor.selector, drm)
        .map(|selection| read(&selection))
        .filter(|output| output.as_ref().is_ok_and(CommandOutput::success));
    let output = match bus_output {
        Some(output) => output,
        None => read(&identity_selection),
    }
    .map_err(|error| map_command_error(BackendKind::Ddc, error))?;
    parse_brightness_output(&output)
}

/// Reads brightness only over the EDID-verified bus, never through ddcutil's
/// slow identity search.
///
/// Returns `Ok(None)` when the monitor has no verifiable connector, so callers
/// can leave it to regular ticks. A monitor that is still waking fails fast
/// with an error instead of stalling for seconds.
pub(crate) fn read_verified_bus_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
) -> Result<Option<super::BackendObservation>, BackendError> {
    read_verified_bus_in(runner, monitor, timeout, drm_root())
}

fn read_verified_bus_in<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
    drm: &Path,
) -> Result<Option<super::BackendObservation>, BackendError> {
    let identity_selection = build_selector(&monitor.selector)?;
    let Some(selection) = verified_bus_selection(&identity_selection, &monitor.selector, drm)
    else {
        return Ok(None);
    };
    // A display the kernel reports as powered off cannot answer, and asking
    // over I2C would only add bus traffic. Reading the connector's power state
    // costs microseconds and works on every desktop and display server.
    if connector_is_powered_off(&monitor.selector, drm) {
        return Err(BackendError::Io {
            backend: BackendKind::Ddc,
            program: String::from("drm"),
            message: String::from("display is powered off"),
            attempts: 0,
        });
    }
    let mut args = vec![String::from("--noconfig"), String::from("--terse")];
    args.extend(selection.args.iter().cloned());
    args.push(String::from("getvcp"));
    args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
    let output = runner
        .run("ddcutil", &args, timeout)
        .map_err(|error| map_command_error(BackendKind::Ddc, error))?;
    parse_brightness_output(&output).map(Some)
}

fn parse_brightness_output(
    output: &CommandOutput,
) -> Result<super::BackendObservation, BackendError> {
    if !output.success() {
        return Err(command_failure(BackendKind::Ddc, "ddcutil", output));
    }
    let percent = output
        .stdout
        .split_whitespace()
        .nth(3)
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| BackendError::Io {
            backend: BackendKind::Ddc,
            program: String::from("ddcutil"),
            message: format!("unable to parse VCP {DDC_BRIGHTNESS_VCP_CODE} output"),
            attempts: 1,
        })?;
    Ok(super::BackendObservation {
        percent: percent.min(100),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DdcSelectorPlan {
    args: Vec<String>,
    pub(crate) description: String,
    kind: DdcSelectorKind,
}

impl DdcSelectorPlan {
    #[must_use]
    pub(crate) fn is_bus_only(&self) -> bool {
        matches!(self.kind, DdcSelectorKind::Bus(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DdcSelectorRelation {
    Equal,
    ProvablyDisjoint,
    MayOverlap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DdcSelectorKind {
    Bus(u8),
    Serial(String),
    SerialModel { serial: String, model: String },
    Model(String),
    Edid(String),
}

fn normalize_compare(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn is_unsupported_noconfig_error(output: &CommandOutput) -> bool {
    output.stderr.contains("Unknown option --noconfig")
        || output.stderr.contains("unrecognized option '--noconfig'")
}

pub(crate) fn filter_unsupported_noconfig(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| arg.as_str() != "--noconfig")
        .cloned()
        .collect()
}

fn build_setvcp_args(selection: &DdcSelectorPlan, percent: u8) -> Vec<String> {
    let mut args = vec![String::from("--noconfig"), String::from("--noverify")];
    args.extend(selection.args.iter().cloned());
    args.push(String::from("setvcp"));
    args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
    args.push(percent.to_string());
    args
}

/// Where the kernel lists display connectors.
fn drm_root() -> &'static Path {
    if cfg!(test) {
        // Unit tests must never observe the host's displays.
        Path::new("/nonexistent/sunreactor-test-drm")
    } else {
        Path::new("/sys/class/drm")
    }
}

/// Addresses a monitor by its I2C bus once the connector's current EDID proves
/// it is the configured display.
///
/// Selecting by serial, model, or EDID makes ddcutil probe every bus first,
/// which takes several seconds on some systems. The connector's own EDID gives
/// the same identity evidence locally, and its I2C bus is then addressed
/// directly. Every configured identity field must match; otherwise the normal
/// ddcutil selection is used.
/// Whether the kernel reports this monitor's connector as powered off
/// (DPMS off or disconnected). Unknown states count as powered on, so a
/// system without the `dpms` attribute keeps probing over DDC.
fn connector_is_powered_off(selector: &MonitorSelector, root: &Path) -> bool {
    let Some(connector) = normalized(&selector.connector) else {
        return false;
    };
    if connector.contains('/') || connector.starts_with('.') {
        return false;
    }
    let directory = root.join(&connector);
    let off = |file: &str, on: &str| {
        std::fs::read_to_string(directory.join(file))
            .ok()
            .is_some_and(|value| !value.trim().eq_ignore_ascii_case(on))
    };
    off("status", "connected") || off("dpms", "on")
}

fn verified_bus_selection(
    identity: &DdcSelectorPlan,
    selector: &MonitorSelector,
    root: &Path,
) -> Option<DdcSelectorPlan> {
    if matches!(identity.kind, DdcSelectorKind::Bus(_)) {
        return None;
    }
    let connector = normalized(&selector.connector)?;
    if connector.contains('/') || connector.starts_with('.') {
        return None;
    }
    let directory = root.join(&connector);
    let status = std::fs::read_to_string(directory.join("status")).ok()?;
    if status.trim() != "connected" {
        return None;
    }
    let edid = std::fs::read(directory.join("edid")).ok()?;
    let identity_edid = EdidIdentity::parse(&edid)?;
    if !identity_edid.matches(selector) {
        return None;
    }
    let bus = connector_bus(&directory)?;
    Some(DdcSelectorPlan {
        args: vec![String::from("--bus"), bus.to_string()],
        description: format!("{} on bus {bus}", identity.description),
        kind: identity.kind.clone(),
    })
}

/// The connector's DDC bus: a DisplayPort AUX adapter listed inside the
/// connector, or the adapter its `ddc` link names.
fn connector_bus(directory: &Path) -> Option<u8> {
    let parse = |name: &str| name.strip_prefix("i2c-")?.parse::<u8>().ok();
    let mut aux: Vec<u8> = std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| parse(entry.file_name().to_str()?))
        .collect();
    aux.sort_unstable();
    match aux.as_slice() {
        [bus] => return Some(*bus),
        [] => {}
        _ => return None,
    }
    let link = std::fs::read_link(directory.join("ddc")).ok()?;
    parse(link.file_name()?.to_str()?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EdidIdentity {
    hex: String,
    model: Option<String>,
    serial: Option<String>,
}

impl EdidIdentity {
    fn parse(bytes: &[u8]) -> Option<Self> {
        const HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
        if bytes.len() < 128 || bytes[..8] != HEADER {
            return None;
        }
        let mut model = None;
        let mut serial = None;
        for descriptor in bytes[54..126].chunks_exact(18) {
            if descriptor[..3] != [0, 0, 0] {
                continue;
            }
            let text: String = descriptor[5..]
                .iter()
                .take_while(|byte| **byte != 0x0a && **byte != 0)
                .map(|byte| char::from(*byte))
                .collect::<String>()
                .trim()
                .to_owned();
            match descriptor[3] {
                0xfc if !text.is_empty() => model = Some(text),
                0xff if !text.is_empty() => serial = Some(text),
                _ => {}
            }
        }
        let hex = bytes[..128].iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
        Some(Self { hex, model, serial })
    }

    fn matches(&self, selector: &MonitorSelector) -> bool {
        let edid = normalized(&selector.edid);
        let serial = normalized(&selector.serial);
        let model = normalized(&selector.model);
        if edid.is_none() && serial.is_none() && model.is_none() {
            return false;
        }
        edid.is_none_or(|edid| edid.to_ascii_lowercase() == self.hex)
            && serial.is_none_or(|serial| self.serial.as_deref() == Some(serial.as_str()))
            && model.is_none_or(|model| self.model.as_deref() == Some(model.as_str()))
    }
}

fn build_selector(selector: &MonitorSelector) -> Result<DdcSelectorPlan, BackendError> {
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
        return Ok(DdcSelectorPlan {
            args: vec![String::from("--edid"), edid.clone()],
            description: String::from("EDID"),
            kind: DdcSelectorKind::Edid(normalize_compare(&edid)),
        });
    }

    if let Some(serial) = serial {
        let mut args = vec![String::from("--sn"), serial.clone()];
        let mut description = format!("serial `{serial}`");
        let kind = if let Some(model) = model {
            args.push(String::from("--model"));
            args.push(model.clone());
            let _ = write!(description, " and model `{model}`");
            DdcSelectorKind::SerialModel {
                serial: normalize_compare(&serial),
                model: normalize_compare(&model),
            }
        } else {
            DdcSelectorKind::Serial(normalize_compare(&serial))
        };

        return Ok(DdcSelectorPlan {
            args,
            description,
            kind,
        });
    }

    if let Some(bus) = selector.ddc_bus {
        return Ok(DdcSelectorPlan {
            args: vec![String::from("--bus"), bus.to_string()],
            description: format!("bus {bus}"),
            kind: DdcSelectorKind::Bus(bus),
        });
    }

    if let Some(model) = model {
        let args = vec![String::from("--model"), model.clone()];
        let description = format!("model `{model}`");

        return Ok(DdcSelectorPlan {
            args,
            description,
            kind: DdcSelectorKind::Model(normalize_compare(&model)),
        });
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

pub(crate) fn selector_plan(selector: &MonitorSelector) -> Result<DdcSelectorPlan, BackendError> {
    build_selector(selector)
}

pub(crate) fn effective_selector_args(
    selector: &MonitorSelector,
) -> Result<Vec<String>, BackendError> {
    Ok(build_selector(selector)?.args)
}

pub(crate) fn selector_relation(
    left: &MonitorSelector,
    right: &MonitorSelector,
) -> Result<DdcSelectorRelation, BackendError> {
    let left = selector_plan(left)?.kind;
    let right = selector_plan(right)?.kind;
    Ok(match (&left, &right) {
        (DdcSelectorKind::Bus(a), DdcSelectorKind::Bus(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Serial(a), DdcSelectorKind::Serial(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Edid(a), DdcSelectorKind::Edid(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (
            DdcSelectorKind::SerialModel {
                serial: a,
                model: am,
            },
            DdcSelectorKind::SerialModel {
                serial: b,
                model: bm,
            },
        ) => {
            if a != b || am != bm {
                DdcSelectorRelation::ProvablyDisjoint
            } else {
                DdcSelectorRelation::Equal
            }
        }
        (DdcSelectorKind::Model(a), DdcSelectorKind::Model(b)) => {
            if a == b {
                DdcSelectorRelation::Equal
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Model(a), DdcSelectorKind::SerialModel { model: b, .. })
        | (DdcSelectorKind::SerialModel { model: b, .. }, DdcSelectorKind::Model(a)) => {
            if a == b {
                DdcSelectorRelation::MayOverlap
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        (DdcSelectorKind::Serial(a), DdcSelectorKind::SerialModel { serial: b, .. })
        | (DdcSelectorKind::SerialModel { serial: b, .. }, DdcSelectorKind::Serial(a)) => {
            if a == b {
                DdcSelectorRelation::MayOverlap
            } else {
                DdcSelectorRelation::ProvablyDisjoint
            }
        }
        _ => DdcSelectorRelation::MayOverlap,
    })
}

pub(crate) fn selector_matches_discovery(
    selector: &MonitorSelector,
    monitor: &crate::discovery::DdcMonitorDiscovery,
) -> Result<bool, BackendError> {
    let plan = selector_plan(selector)?;
    let matches = match &plan.kind {
        DdcSelectorKind::Bus(bus) => monitor.bus_number == Some(u32::from(*bus)),
        DdcSelectorKind::Serial(serial) => {
            normalized_compare(&monitor.serial).is_some_and(|value| value == *serial)
        }
        DdcSelectorKind::SerialModel { serial, model } => {
            normalized_compare(&monitor.serial).is_some_and(|value| value == *serial)
                && normalized_compare(&monitor.model).is_some_and(|value| value == *model)
        }
        DdcSelectorKind::Model(model) => {
            normalized_compare(&monitor.model).is_some_and(|value| value == *model)
        }
        DdcSelectorKind::Edid(_) => false,
    };
    Ok(matches)
}

pub(crate) fn selector_match_count(
    selector: &MonitorSelector,
    monitors: &[crate::discovery::DdcMonitorDiscovery],
) -> Result<usize, BackendError> {
    let mut count = 0;
    for monitor in monitors {
        if selector_matches_discovery(selector, monitor)? {
            count += 1;
        }
    }
    Ok(count)
}

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn normalized_compare(value: &Option<String>) -> Option<String> {
    value.as_deref().map(normalize_compare)
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

    use super::{
        apply_with_runner, apply_with_runner_in, build_selector, read_with_runner_in,
        selector_relation, verified_bus_selection, DdcSelectorRelation, EdidIdentity,
    };

    /// A minimal EDID base block with a monitor name and serial descriptor.
    fn edid(model: &str, serial: &str) -> Vec<u8> {
        let mut bytes = vec![0_u8; 128];
        bytes[..8].copy_from_slice(&[0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00]);
        for (slot, (tag, text)) in [(0xfc_u8, model), (0xff, serial)].into_iter().enumerate() {
            let start = 54 + 18 * slot;
            bytes[start + 3] = tag;
            let mut field = text.as_bytes().to_vec();
            field.push(0x0a);
            field.resize(13, b' ');
            bytes[start + 5..start + 18].copy_from_slice(&field);
        }
        bytes
    }

    fn connector(
        root: &std::path::Path,
        name: &str,
        status: &str,
        edid: &[u8],
    ) -> std::path::PathBuf {
        let directory = root.join(name);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("status"), format!("{status}\n")).unwrap();
        std::fs::write(directory.join("edid"), edid).unwrap();
        directory
    }

    fn identified(connector: &str, model: &str, serial: Option<&str>) -> MonitorSelector {
        MonitorSelector {
            connector: Some(String::from(connector)),
            model: Some(String::from(model)),
            serial: serial.map(String::from),
            ..MonitorSelector::default()
        }
    }

    #[test]
    fn bus_only_read_never_falls_back_to_the_identity_search() {
        let root = tempfile::tempdir().unwrap();
        let dp = connector(
            root.path(),
            "card1-DP-1",
            "connected",
            &edid("Mi Monitor", "X1"),
        );
        std::fs::create_dir(dp.join("i2c-7")).unwrap();
        let monitor = ddc_monitor(identified("card1-DP-1", "Mi Monitor", None));

        let awake = FakeRunner::new().with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "7", "getvcp", "10"],
            "VCP 10 C 41 100\n",
        );
        let observed =
            super::read_verified_bus_in(&awake, &monitor, Duration::from_secs(1), root.path())
                .expect("read")
                .expect("verified bus");
        assert_eq!(observed.percent, 41);

        // A waking monitor fails on its bus; no model-based lookup is tried.
        let waking = FakeRunner::new().with_output(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "7", "getvcp", "10"],
            Some(1),
            "",
            "No monitor detected on bus /dev/i2c-7",
        );
        assert!(super::read_verified_bus_in(
            &waking,
            &monitor,
            Duration::from_secs(1),
            root.path()
        )
        .is_err());
        assert_eq!(waking.calls().len(), 1);

        // Without a verifiable connector there is nothing to probe.
        let unverified = ddc_monitor(identified("card1-DP-9", "Mi Monitor", None));
        let none = FakeRunner::new();
        assert!(super::read_verified_bus_in(
            &none,
            &unverified,
            Duration::from_secs(1),
            root.path()
        )
        .expect("no error")
        .is_none());
        assert!(none.calls().is_empty());
    }

    #[test]
    fn a_powered_off_display_is_reported_without_touching_the_bus() {
        let root = tempfile::tempdir().unwrap();
        let dp = connector(
            root.path(),
            "card1-DP-1",
            "connected",
            &edid("Mi Monitor", "X1"),
        );
        std::fs::create_dir(dp.join("i2c-7")).unwrap();
        let monitor = ddc_monitor(identified("card1-DP-1", "Mi Monitor", None));
        let read = || {
            let runner = FakeRunner::new().with_success(
                "ddcutil",
                &["--noconfig", "--terse", "--bus", "7", "getvcp", "10"],
                "VCP 10 C 41 100\n",
            );
            let result =
                super::read_verified_bus_in(&runner, &monitor, Duration::from_secs(1), root.path());
            (result, runner.calls().len())
        };

        // No `dpms` attribute: the kernel tells us nothing, so DDC is used.
        let (result, calls) = read();
        assert_eq!(result.expect("read").map(|value| value.percent), Some(41));
        assert_eq!(calls, 1);

        std::fs::write(dp.join("dpms"), "On\n").unwrap();
        let (result, calls) = read();
        assert_eq!(result.expect("read").map(|value| value.percent), Some(41));
        assert_eq!(calls, 1);

        // Powered off: reported as unreachable with no ddcutil call at all.
        std::fs::write(dp.join("dpms"), "Off\n").unwrap();
        let (result, calls) = read();
        assert!(result.is_err());
        assert_eq!(calls, 0, "no bus traffic while powered off");

        // A disconnected connector has nothing to verify against.
        std::fs::write(dp.join("dpms"), "On\n").unwrap();
        std::fs::write(dp.join("status"), "disconnected\n").unwrap();
        let (result, calls) = read();
        assert!(result.expect("no verified connector").is_none());
        assert_eq!(calls, 0);
    }

    #[test]
    fn edid_identity_reads_name_and_serial_descriptors() {
        let identity = EdidIdentity::parse(&edid("LEN P24h-20", "V305PTDA")).unwrap();
        assert_eq!(identity.model.as_deref(), Some("LEN P24h-20"));
        assert_eq!(identity.serial.as_deref(), Some("V305PTDA"));
        assert_eq!(identity.hex.len(), 256);
        assert!(EdidIdentity::parse(&[0_u8; 128]).is_none());
        assert!(EdidIdentity::parse(&edid("A", "B")[..100]).is_none());
    }

    #[test]
    fn bus_is_used_only_when_the_connector_edid_proves_identity() {
        let root = tempfile::tempdir().unwrap();
        let dp = connector(
            root.path(),
            "card1-DP-1",
            "connected",
            &edid("Mi Monitor", "X1"),
        );
        std::fs::create_dir(dp.join("i2c-7")).unwrap();
        std::os::unix::fs::symlink("../../../i2c-2", dp.join("ddc")).unwrap();
        let hdmi = connector(
            root.path(),
            "card1-HDMI-A-1",
            "connected",
            &edid("Other", "S2"),
        );
        std::os::unix::fs::symlink("../../../i2c-5", hdmi.join("ddc")).unwrap();
        connector(
            root.path(),
            "card1-DP-2",
            "disconnected",
            &edid("Mi Monitor", "X1"),
        );

        let plan = |selector: &MonitorSelector| {
            let identity = build_selector(selector).unwrap();
            verified_bus_selection(&identity, selector, root.path()).map(|plan| plan.args.join(" "))
        };
        // DisplayPort: the AUX adapter inside the connector wins over `ddc`.
        assert_eq!(
            plan(&identified("card1-DP-1", "Mi Monitor", None)).as_deref(),
            Some("--bus 7")
        );
        assert_eq!(
            plan(&identified("card1-DP-1", "Mi Monitor", Some("X1"))).as_deref(),
            Some("--bus 7")
        );
        // Other connectors use their `ddc` adapter.
        assert_eq!(
            plan(&identified("card1-HDMI-A-1", "Other", None)).as_deref(),
            Some("--bus 5")
        );
        // Any mismatching identity keeps ddcutil's own matching.
        assert_eq!(
            plan(&identified("card1-DP-1", "Mi Monitor", Some("X2"))),
            None
        );
        assert_eq!(plan(&identified("card1-DP-1", "Other", None)), None);
        assert_eq!(plan(&identified("card1-DP-2", "Mi Monitor", None)), None);
        assert_eq!(plan(&identified("card1-DP-9", "Mi Monitor", None)), None);
        assert_eq!(plan(&identified("../card1-DP-1", "Mi Monitor", None)), None);
        // Without an identity field the connector alone proves nothing.
        let connector_only = MonitorSelector {
            connector: Some(String::from("card1-DP-1")),
            ddc_bus: Some(3),
            ..MonitorSelector::default()
        };
        assert_eq!(plan(&connector_only), None);
    }

    #[test]
    fn verified_bus_writes_directly_and_falls_back_when_the_bus_fails() {
        let root = tempfile::tempdir().unwrap();
        let dp = connector(
            root.path(),
            "card1-DP-1",
            "connected",
            &edid("Mi Monitor", "X1"),
        );
        std::fs::create_dir(dp.join("i2c-7")).unwrap();
        let monitor = ddc_monitor(identified("card1-DP-1", "Mi Monitor", None));

        let fast = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "7",
                "setvcp",
                "10",
                "40",
            ],
            "",
        );
        let write = apply_with_runner_in(&fast, &monitor, 40, Duration::from_secs(4), root.path())
            .expect("bus write");
        assert_eq!(write.attempts, 1);
        assert!(write.detail.contains("bus 7"), "{}", write.detail);

        let fallback = FakeRunner::new()
            .with_output(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "7",
                    "setvcp",
                    "10",
                    "40",
                ],
                Some(1),
                "",
                "No monitor detected on bus /dev/i2c-7",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--model",
                    "Mi Monitor",
                    "setvcp",
                    "10",
                    "40",
                ],
                "",
            );
        let write =
            apply_with_runner_in(&fallback, &monitor, 40, Duration::from_secs(4), root.path())
                .expect("fallback write");
        assert_eq!(write.attempts, 2);

        let read = FakeRunner::new()
            .with_output(
                "ddcutil",
                &["--noconfig", "--terse", "--bus", "7", "getvcp", "10"],
                Some(1),
                "",
                "No monitor detected on bus /dev/i2c-7",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--terse",
                    "--model",
                    "Mi Monitor",
                    "getvcp",
                    "10",
                ],
                "VCP 10 C 33 100\n",
            );
        let observed = read_with_runner_in(&read, &monitor, Duration::from_secs(4), root.path())
            .expect("fallback read");
        assert_eq!(observed.percent, 33);
    }

    #[test]
    fn selector_semantics_distinguish_equality_from_overlap() {
        let model = MonitorSelector {
            model: Some(String::from("U2720Q")),
            ..MonitorSelector::default()
        };
        let serial_model = MonitorSelector {
            serial: Some(String::from("ABC123")),
            model: Some(String::from("U2720Q")),
            ..MonitorSelector::default()
        };
        let other_serial_model = MonitorSelector {
            serial: Some(String::from("BBB")),
            model: Some(String::from("U2720Q")),
            ..MonitorSelector::default()
        };
        let other_model = MonitorSelector {
            model: Some(String::from("OTHER")),
            ..MonitorSelector::default()
        };

        assert_eq!(
            selector_relation(&model, &serial_model).unwrap(),
            DdcSelectorRelation::MayOverlap
        );
        assert_eq!(
            selector_relation(&serial_model, &other_serial_model).unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            selector_relation(&model, &other_model).unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            selector_relation(
                &MonitorSelector {
                    serial: Some(String::from("ABC123")),
                    ..MonitorSelector::default()
                },
                &MonitorSelector {
                    serial: Some(String::from("abc123")),
                    ..MonitorSelector::default()
                },
            )
            .unwrap(),
            DdcSelectorRelation::Equal
        );
        assert_eq!(
            selector_relation(
                &MonitorSelector {
                    ddc_bus: Some(7),
                    ..MonitorSelector::default()
                },
                &MonitorSelector {
                    ddc_bus: Some(8),
                    ..MonitorSelector::default()
                },
            )
            .unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );
        assert_eq!(
            selector_relation(
                &MonitorSelector {
                    ddc_bus: Some(7),
                    ..MonitorSelector::default()
                },
                &MonitorSelector {
                    serial: Some(String::from("ABC123")),
                    ..MonitorSelector::default()
                },
            )
            .unwrap(),
            DdcSelectorRelation::MayOverlap
        );

        let mi_model = MonitorSelector {
            model: Some(String::from("Mi Monitor")),
            connector: Some(String::from("card1-DP-1")),
            ..MonitorSelector::default()
        };
        let len_serial_model = MonitorSelector {
            serial: Some(String::from("V305PTDA")),
            model: Some(String::from("LEN P24h-20")),
            connector: Some(String::from("card1-DP-3")),
            ddc_bus: Some(9),
            ..MonitorSelector::default()
        };
        assert_eq!(
            selector_relation(&mi_model, &len_serial_model).unwrap(),
            DdcSelectorRelation::ProvablyDisjoint
        );

        let model_with_bus = MonitorSelector {
            model: Some(String::from("Mi Monitor")),
            ddc_bus: Some(7),
            ..MonitorSelector::default()
        };
        let plan = super::selector_plan(&model_with_bus).expect("model selector should be valid");
        assert_eq!(plan.args, ["--bus", "7"]);
    }

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

        let result = apply_with_runner(&runner, &monitor, 64, Duration::from_secs(4))
            .expect("ddc write should succeed");

        assert_eq!(result.applied_percent, 64);
        assert_eq!(result.attempts, 2);

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].contains("|--sn|ABC123|--model|U2720Q|setvcp|10|64"));
        assert!(!calls[0].contains("|--bus|7|"));
        assert!(!calls.iter().any(|call| call.contains("getvcp")));
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
        let runner = FakeRunner::new().with_success(
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

        let result = apply_with_runner(&runner, &monitor, 42, Duration::from_secs(4))
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
    fn reads_current_brightness_from_terse_vcp_output() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: None,
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(7),
            ddc_address: None,
        });
        let runner = FakeRunner::new().with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "7", "getvcp", "10"],
            "VCP 10 C 37 100\n",
        );
        let observation =
            super::read_with_runner(&runner, &monitor, Duration::from_secs(4)).expect("read");
        assert_eq!(observation.percent, 37);
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

        let error = apply_with_runner(&runner, &monitor, 55, Duration::from_secs(4))
            .expect_err("second timeout should fail");

        assert!(error.to_string().contains("timed out"));
        assert_eq!(runner.calls().len(), 2);
    }

    #[test]
    fn legacy_ddcutil_without_noconfig_falls_back_and_succeeds() {
        let monitor = ddc_monitor(MonitorSelector {
            connector: None,
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: Some(9),
            ddc_address: None,
        });

        // Test write fallback
        let write_runner = FakeRunner::new()
            .with_output(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "9",
                    "setvcp",
                    "10",
                    "60",
                ],
                Some(1),
                "",
                "ddcutil option parsing failed: Unknown option --noconfig\n",
            )
            .with_success(
                "ddcutil",
                &["--noverify", "--bus", "9", "setvcp", "10", "60"],
                "",
            );

        let write_result = apply_with_runner(&write_runner, &monitor, 60, Duration::from_secs(4))
            .expect("write with fallback should succeed");
        assert_eq!(write_result.applied_percent, 60);

        // Test read fallback
        let read_runner = FakeRunner::new()
            .with_output(
                "ddcutil",
                &["--noconfig", "--terse", "--bus", "9", "getvcp", "10"],
                Some(1),
                "",
                "ddcutil option parsing failed: Unknown option --noconfig\n",
            )
            .with_success(
                "ddcutil",
                &["--terse", "--bus", "9", "getvcp", "10"],
                "VCP 10 C 60 100\n",
            );

        let read_result = super::read_with_runner(&read_runner, &monitor, Duration::from_secs(4))
            .expect("read with fallback should succeed");
        assert_eq!(read_result.percent, 60);
    }

    #[test]
    fn legacy_fallback_does_not_mask_real_errors() {
        let monitor = ddc_monitor(MonitorSelector {
            ddc_bus: Some(9),
            ..MonitorSelector::default()
        });

        // Permission denied must fail without retrying without --noconfig
        let perm_runner = FakeRunner::new().with_output(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "9",
                "setvcp",
                "10",
                "60",
            ],
            Some(1),
            "",
            "Error: Permission denied accessing /dev/i2c-9\n",
        );

        let perm_err = apply_with_runner(&perm_runner, &monitor, 60, Duration::from_secs(4))
            .expect_err("permission denied must fail immediately");
        assert!(perm_err.to_string().contains("Permission denied"));

        // Unsupported VCP must fail without retrying without --noconfig
        let vcp_runner = FakeRunner::new().with_output(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "9", "getvcp", "10"],
            Some(1),
            "",
            "VCP feature 0x10 is not supported\n",
        );

        let vcp_err = super::read_with_runner(&vcp_runner, &monitor, Duration::from_secs(4))
            .expect_err("unsupported VCP must fail immediately");
        assert!(vcp_err.to_string().contains("not supported"));
    }

    fn ddc_monitor(selector: MonitorSelector) -> MonitorConfig {
        MonitorConfig {
            logical_id: String::from("desk"),
            backend: crate::backends::BackendKind::Ddc,
            enabled: true,
            allow_topology_retargeting: false,
            min_pct: 0,
            max_pct: 100,
            gain: 1.0,
            transition_gamma: 1.4,
            milestone_adjustments: Vec::new(),
            selector,
        }
    }
}
