use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io;
use std::path::Path;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(unix)]
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;

use crate::config::{MonitorConfig, MonitorSelector};
use crate::ddcutil::DdcutilClient;

#[cfg(any(target_os = "linux", test))]
use super::CommandOutput;
use super::{
    clamp_percent, command_failure, map_command_error, BackendError, BackendKind, BackendWrite,
    ProcessRunner, RealProcessRunner,
};

const DDC_BRIGHTNESS_VCP_CODE: &str = "10";
/// Upper bound for the extra reads of the write path (max lookup, readback
/// after a failed write), so they never add more than a few seconds.
const AUX_READ_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_DDC_ADDRESS: u16 = 0x37;
#[cfg(unix)]
const DDC_LOCK_FILE_NAME: &str = "sunreactor_i2c.lock";
#[cfg(unix)]
static DDC_LOCK_OPEN_WARNED: AtomicBool = AtomicBool::new(false);
/// Serializes DDC traffic between threads of this process. The apply engine
/// dispatches monitors concurrently; without this queue sibling writers would
/// burn their per-command timeout polling the flock and fail into backoff.
#[cfg(unix)]
static DDC_PROCESS_LOCK: Mutex<()> = Mutex::new(());

/// VCP 0x10 maximum learned from the last successful read, per monitor.
/// Writes scale percent to this range; an unknown monitor is assumed to use
/// the common 0..=100 range.
static VCP_MAX_CACHE: Mutex<BTreeMap<String, u16>> = Mutex::new(BTreeMap::new());

/// Monitors whose VCP maximum was already looked up by the write path. The
/// lookup (one bounded getvcp) happens at most once per monitor per process,
/// whether or not it succeeded, so an unreadable monitor is never polled.
static VCP_MAX_PROBED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

fn max_cache() -> MutexGuard<'static, BTreeMap<String, u16>> {
    VCP_MAX_CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn max_cache_key(selector: &MonitorSelector, identity: &DdcSelectorPlan) -> String {
    format!(
        "{}|{}",
        selector.connector.as_deref().unwrap_or("").trim(),
        identity.description
    )
}

fn learned_max(key: &str) -> u16 {
    max_cache().get(key).copied().unwrap_or(100)
}

/// Raw VCP value for a brightness percent: `round(percent * max / 100)`.
/// Exact for `max == 100`. For `max < 100` several percents share one raw
/// step, so percent -> raw -> percent is only stable to that resolution.
fn percent_to_raw(percent: u8, max: u16) -> u16 {
    let raw = (u32::from(percent.min(100)) * u32::from(max) + 50) / 100;
    u16::try_from(raw).unwrap_or(max)
}

/// The percent a monitor with this `max` reads back after `percent` is
/// written. Equal to `percent` for every value when `max >= 100`; for
/// `max < 100` it snaps to the raw step (e.g. max 50 turns odd percents into
/// the next even one). `quantize(quantize(p)) == quantize(p)`, so callers
/// that compare a readback with a wanted value must compare against the
/// reported `applied_percent` (which is already quantized), not the raw wish,
/// or they would rewrite the panel for nothing.
#[cfg(test)]
fn quantize_percent(percent: u8, max: u16) -> u8 {
    raw_to_percent(percent_to_raw(percent, max), max)
}

/// Brightness percent for a raw reading: `round(current * 100 / max)`.
fn raw_to_percent(current: u16, max: u16) -> u8 {
    let percent = (u32::from(current) * 100 + u32::from(max) / 2) / u32::from(max.max(1));
    u8::try_from(percent.min(100)).unwrap_or(100)
}

/// Exclusive DDC access held for one ddcutil invocation.
///
/// Fields drop in declaration order: the cross-process flock is released
/// before the next in-process writer is let through. The lock is reentrant per
/// thread: a thread already holding it gets a no-op guard, so nested helpers
/// (a read inside a write, a locked runner inside a locked section) can never
/// deadlock on the non-reentrant process mutex.
#[cfg(unix)]
#[derive(Debug)]
struct DdcLock {
    _file: Option<File>,
    _process: Option<MutexGuard<'static, ()>>,
    owner: bool,
}

#[cfg(unix)]
impl Drop for DdcLock {
    fn drop(&mut self) {
        if self.owner {
            DDC_LOCK_HELD.with(|held| held.set(false));
        }
    }
}

