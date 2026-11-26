use super::super::probe::ProbeSummary;
use super::super::*;
use crate::apply::types::ApplySettings;
use crate::backends::{BackendError, BackendKind};
use crate::config::{MonitorConfig, MonitorSelector};
use crate::policy::{PerMonitorTarget, PolicyOutput};
#[cfg(target_os = "linux")]
use crate::process::{CommandError, CommandOutput, ProcessRunner};
use crate::state::RuntimeState;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(target_os = "linux")]
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_os = "linux")]
pub(super) const GETVCP_PANEL: [&str; 6] =
    ["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"];

pub(super) const HOLD: Duration = Duration::from_mins(30);

/// A monitor stand-in for probe tests: readback reflects writes, unless the
/// panel is configured to ignore them (still waking).
pub(super) struct FakePanel {
    pub(super) percent: std::cell::Cell<u8>,
    pub(super) ignores_writes: bool,
    /// Added to every written value (the monitor snaps to its own step grid).
    pub(super) snap_offset: u8,
    pub(super) writes: std::cell::Cell<usize>,
    /// Readbacks served first, in order, before the live value.
    pub(super) script: std::cell::RefCell<std::collections::VecDeque<u8>>,
}

#[allow(clippy::unnecessary_wraps)]
impl FakePanel {
    pub(super) fn new(percent: u8) -> Self {
        Self {
            percent: std::cell::Cell::new(percent),
            ignores_writes: false,
            snap_offset: 0,
            writes: std::cell::Cell::new(0),
            script: std::cell::RefCell::new(std::collections::VecDeque::new()),
        }
    }

    pub(super) fn scripted(percent: u8, reads: &[u8]) -> Self {
        let panel = Self::new(percent);
        panel.script.borrow_mut().extend(reads.iter().copied());
        panel
    }

    pub(super) fn read(
        &self,
        _: &MonitorConfig,
    ) -> Result<Option<crate::backends::BackendObservation>, BackendError> {
        let percent = self
            .script
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| self.percent.get());
        Ok(Some(crate::backends::BackendObservation { percent }))
    }

    pub(super) fn write(
        &self,
        _: &MonitorConfig,
        _: &PerMonitorTarget,
        requested: u8,
    ) -> Result<crate::backends::BackendWrite, BackendError> {
        self.writes.set(self.writes.get() + 1);
        self.script.borrow_mut().clear();
        if !self.ignores_writes {
            self.percent.set(requested.saturating_add(self.snap_offset));
        }
        Ok(crate::backends::BackendWrite {
            backend: BackendKind::Ddc,
            applied_percent: requested,
            attempts: 1,
            detail: String::new(),
        })
    }
}

/// Simulates hardware with configurable per-call latency.
/// Sync-safe via `AtomicUsize`; suitable for `thread::scope` dispatch.
#[cfg(target_os = "linux")]
pub(super) struct SlowRunner {
    delay: Duration,
    call_count: Arc<AtomicUsize>,
}