#[cfg(unix)]
thread_local! {
    static DDC_LOCK_HELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runner decorator that serializes every `ddcutil` invocation (detect,
/// capabilities, getvcp, setvcp) through the DDC process lock, so discovery
/// and reads never interleave with writes on the I2C bus. Other programs pass
/// through untouched. Safe to nest inside an already locked section.
pub(crate) struct DdcLockedRunner<'a, R: ProcessRunner> {
    inner: &'a R,
    #[cfg(unix)]
    lock_path: Option<std::path::PathBuf>,
}

impl<'a, R: ProcessRunner> DdcLockedRunner<'a, R> {
    pub(crate) fn new(inner: &'a R) -> Self {
        Self {
            inner,
            #[cfg(unix)]
            lock_path: None,
        }
    }

    #[cfg(all(unix, test))]
    fn with_lock_path(inner: &'a R, lock_path: std::path::PathBuf) -> Self {
        Self {
            inner,
            lock_path: Some(lock_path),
        }
    }
}

impl<R: ProcessRunner> ProcessRunner for DdcLockedRunner<'_, R> {
    fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<crate::process::CommandOutput, crate::process::CommandError> {
        #[cfg(unix)]
        let _lock = if program == "ddcutil" {
            Some(
                match &self.lock_path {
                    Some(path) => acquire_ddc_lock_at(path, timeout),
                    None => acquire_ddc_lock(timeout),
                }
                .map_err(|error| crate::process::CommandError::Io {
                    program: program.to_owned(),
                    message: error.to_string(),
                })?,
            )
        } else {
            None
        };
        self.inner.run(program, args, timeout)
    }
}

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
    let cache_key = max_cache_key(&monitor.selector, &identity_selection);

    let mut attempts = 0u8;

    loop {
        let selection = if let (Some(selection), true) = (&bus_selection, use_bus) {
            selection
        } else {
            refuse_ambiguous_identity(&monitor.selector, drm)?;
            &identity_selection
        };

        // One lock section per attempt: the maximum lookup, the write and the
        // post-failure readback must not interleave with other DDC traffic.
        #[cfg(unix)]
        let _lock = acquire_ddc_lock(timeout).map_err(|error| error.with_attempts(attempts))?;

        ensure_max_known(runner, monitor, selection, &cache_key, timeout, drm);
        let max = learned_max(&cache_key);
        let raw = percent_to_raw(percent, max);
        let args = build_setvcp_args(selection, raw);

        attempts += 1;
        let run_result = DdcutilClient::run_with_compatible_arguments(runner, &args, timeout);

        let error = match run_result {
            Ok(output) if output.success() => {
                return Ok(BackendWrite {
                    backend: BackendKind::Ddc,
                    applied_percent: raw_to_percent(raw, max),
                    attempts,
                    detail: format!("applied via ddcutil using {}", selection.description),
                });
            }
            Ok(output) => command_failure(BackendKind::Ddc, "ddcutil", &output),
            Err(error) => map_command_error(BackendKind::Ddc, error),
        };
        if attempts == 1 && (use_bus || should_retry(&error)) {
            // The failed command may still have taken effect (a non-zero exit
            // after the value was set). One cheap read decides: if the panel
            // already holds the target, a second write is pure EEPROM wear.
            if let Some(applied_percent) =
                readback_matches_target(runner, selection, &cache_key, percent, timeout)
            {
                return Ok(BackendWrite {
                    backend: BackendKind::Ddc,
                    applied_percent,
                    attempts,
                    detail: format!(
                        "applied via ddcutil using {} (confirmed by readback after a failed exit)",
                        selection.description
                    ),
                });
            }
            // A bus that stopped answering falls back to ddcutil's own
            // identity matching for the retry.
            use_bus = false;
            continue;
        }
        return Err(error.with_attempts(attempts));
    }
}

fn getvcp_args(selection: &DdcSelectorPlan) -> Vec<String> {
    let mut args = vec![String::from("--noconfig"), String::from("--terse")];
    args.extend(selection.args.iter().cloned());
    args.push(String::from("getvcp"));
    args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
    args
}

/// Learns the monitor's VCP maximum before its first write, so a forced first
/// write to a max=255 monitor does not use the 0..=100 assumption. At most one
/// bounded getvcp per monitor per process; it is skipped when the maximum is
/// already known from a read, or the display is powered off. Failure keeps the
/// 100 default (the pre-existing behaviour) and is never retried.
fn ensure_max_known<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    selection: &DdcSelectorPlan,
    cache_key: &str,
    timeout: Duration,
    drm: &Path,
) {
    if max_cache().contains_key(cache_key) {
        return;
    }
    if !VCP_MAX_PROBED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(cache_key.to_owned())
    {
        return;
    }
    if connector_is_powered_off(&monitor.selector, drm) {
        return;
    }
    let args = getvcp_args(selection);
    let learned =
        DdcutilClient::run_with_compatible_arguments(runner, &args, timeout.min(AUX_READ_TIMEOUT))
            .ok()
            .filter(CommandOutput::success)
            .and_then(|output| {
                crate::ddcutil::parser::parse_getvcp_brightness(&output.stdout).ok()
            });
    if let Some(value) = learned {
        max_cache().insert(cache_key.to_owned(), value.maximum);
    } else {
        tracing::warn!(
            monitor = %monitor.logical_id,
            "ddc_vcp_max_unknown_before_first_write; assuming 0..=100"
        );
    }
}

/// One getvcp after a failed setvcp. `Some(applied_percent)` when the monitor
/// already reports the raw value the target maps to.
fn readback_matches_target<R: ProcessRunner>(
    runner: &R,
    selection: &DdcSelectorPlan,
    cache_key: &str,
    percent: u8,
    timeout: Duration,
) -> Option<u8> {
    let args = getvcp_args(selection);
    let output =
        DdcutilClient::run_with_compatible_arguments(runner, &args, timeout.min(AUX_READ_TIMEOUT))
            .ok()?;
    if !output.success() {
        return None;
    }
    let value = crate::ddcutil::parser::parse_getvcp_brightness(&output.stdout).ok()?;
    max_cache().insert(cache_key.to_owned(), value.maximum);
    let target_raw = percent_to_raw(percent, value.maximum);
    (value.current == target_raw).then(|| raw_to_percent(target_raw, value.maximum))
}

#[cfg(unix)]
fn acquire_ddc_lock(timeout: Duration) -> Result<DdcLock, BackendError> {
    // Unit tests must not contend with (or touch) a real daemon's lock file.
    #[cfg(test)]
    let lock_path = std::env::temp_dir().join(format!(
        "sunreactor-test-{}-{DDC_LOCK_FILE_NAME}",
        std::process::id()
    ));
    #[cfg(not(test))]
    let lock_path = crate::paths::runtime_socket_dir().map_or_else(
        |_| {
            let uid = rustix::process::geteuid().as_raw();
            std::path::PathBuf::from(format!("/run/user/{uid}/sunreactor/{DDC_LOCK_FILE_NAME}"))
        },
        |path| path.join(DDC_LOCK_FILE_NAME),
    );
    acquire_ddc_lock_at(&lock_path, timeout)
}

#[cfg(unix)]
/// Waits (unbounded) for the in-process DDC queue, then takes the
/// cross-process flock within `timeout`.
fn acquire_ddc_lock_at(lock_path: &Path, timeout: Duration) -> Result<DdcLock, BackendError> {
    if DDC_LOCK_HELD.with(std::cell::Cell::get) {
        return Ok(DdcLock {
            _file: None,
            _process: None,
            owner: false,
        });
    }
    // The mutex guards no data, so a panic in a previous holder leaves nothing
    // inconsistent; recover the guard instead of propagating the poison.
    let process = DDC_PROCESS_LOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let lock_file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(lock_path)
    {
        Ok(file) => file,
        Err(error) => {
            if !DDC_LOCK_OPEN_WARNED.swap(true, Ordering::AcqRel) {
                tracing::warn!(path = %lock_path.display(), error = %error, "ddc_lock_unavailable_continuing_without_lock");
            }
            DDC_LOCK_HELD.with(|held| held.set(true));
            return Ok(DdcLock {
                _file: None,
                _process: Some(process),
                owner: true,
            });
        }
    };

    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    loop {
        match rustix::fs::flock(
            &lock_file,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        ) {
            Ok(()) => {
                DDC_LOCK_HELD.with(|held| held.set(true));
                return Ok(DdcLock {
                    _file: Some(lock_file),
                    _process: Some(process),
                    owner: true,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(BackendError::CommandTimeout {
                        backend: BackendKind::Ddc,
                        program: String::from(DDC_LOCK_FILE_NAME),
                        after: timeout,
                        detail: format!("timed out acquiring {}", lock_path.display()),
                        attempts: 1,
                    });
                }
                std::thread::sleep(Duration::from_millis(25).min(remaining));
            }
            Err(error) => {
                return Err(BackendError::Io {
                    backend: BackendKind::Ddc,
                    program: String::from(DDC_LOCK_FILE_NAME),
                    message: format!("failed to acquire {}: {error}", lock_path.display()),
                    attempts: 1,
                });
            }
        }
    }
}

#[cfg(any(target_os = "linux", test))]
pub(crate) fn read_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
) -> Result<super::BackendObservation, BackendError> {
    read_with_runner_in(runner, monitor, timeout, drm_root())
}