#[cfg(target_os = "linux")]
impl SlowRunner {
    pub(super) fn new(delay: Duration) -> Self {
        Self {
            delay,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "linux")]
impl ProcessRunner for SlowRunner {
    fn run(
        &self,
        _program: &str,
        _args: &[String],
        _timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.delay);
        Ok(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}

pub(super) fn test_monitor(logical_id: &str, backend: BackendKind) -> MonitorConfig {
    MonitorConfig {
        logical_id: logical_id.to_owned(),
        backend,
        enabled: true,
        allow_topology_retargeting: false,
        min_pct: 0,
        max_pct: 100,
        gain: 1.0,
        transition_gamma: 1.0,
        milestone_adjustments: Vec::new(),
        selector: MonitorSelector {
            connector: None,
            // Backlight uses sysfs_path only; serial/model/edid are rejected.
            // DDC uses serial for stable identification.
            serial: match backend {
                BackendKind::Ddc => Some(format!("SN_{logical_id}")),
                BackendKind::Backlight => None,
            },
            model: None,
            edid: None,
            sysfs_path: match backend {
                BackendKind::Backlight => Some(format!("/sys/class/backlight/{logical_id}")),
                BackendKind::Ddc => None,
            },
            ddc_bus: match backend {
                BackendKind::Ddc => Some(1),
                BackendKind::Backlight => None,
            },
            ddc_address: None,
        },
    }
}

pub(super) fn test_policy(targets: Vec<(&str, u8)>) -> PolicyOutput {
    PolicyOutput {
        solar_elevation_deg: 30.0,
        weather_multiplier: 1.0,
        targets: targets
            .into_iter()
            .map(|(id, pct)| PerMonitorTarget {
                logical_id: id.to_owned(),
                percent: pct,
                solar_daylight_factor: 1.0,
                effective_daylight_factor: 1.0,
            })
            .collect(),
    }
}

pub(super) fn fast_settings() -> ApplySettings {
    ApplySettings {
        // 1% delta threshold: same-percent requests on subsequent ticks
        // will be skipped by hysteresis (delta=0 < threshold=1).
        min_write_delta_pct: 1,
        max_step_pct_per_tick: 100,
        smooth_transition: false,
        min_apply_interval: Duration::ZERO,
        dry_run: false,
        // Large reassert interval (1 billion seconds ≈ 31 years) so it
        // never fires during the 10k-tick simulation test.
        apply_reassert_interval: Duration::from_secs(1_000_000_000),
        external_change_hold: Duration::from_mins(30),
        ddc_timeout: Duration::from_secs(10),
        backlight_timeout: Duration::from_secs(2),
    }
}

pub(super) fn panel_monitor() -> MonitorConfig {
    MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    }
}

/// Runs one probe over a single `panel` monitor against `fake`.
pub(super) fn probe_panel(
    fake: &FakePanel,
    state: &mut RuntimeState,
    holds: &mut BTreeMap<String, ExternalHold>,
    down: &mut BTreeSet<String>,
    requested: u8,
) -> ProbeSummary {
    probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors: std::slice::from_ref(&panel_monitor()),
            policy: &test_policy(vec![("panel", requested)]),
            now_epoch_s: 1_000,
            dry_run: false,
            external_change_hold: Duration::from_mins(30),
            recovery_until_epoch_s: None,
        },
        ProbeSinks {
            state,
            external_holds: Some(holds),
            fade_engine: None,
            unreachable_monitors: Some(down),
        },
        |m| fake.read(m),
        |m, t, percent| fake.write(m, t, percent),
    )
}

pub(super) fn read_error() -> BackendError {
    BackendError::Io {
        backend: BackendKind::Ddc,
        program: String::from("ddcutil"),
        message: String::from("No monitor detected"),
        attempts: 1,
    }
}

/// One probe of `panel` at `now`; `fake == None` makes the read fail.
pub(super) fn probe_at(
    fake: Option<&FakePanel>,
    state: &mut RuntimeState,
    holds: &mut BTreeMap<String, ExternalHold>,
    down: &mut BTreeSet<String>,
    requested: u8,
    now: u64,
    recovery_until: Option<u64>,
    hold: Duration,
) -> ProbeSummary {
    let panel = FakePanel::new(0);
    probe_and_correct_with_external_state(
        &ProbeInputs {
            monitors: std::slice::from_ref(&panel_monitor()),
            policy: &test_policy(vec![("panel", requested)]),
            now_epoch_s: now,
            dry_run: false,
            external_change_hold: hold,
            recovery_until_epoch_s: recovery_until,
        },
        ProbeSinks {
            state,
            external_holds: Some(holds),
            fade_engine: None,
            unreachable_monitors: Some(down),
        },
        |m| fake.map_or_else(|| Err(read_error()), |f| f.read(m)),
        |m, t, percent| fake.map_or_else(|| panel.write(m, t, percent), |f| f.write(m, t, percent)),
    )
}

pub(super) fn held(percent: u8) -> BTreeMap<String, ExternalHold> {
    BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 2_700,
            observed_percent: percent,
        },
    )])
}