#[cfg(any(target_os = "linux", test))]
fn read_with_runner_in<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
    drm: &Path,
) -> Result<super::BackendObservation, BackendError> {
    let runner = &DdcLockedRunner::new(runner);
    let identity_selection = build_selector(&monitor.selector)?;
    let cache_key = max_cache_key(&monitor.selector, &identity_selection);
    let read = |selection: &DdcSelectorPlan| {
        let mut args = vec![String::from("--noconfig"), String::from("--terse")];
        args.extend(selection.args.iter().cloned());
        args.push(String::from("getvcp"));
        args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
        DdcutilClient::run_with_compatible_arguments(runner, &args, timeout)
    };
    let bus_output = verified_bus_selection(&identity_selection, &monitor.selector, drm)
        .map(|selection| read(&selection))
        .filter(|output| output.as_ref().is_ok_and(CommandOutput::success));
    let output = if let Some(output) = bus_output {
        output
    } else {
        refuse_ambiguous_identity(&monitor.selector, drm)?;
        read(&identity_selection)
    }
    .map_err(|error| map_command_error(BackendKind::Ddc, error))?;
    parse_and_learn(&output, &cache_key)
}

/// Reads brightness only over the EDID-verified bus, never through ddcutil's
/// slow identity search.
///
/// Returns `Ok(None)` when the monitor has no verifiable connector, so callers
/// can leave it to regular ticks. A monitor that is still waking fails fast
/// with an error instead of stalling for seconds.
#[cfg(target_os = "linux")]
pub(crate) fn read_verified_bus_with_runner<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
) -> Result<Option<super::BackendObservation>, BackendError> {
    read_verified_bus_in(runner, monitor, timeout, drm_root())
}

#[cfg(any(target_os = "linux", test))]
fn read_verified_bus_in<R: ProcessRunner>(
    runner: &R,
    monitor: &MonitorConfig,
    timeout: Duration,
    drm: &Path,
) -> Result<Option<super::BackendObservation>, BackendError> {
    let runner = &DdcLockedRunner::new(runner);
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
    let output = DdcutilClient::run_with_compatible_arguments(runner, &args, timeout)
        .map_err(|error| map_command_error(BackendKind::Ddc, error))?;
    let cache_key = max_cache_key(&monitor.selector, &identity_selection);
    parse_and_learn(&output, &cache_key).map(Some)
}

#[cfg(any(target_os = "linux", test))]
fn parse_and_learn(
    output: &CommandOutput,
    cache_key: &str,
) -> Result<super::BackendObservation, BackendError> {
    let (observation, max) = parse_brightness_output(output)?;
    max_cache().insert(cache_key.to_owned(), max);
    Ok(observation)
}

/// Returns the observation and the monitor's VCP maximum. A reading with
/// `max == 0` or `current > max` is an error, not a clamped value.
#[cfg(any(target_os = "linux", test))]
fn parse_brightness_output(
    output: &CommandOutput,
) -> Result<(super::BackendObservation, u16), BackendError> {
    if !output.success() {
        return Err(command_failure(BackendKind::Ddc, "ddcutil", output));
    }
    let value =
        crate::ddcutil::parser::parse_getvcp_brightness(&output.stdout).map_err(|error| {
            BackendError::Io {
                backend: BackendKind::Ddc,
                program: String::from("ddcutil"),
                message: error.to_string(),
                attempts: 1,
            }
        })?;
    Ok((
        super::BackendObservation {
            percent: raw_to_percent(value.current, value.maximum),
        },
        value.maximum,
    ))
}

/// ddcutil's own identity search (`--model`, `--sn`, `--edid`) silently picks
/// the first match. When two connected displays are indistinguishable by the
/// selector's identity fields, that could be the wrong monitor, so the search
/// is refused and only the EDID-verified connector bus may be used.
fn refuse_ambiguous_identity(selector: &MonitorSelector, root: &Path) -> Result<(), BackendError> {
    if identity_match_count(selector, root) > 1 {
        return Err(BackendError::Io {
            backend: BackendKind::Ddc,
            program: String::from("ddcutil"),
            message: String::from(
                "refusing ddcutil identity search: several connected displays match this selector",
            ),
            attempts: 0,
        });
    }
    Ok(())
}

fn identity_match_count(selector: &MonitorSelector, root: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let directory = entry.path();
            let connected = std::fs::read_to_string(directory.join("status"))
                .is_ok_and(|status| status.trim() == "connected");
            connected
                && std::fs::read(directory.join("edid"))
                    .ok()
                    .and_then(|bytes| EdidIdentity::parse(&bytes))
                    .is_some_and(|identity| identity.matches(selector))
        })
        .count()
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

fn build_setvcp_args(selection: &DdcSelectorPlan, raw: u16) -> Vec<String> {
    let mut args = vec![String::from("--noconfig"), String::from("--noverify")];
    args.extend(selection.args.iter().cloned());
    args.push(String::from("setvcp"));
    args.push(String::from(DDC_BRIGHTNESS_VCP_CODE));
    args.push(raw.to_string());
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
/// Whether the kernel reports this monitor's connector as disconnected.
///
/// The sysfs `dpms` and `enabled` attributes are deliberately ignored: they
/// mirror the legacy DPMS property, which atomic-modesetting compositors
/// (KWin, Mutter, wlroots) never update. On amdgpu they read `Off` /
/// `disabled` for displays that are lit, so trusting them silently disabled
/// every wake probe and made each readback look like a power transition.
/// Sleep is detected from DDC itself (repeated failed reads) instead.
#[cfg(any(target_os = "linux", test))]
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
    off("status", "connected")
}

/// Connector power probe for callers outside this module (sysfs only, no DDC).
#[cfg(target_os = "linux")]
pub(crate) fn connector_powered_off(selector: &MonitorSelector) -> bool {
    connector_is_powered_off(selector, drm_root())
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
    #[cfg(unix)]
    use std::fs::OpenOptions;
    use std::time::Duration;

    use crate::backends::testutil::FakeRunner;
    use crate::backends::ProcessRunner;
    use crate::config::{MonitorConfig, MonitorSelector};

    use super::{
        apply_with_runner, apply_with_runner_in, read_with_runner_in, selector_relation,
        DdcSelectorRelation, EdidIdentity,
    };
    #[cfg(target_os = "linux")]
    use super::{build_selector, verified_bus_selection};

    #[cfg(unix)]
    #[test]
    fn ddc_lock_times_out_without_blocking_indefinitely() {
        let temp = tempfile::tempdir().expect("tempdir");
        let lock_path = temp.path().join("ddc.lock");
        let holder = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .expect("lock file");
        rustix::fs::flock(&holder, rustix::fs::FlockOperation::LockExclusive)
            .expect("holder should acquire lock");

        let started = std::time::Instant::now();
        let error = super::acquire_ddc_lock_at(&lock_path, Duration::from_millis(60))
            .expect_err("contended lock should time out");
        assert!(matches!(error, super::BackendError::CommandTimeout { .. }));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    /// Sibling monitors in one apply run dispatch DDC writes concurrently.
    /// They must queue for the bus in-process instead of spending their
    /// per-command timeout on the cross-process flock and failing.
    #[cfg(unix)]
    #[test]
    fn in_process_ddc_writers_wait_their_turn_instead_of_timing_out() {
        let temp = tempfile::tempdir().expect("tempdir");
        let lock_path = temp.path().join("ddc.lock");
        let holder_path = lock_path.clone();
        let (held_tx, held_rx) = std::sync::mpsc::channel();
        let holder = std::thread::spawn(move || {
            let lock = super::acquire_ddc_lock_at(&holder_path, Duration::from_millis(60))
                .expect("first writer acquires the lock");
            held_tx.send(()).expect("signal held");
            // Longer than the second writer's timeout, like a slow setvcp.
            std::thread::sleep(Duration::from_millis(200));
            drop(lock);
        });
        held_rx.recv().expect("holder took the lock");

        let second = super::acquire_ddc_lock_at(&lock_path, Duration::from_millis(60));
        holder.join().expect("holder thread");
        assert!(
            second.is_ok(),
            "in-process writer should wait for the bus, got {second:?}"
        );
    }

    fn getvcp_args_for_bus(bus: &str) -> [&str; 6] {
        ["--noconfig", "--terse", "--bus", bus, "getvcp", "10"]
    }

    #[test]
    fn first_write_learns_vcp_max_once_before_writing() {
        let monitor = bus_monitor(201);
        let runner = FakeRunner::new()
            .with_success(
                "ddcutil",
                &getvcp_args_for_bus("201"),
                "VCP code 0x10 (Brightness): current value = 10, max value = 255\n",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "201",
                    "setvcp",
                    "10",
                    "128",
                ],
                "",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "201",
                    "setvcp",
                    "10",
                    "255",
                ],
                "",
            );

        let first = apply_with_runner(&runner, &monitor, 50, Duration::from_secs(4))
            .expect("forced first write succeeds");
        assert_eq!(first.attempts, 1);
        let second = apply_with_runner(&runner, &monitor, 100, Duration::from_secs(4))
            .expect("second write succeeds");
        assert_eq!(second.applied_percent, 100);

        let calls = runner.calls();
        assert!(
            calls[0].contains("getvcp"),
            "max is learned before the write"
        );
        assert!(
            calls[1].ends_with("setvcp|10|128"),
            "raw uses max 255: {calls:?}"
        );
        assert_eq!(
            calls.iter().filter(|call| call.contains("getvcp")).count(),
            1,
            "the lookup happens once per monitor per process"
        );
    }

    #[test]
    fn failed_lookup_is_not_retried_and_keeps_default_range() {
        let monitor = bus_monitor(202);
        let runner = FakeRunner::new()
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "202",
                    "setvcp",
                    "10",
                    "40",
                ],
                "",
            )
            .with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    "202",
                    "setvcp",
                    "10",
                    "41",
                ],
                "",
            );
        apply_with_runner(&runner, &monitor, 40, Duration::from_secs(4)).expect("write");
        apply_with_runner(&runner, &monitor, 41, Duration::from_secs(4)).expect("write");
        assert_eq!(
            runner
                .calls()
                .iter()
                .filter(|call| call.contains("getvcp"))
                .count(),
            1
        );
    }

    #[test]
    fn nonzero_setvcp_exit_with_matching_readback_is_not_written_twice() {
        let monitor = bus_monitor(203);
        let setvcp = [
            "--noconfig",
            "--noverify",
            "--bus",
            "203",
            "setvcp",
            "10",
            "30",
        ];
        let runner = FakeRunner::new()
            .with_success(
                "ddcutil",
                &getvcp_args_for_bus("203"),
                "VCP code 0x10 (Brightness): current value = 5, max value = 100\n",
            )
            .with_output("ddcutil", &setvcp, Some(1), "", "Device or resource busy")
            .with_success(
                "ddcutil",
                &getvcp_args_for_bus("203"),
                "VCP code 0x10 (Brightness): current value = 30, max value = 100\n",
            );

        let write = apply_with_runner(&runner, &monitor, 30, Duration::from_secs(4))
            .expect("readback proves the write landed");
        assert_eq!(write.attempts, 1);
        assert_eq!(write.applied_percent, 30);
        assert_eq!(
            runner
                .calls()
                .iter()
                .filter(|call| call.contains("setvcp"))
                .count(),
            1,
            "no second setvcp"
        );
    }

    #[test]
    fn nonzero_setvcp_exit_with_different_readback_retries_and_counts_both_attempts() {
        let monitor = bus_monitor(204);
        let setvcp = [
            "--noconfig",
            "--noverify",
            "--bus",
            "204",
            "setvcp",
            "10",
            "30",
        ];
        let runner = FakeRunner::new()
            .with_success(
                "ddcutil",
                &getvcp_args_for_bus("204"),
                "VCP code 0x10 (Brightness): current value = 5, max value = 100\n",
            )
            .with_output("ddcutil", &setvcp, Some(1), "", "Device or resource busy")
            .with_success(
                "ddcutil",
                &getvcp_args_for_bus("204"),
                "VCP code 0x10 (Brightness): current value = 5, max value = 100\n",
            )
            .with_success("ddcutil", &setvcp, "");

        let write = apply_with_runner(&runner, &monitor, 30, Duration::from_secs(4))
            .expect("retry succeeds");
        assert_eq!(write.attempts, 2, "every actual setvcp is counted");
    }

    #[cfg(unix)]
    #[test]
    fn locked_runner_serializes_ddcutil_and_is_reentrant() {
        let temp = tempfile::tempdir().expect("tempdir");
        let lock_path = temp.path().join("ddc.lock");
        let inner = FakeRunner::new().with_success("ddcutil", &["detect"], "ok");

        // Reentrant: running a locked ddcutil call inside a locked section on
        // the same thread must not deadlock.
        {
            let _held =
                super::acquire_ddc_lock_at(&lock_path, Duration::from_secs(1)).expect("outer lock");
            let runner = super::DdcLockedRunner::with_lock_path(&inner, lock_path.clone());
            runner
                .run("ddcutil", &[String::from("detect")], Duration::from_secs(1))
                .expect("nested call must not deadlock");
        }

        // Cross-thread: the call waits for the holder.
        let inner = FakeRunner::new().with_success("ddcutil", &["detect"], "ok");
        let held = super::acquire_ddc_lock_at(&lock_path, Duration::from_secs(1)).expect("lock");
        let worker_path = lock_path.clone();
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let runner = super::DdcLockedRunner::with_lock_path(&inner, worker_path);
                runner.run("ddcutil", &[String::from("detect")], Duration::from_secs(5))
            });
            std::thread::sleep(Duration::from_millis(150));
            assert!(inner.calls().is_empty(), "detect must wait for the holder");
            drop(held);
            handle
                .join()
                .expect("worker")
                .expect("detect runs after release");
        });
        assert_eq!(inner.calls().len(), 1);
    }

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
    fn only_a_disconnected_connector_skips_the_bus() {
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

        // A stale legacy `dpms`/`enabled` (atomic KMS never updates them) must
        // not stop a lit display from being read.
        std::fs::write(dp.join("dpms"), "Off\n").unwrap();
        std::fs::write(dp.join("enabled"), "disabled\n").unwrap();
        let (result, calls) = read();
        assert_eq!(result.expect("read").map(|value| value.percent), Some(41));
        assert_eq!(calls, 1, "legacy dpms is ignored");

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

    #[cfg(target_os = "linux")]
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
        let writes: Vec<_> = calls
            .iter()
            .filter(|call| call.contains("setvcp"))
            .collect();
        assert_eq!(writes.len(), 2);
        assert!(writes[0].contains("|--sn|ABC123|--model|U2720Q|setvcp|10|64"));
        assert!(!writes[0].contains("|--bus|7|"));
        // Reads stay bounded: one max lookup plus one readback before the retry.
        assert!(calls.iter().filter(|call| call.contains("getvcp")).count() <= 2);
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
        let calls = runner.calls();
        assert_eq!(
            calls.iter().filter(|call| call.contains("setvcp")).count(),
            2
        );
        assert!(calls.iter().filter(|call| call.contains("getvcp")).count() <= 2);
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
            .with_success("ddcutil", &["--help"], "--brief")
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
            .with_success("ddcutil", &["--bus", "9", "setvcp", "10", "60"], "");

        let write_result = apply_with_runner(&write_runner, &monitor, 60, Duration::from_secs(4))
            .expect("write with fallback should succeed");
        assert_eq!(write_result.applied_percent, 60);

        // Test read fallback
        let read_runner = FakeRunner::new()
            .with_success("ddcutil", &["--help"], "--brief")
            .with_output(
                "ddcutil",
                &["--noconfig", "--terse", "--bus", "9", "getvcp", "10"],
                Some(1),
                "",
                "ddcutil option parsing failed: Unknown option --noconfig\n",
            )
            .with_success(
                "ddcutil",
                &["--brief", "--bus", "9", "getvcp", "10"],
                "VCP code 0x10 (Brightness): current value = 60, max value = 100\n",
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

    fn bus_monitor(bus: u8) -> MonitorConfig {
        ddc_monitor(MonitorSelector {
            ddc_bus: Some(bus),
            ..MonitorSelector::default()
        })
    }

    const NO_DRM: &str = "/nonexistent-drm";

    #[test]
    fn percent_raw_scaling_round_trips_exactly_for_max_100() {
        for percent in 0..=100u8 {
            let raw = super::percent_to_raw(percent, 100);
            assert_eq!(raw, u16::from(percent));
            assert_eq!(super::raw_to_percent(raw, 100), percent);
        }
        // max >= 100 is stable too; max < 100 only to the raw resolution.
        for percent in 0..=100u8 {
            let raw = super::percent_to_raw(percent, 255);
            assert_eq!(super::raw_to_percent(raw, 255), percent);
        }
        assert_eq!(super::percent_to_raw(100, 50), 50);
        assert_eq!(super::percent_to_raw(50, 50), 25);
        assert_eq!(super::raw_to_percent(25, 50), 50);
        assert_eq!(super::raw_to_percent(7, 7), 100);
    }

    #[test]
    fn write_then_read_is_idempotent_so_no_redundant_writes() {
        for percent in 0..=100u8 {
            assert_eq!(super::quantize_percent(percent, 100), percent);
            assert_eq!(super::quantize_percent(percent, 255), percent);
        }
        // max 50 cannot represent every percent; the readback is the
        // quantized value and stays a fixed point.
        for percent in 0..=100u8 {
            let once = super::quantize_percent(percent, 50);
            assert_eq!(super::quantize_percent(once, 50), once);
            assert!(once.abs_diff(percent) <= 1);
        }
    }

    #[test]
    fn applied_percent_reports_what_the_monitor_will_read_back() {
        let drm = std::path::Path::new(NO_DRM);
        let monitor = bus_monitor(35);
        let read = FakeRunner::new().with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "35", "getvcp", "10"],
            "VCP 10 C 10 50\n",
        );
        read_with_runner_in(&read, &monitor, Duration::from_secs(4), drm).expect("read");
        let write = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "35",
                "setvcp",
                "10",
                "26",
            ],
            "",
        );
        let result =
            apply_with_runner_in(&write, &monitor, 51, Duration::from_secs(4), drm).expect("write");
        assert_eq!(result.applied_percent, 52);
        assert_eq!(write.calls().len(), 1);
    }

    #[test]
    fn read_scales_by_max_and_the_learned_max_scales_later_writes() {
        let drm = std::path::Path::new(NO_DRM);
        let monitor = bus_monitor(31);
        let read = FakeRunner::new().with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "31", "getvcp", "10"],
            "VCP 10 C 128 255\n",
        );
        let observed =
            read_with_runner_in(&read, &monitor, Duration::from_secs(4), drm).expect("read");
        assert_eq!(observed.percent, 50);

        let write = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "31",
                "setvcp",
                "10",
                "128",
            ],
            "",
        );
        apply_with_runner_in(&write, &monitor, 50, Duration::from_secs(4), drm).expect("write");
    }

    #[test]
    fn write_uses_learned_max_and_unknown_monitors_default_to_100() {
        let drm = std::path::Path::new(NO_DRM);
        let monitor = bus_monitor(32);
        let read = FakeRunner::new().with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "32", "getvcp", "10"],
            "VCP 10 C 25 50\n",
        );
        let observed =
            read_with_runner_in(&read, &monitor, Duration::from_secs(4), drm).expect("read");
        assert_eq!(observed.percent, 50);
        let write = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "32",
                "setvcp",
                "10",
                "50",
            ],
            "",
        );
        apply_with_runner_in(&write, &monitor, 100, Duration::from_secs(4), drm).expect("write");

        let unknown = bus_monitor(33);
        let write = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "33",
                "setvcp",
                "10",
                "73",
            ],
            "",
        );
        apply_with_runner_in(&write, &unknown, 73, Duration::from_secs(4), drm).expect("write");
    }

    #[test]
    fn impossible_readbacks_are_errors_not_values() {
        let drm = std::path::Path::new(NO_DRM);
        let monitor = bus_monitor(34);
        for reply in [
            "VCP 10 C 5 0\n",
            "VCP 10 C 101 100\n",
            "VCP 10 ERR\n",
            "",
            "Display not found\n",
        ] {
            let runner = FakeRunner::new().with_success(
                "ddcutil",
                &["--noconfig", "--terse", "--bus", "34", "getvcp", "10"],
                reply,
            );
            assert!(
                read_with_runner_in(&runner, &monitor, Duration::from_secs(4), drm).is_err(),
                "{reply:?}"
            );
        }
        // A failed read must not poison the learned max.
        let write = FakeRunner::new().with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--bus",
                "34",
                "setvcp",
                "10",
                "40",
            ],
            "",
        );
        apply_with_runner_in(&write, &monitor, 40, Duration::from_secs(4), drm).expect("write");
    }

    #[test]
    fn renumbered_bus_is_resolved_from_the_connector_on_every_write() {
        let root = tempfile::tempdir().unwrap();
        let dp = connector(
            root.path(),
            "card1-DP-1",
            "connected",
            &edid("Mi Monitor", ""),
        );
        let monitor = ddc_monitor(identified("card1-DP-1", "Mi Monitor", None));
        for bus in ["7", "12"] {
            for entry in std::fs::read_dir(&dp).unwrap().flatten() {
                if entry.file_name().to_string_lossy().starts_with("i2c-") {
                    std::fs::remove_dir(entry.path()).unwrap();
                }
            }
            std::fs::create_dir(dp.join(format!("i2c-{bus}"))).unwrap();
            let runner = FakeRunner::new().with_success(
                "ddcutil",
                &[
                    "--noconfig",
                    "--noverify",
                    "--bus",
                    bus,
                    "setvcp",
                    "10",
                    "30",
                ],
                "",
            );
            let write =
                apply_with_runner_in(&runner, &monitor, 30, Duration::from_secs(4), root.path())
                    .expect("write");
            assert!(write.detail.contains(&format!("bus {bus}")));
        }
    }

    #[test]
    fn identical_serialless_monitors_never_fall_back_to_the_first_match() {
        let root = tempfile::tempdir().unwrap();
        let first = connector(
            root.path(),
            "card1-DP-1",
            "connected",
            &edid("Mi Monitor", ""),
        );
        std::fs::create_dir(first.join("i2c-7")).unwrap();
        let second = connector(
            root.path(),
            "card1-DP-2",
            "connected",
            &edid("Mi Monitor", ""),
        );
        std::fs::create_dir(second.join("i2c-9")).unwrap();
        let monitor = ddc_monitor(identified("card1-DP-2", "Mi Monitor", None));
        let bus_args = [
            "--noconfig",
            "--noverify",
            "--bus",
            "9",
            "setvcp",
            "10",
            "40",
        ];

        // The verified bus targets the right connector.
        let ok = FakeRunner::new().with_success("ddcutil", &bus_args, "");
        let write = apply_with_runner_in(&ok, &monitor, 40, Duration::from_secs(4), root.path())
            .expect("bus write");
        assert!(write.detail.contains("bus 9"));

        // If that bus fails, `--model` would hit either monitor: refuse.
        let failing =
            FakeRunner::new().with_output("ddcutil", &bus_args, Some(1), "", "No monitor");
        let error =
            apply_with_runner_in(&failing, &monitor, 40, Duration::from_secs(4), root.path())
                .expect_err("ambiguous fallback must fail");
        assert!(error.to_string().contains("several connected displays"));
        assert!(failing.calls().iter().all(|call| !call.contains("--model")));

        // Same for reads.
        let read = FakeRunner::new().with_output(
            "ddcutil",
            &["--noconfig", "--terse", "--bus", "9", "getvcp", "10"],
            Some(1),
            "",
            "No monitor",
        );
        let error = read_with_runner_in(&read, &monitor, Duration::from_secs(4), root.path())
            .expect_err("ambiguous read fallback must fail");
        assert!(error.to_string().contains("several connected displays"));
        assert!(read.calls().iter().all(|call| !call.contains("--model")));
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
