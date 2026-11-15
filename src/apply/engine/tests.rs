use super::probe::ProbeSummary;
use super::*;
use crate::apply::types::ApplyStatus;
use crate::backends::BackendError;
use crate::backends::{testutil::FakeRunner, BackendKind};
use crate::config::MonitorSelector;
use crate::policy::PerMonitorTarget;
#[cfg(target_os = "linux")]
use crate::process::{CommandError, CommandOutput, ProcessRunner};
use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(target_os = "linux")]
use std::sync::Arc;
use std::time::Duration;
#[cfg(target_os = "linux")]
use std::time::Instant;

// -------------------------------------------------------------------------
// SlowRunner — simulates hardware with configurable per-call latency.
// Sync-safe via AtomicUsize; suitable for thread::scope dispatch.
// -------------------------------------------------------------------------
#[cfg(target_os = "linux")]
struct SlowRunner {
    delay: Duration,
    call_count: Arc<AtomicUsize>,
}

#[cfg(target_os = "linux")]
impl SlowRunner {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn calls(&self) -> usize {
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

// -------------------------------------------------------------------------
// Test helpers
// -------------------------------------------------------------------------
fn test_monitor(logical_id: &str, backend: BackendKind) -> MonitorConfig {
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

fn test_policy(targets: Vec<(&str, u8)>) -> PolicyOutput {
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

#[test]
fn reconciled_apply_skips_unavailable_topology_before_dispatch() {
    let monitors = vec![test_monitor("panel", BackendKind::Backlight)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();

    let summary = apply_policy_with_runner_reconciled(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        &runner,
        1000,
        &CapabilitySnapshot::default(),
        None,
        None,
    );

    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.attempted, 0);
    assert_eq!(summary.records[0].status, ApplyStatus::SkippedTopology);
    assert!(runner.calls().is_empty());
}

/// Builds a backlight monitor whose selector is ONLY a connector (legacy
/// connector-only form). This exercises the Phase 8 owner-approved resolution:
/// the daemon must fail closed when no authoritative topology link exists.
#[cfg(target_os = "linux")]
fn legacy_connector_monitor(logical_id: &str, connector: &str) -> MonitorConfig {
    MonitorConfig {
        logical_id: logical_id.to_owned(),
        backend: BackendKind::Backlight,
        enabled: true,
        allow_topology_retargeting: false,
        min_pct: 0,
        max_pct: 100,
        gain: 1.0,
        transition_gamma: 1.0,
        milestone_adjustments: Vec::new(),
        selector: MonitorSelector {
            connector: Some(connector.to_owned()),
            serial: None,
            model: None,
            edid: None,
            sysfs_path: None,
            ddc_bus: None,
            ddc_address: None,
        },
    }
}

// =========================================================================
// TEST — legacy connector-only backlight resolution surfaces the
// deprecation diagnostic through the apply backend. Uses injectable roots,
// so it is deterministic on every host (unlike a real /sys probe).
// =========================================================================
#[test]
#[cfg(target_os = "linux")]
fn legacy_connector_only_resolves_through_backend_and_surfaces_diagnostic() {
    use std::fs;

    let tmpdir = tempfile::tempdir().expect("tempdir");
    let drm_root = tmpdir.path().join("drm");
    let conn = drm_root.join("card1-DP-1");
    fs::create_dir_all(&conn).expect("mkdir connector dir");
    let backlight_root = tmpdir.path().join("backlight");
    let hw_device = tmpdir.path().join("hwdd");
    fs::create_dir_all(&hw_device).expect("mkdir hw device");
    let device = backlight_root.join("ddcci_backlight_0");
    fs::create_dir_all(&device).expect("mkdir device");
    fs::write(device.join("max_brightness"), "100").expect("max");
    fs::write(device.join("type"), "raw\n").expect("type");
    std::os::unix::fs::symlink(&hw_device, conn.join("ddcci_backlight")).expect("symlink");
    std::os::unix::fs::symlink(&hw_device, device.join("device")).expect("device symlink");

    let monitor = legacy_connector_monitor("legacy", "card1-DP-1");
    let runner = FakeRunner::new().with_success(
        "brightnessctl",
        &[
            "--quiet",
            "--class",
            "backlight",
            "--device",
            "ddcci_backlight_0",
            "set",
            "50%",
        ],
        "",
    );

    let result = crate::backends::backlight::apply_with_runner_roots_for_test(
        &runner,
        &monitor,
        50,
        std::time::Duration::from_secs(2),
        &drm_root,
        &backlight_root,
    )
    .expect("exactly one proven mapping must resolve and succeed");

    assert_eq!(result.applied_percent, 50);
    assert!(
        result.detail.contains("deprecat"),
        "BackendWrite.detail must surface the deprecation diagnostic, got: {}",
        result.detail
    );
    assert_eq!(
        runner.calls().len(),
        1,
        "brightnessctl must be invoked once"
    );
}

#[test]
fn wake_probe_rewrites_only_monitors_that_answer_with_the_wrong_value() {
    use crate::backends::{BackendObservation, BackendWrite};
    use std::cell::RefCell;

    let monitor = |id: &str| MonitorConfig {
        logical_id: String::from(id),
        ..MonitorConfig::default()
    };
    let monitors = vec![monitor("right"), monitor("wrong"), monitor("asleep")];
    let target = |id: &str, percent| PerMonitorTarget {
        logical_id: String::from(id),
        percent,
        solar_daylight_factor: 0.0,
        effective_daylight_factor: 0.0,
    };
    let policy = PolicyOutput {
        solar_elevation_deg: 10.0,
        weather_multiplier: 1.0,
        targets: vec![
            target("right", 40),
            target("wrong", 40),
            target("asleep", 40),
        ],
    };
    let mut state = RuntimeState::default();
    let writes = RefCell::new(Vec::new());
    let summary = probe_and_correct_with(
        &monitors,
        &policy,
        &mut state,
        1_000,
        false,
        |monitor| match monitor.logical_id.as_str() {
            "right" => Ok(Some(BackendObservation { percent: 40 })),
            "wrong" => Ok(Some(BackendObservation { percent: 3 })),
            _ => Err(BackendError::Io {
                backend: crate::backends::BackendKind::Ddc,
                program: String::from("ddcutil"),
                message: String::from("No monitor detected"),
                attempts: 1,
            }),
        },
        |monitor, _, percent| {
            writes
                .borrow_mut()
                .push((monitor.logical_id.clone(), percent));
            Ok(BackendWrite {
                backend: crate::backends::BackendKind::Ddc,
                applied_percent: percent,
                attempts: 1,
                detail: String::new(),
            })
        },
    );
    assert_eq!(writes.into_inner(), vec![(String::from("wrong"), 40)]);
    assert_eq!(
        summary,
        ProbeSummary {
            answered: 2,
            corrected: 1,
            failed: 0,
            unreachable: 1,
        }
    );
    assert_eq!(
        state.monitor("wrong").and_then(|m| m.last_applied_percent),
        Some(40)
    );
    // The monitor that did not answer gets no backoff.
    assert!(state.monitor("asleep").is_none_or(|m| m.backoff.is_none()));

    // Dry run reports drift without writing.
    let mut dry_state = RuntimeState::default();
    let dry = probe_and_correct_with(
        &monitors,
        &policy,
        &mut dry_state,
        1_000,
        true,
        |_| Ok(Some(BackendObservation { percent: 3 })),
        |_, _, _| panic!("dry run must not write"),
    );
    assert_eq!(dry.corrected, 0);
    assert_eq!(dry.answered, 3);
}

#[test]
#[cfg(target_os = "linux")]
fn due_reassert_reads_matching_hardware_without_writing() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new().with_success(
        "ddcutil",
        &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
        "VCP 10 C 50 100\n",
    );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);

    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    let summary = apply_policy_with_runner_monitors(
        &monitors, settings, &policy, &mut state, None, &runner, 1_120, None,
    );

    assert_eq!(summary.succeeded, 0);
    assert_eq!(summary.skipped, 1);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    assert_eq!(
        state
            .monitor("panel")
            .and_then(|m| m.last_integrity_check_at_epoch_s),
        Some(1_120)
    );
}

#[test]
#[cfg(target_os = "linux")]
fn external_change_outside_recovery_window_sets_hold_without_writing() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new().with_success(
        "ddcutil",
        &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
        "VCP 10 C 35 100\n",
    );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::new();

    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    settings.external_change_hold = Duration::from_mins(30);
    let summary = apply_policy_with_runner_monitors_external(
        &monitors, settings, &policy, &mut state, None, &runner, 1_120, None, &mut holds, None,
    );

    assert_eq!(summary.succeeded, 0);
    assert_eq!(summary.records[0].status, ApplyStatus::SkippedHysteresis);
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
    assert_eq!(holds["panel"].observed_percent, 35);
}

#[test]
#[cfg(target_os = "linux")]
fn external_change_within_recovery_window_is_corrected() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new()
        .with_success(
            "ddcutil",
            &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
            "VCP 10 C 35 100\n",
        )
        .with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_panel",
                "setvcp",
                "10",
                "50",
            ],
            "",
        );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::new();
    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    settings.external_change_hold = Duration::from_mins(30);

    let summary = apply_policy_with_runner_monitors_external(
        &monitors,
        settings,
        &policy,
        &mut state,
        None,
        &runner,
        1_120,
        None,
        &mut holds,
        Some(1_200),
    );

    assert_eq!(summary.succeeded, 1);
    assert!(holds.is_empty());
    assert!(runner.calls().iter().any(|call| call.contains("|setvcp|")));
}

#[test]
fn external_hold_expiry_starts_fade_from_current_readback() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 80)]);
    let runner = FakeRunner::new().with_success(
        "ddcutil",
        &["--noconfig", "--terse", "--sn", "SN_panel", "getvcp", "10"],
        "VCP 10 C 35 100\n",
    );
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 1_100,
            observed_percent: 35,
        },
    )]);
    let mut fade_engine = crate::runtime::fade::FadeEngine::new();
    let mut settings = fast_settings();
    settings.external_change_hold = Duration::from_mins(30);

    let summary = apply_policy_with_runner_monitors_external(
        &monitors,
        settings,
        &policy,
        &mut state,
        Some(&mut fade_engine),
        &runner,
        1_120,
        None,
        &mut holds,
        None,
    );

    assert_eq!(summary.succeeded, 1);
    assert!(holds.is_empty());
    assert_eq!(fade_engine.active_fades["panel"].start_percent, 35);
    assert_eq!(fade_engine.active_fades["panel"].target_percent, 80);
    assert!(runner.calls().iter().any(|call| call.contains("getvcp")));
    assert!(runner.calls().iter().all(|call| !call.contains("|setvcp|")));
}

#[test]
fn external_hold_readback_change_extends_until_and_updates_observation() {
    use crate::backends::{BackendObservation, BackendWrite};

    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let mut holds = BTreeMap::from([(
        String::from("panel"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 1_800,
            observed_percent: 35,
        },
    )]);
    let summary = probe_and_correct_with_external_state(
        std::slice::from_ref(&monitor),
        &policy,
        &mut state,
        1_000,
        false,
        Duration::from_mins(30),
        |_| Ok(Some(BackendObservation { percent: 20 })),
        |_, _, _| {
            Ok(BackendWrite {
                backend: BackendKind::Ddc,
                applied_percent: 50,
                attempts: 1,
                detail: String::new(),
            })
        },
        Some(&mut holds),
        None,
        None,
    );

    assert_eq!(summary.corrected, 0);
    assert_eq!(holds["panel"].observed_percent, 20);
    assert_eq!(holds["panel"].until_epoch_s, 2_800);
}

#[test]
fn hold_minutes_zero_keeps_legacy_correction() {
    use crate::backends::{BackendObservation, BackendWrite};

    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 900);
    let mut holds = BTreeMap::new();
    let writes = std::cell::Cell::new(0);
    let summary = probe_and_correct_with_external_state(
        std::slice::from_ref(&monitor),
        &policy,
        &mut state,
        1_000,
        false,
        Duration::ZERO,
        |_| Ok(Some(BackendObservation { percent: 20 })),
        |_, _, percent| {
            writes.set(writes.get() + 1);
            Ok(BackendWrite {
                backend: BackendKind::Ddc,
                applied_percent: percent,
                attempts: 1,
                detail: String::new(),
            })
        },
        Some(&mut holds),
        None,
        None,
    );

    assert_eq!(summary.corrected, 1);
    assert_eq!(writes.get(), 1);
    assert!(holds.is_empty());
}

#[test]
fn own_recent_write_with_lagging_readback_is_not_external() {
    use crate::backends::{BackendObservation, BackendWrite};

    let monitor = MonitorConfig {
        logical_id: String::from("panel"),
        ..MonitorConfig::default()
    };
    let policy = test_policy(vec![("panel", 50)]);
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 998);
    let mut holds = BTreeMap::new();
    let summary = probe_and_correct_with_external_state(
        std::slice::from_ref(&monitor),
        &policy,
        &mut state,
        1_000,
        false,
        Duration::from_mins(30),
        |_| Ok(Some(BackendObservation { percent: 20 })),
        |_, _, percent| {
            Ok(BackendWrite {
                backend: BackendKind::Ddc,
                applied_percent: percent,
                attempts: 1,
                detail: String::new(),
            })
        },
        Some(&mut holds),
        None,
        None,
    );

    assert_eq!(summary.corrected, 1);
    assert!(holds.is_empty());
}

#[test]
fn before_reassert_deadline_hysteresis_still_avoids_read_and_write() {
    let monitors = vec![test_monitor("panel", BackendKind::Ddc)];
    let policy = test_policy(vec![("panel", 50)]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();
    state.record_apply_success("panel", 50, 1_000);

    let mut settings = fast_settings();
    settings.apply_reassert_interval = Duration::from_mins(2);
    let summary = apply_policy_with_runner_monitors(
        &monitors, settings, &policy, &mut state, None, &runner, 1_119, None,
    );

    assert_eq!(summary.records[0].status, ApplyStatus::SkippedHysteresis);
    assert!(runner.calls().is_empty());
}

fn fast_settings() -> ApplySettings {
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

#[test]
#[cfg(target_os = "linux")]
fn disabled_smooth_transition_sends_one_direct_ddc_write() {
    let monitors = vec![test_monitor("external", BackendKind::Ddc)];
    let policy = test_policy(vec![("external", 80)]);
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --noverify")
        .with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_external",
                "setvcp",
                "10",
                "80",
            ],
            "",
        );
    let mut state = RuntimeState::default();
    state.record_apply_success("external", 20, 1);
    let mut fade_engine = crate::runtime::fade::FadeEngine::new();
    assert!(fade_engine.maybe_enqueue("external", 20, 80));

    let settings = ApplySettings {
        max_step_pct_per_tick: 6,
        ..fast_settings()
    };
    let summary = apply_policy_with_runner_monitors(
        &monitors,
        settings,
        &policy,
        &mut state,
        Some(&mut fade_engine),
        &runner,
        100,
        None,
    );

    assert_eq!(summary.succeeded, 1);
    assert_eq!(summary.records[0].applied_percent, 80);
    assert!(!fade_engine.is_fading());
    let writes = runner
        .calls()
        .into_iter()
        .filter(|call| call.contains("|setvcp|10|"))
        .collect::<Vec<_>>();
    assert_eq!(
        writes,
        vec![String::from(
            "ddcutil|--noconfig|--noverify|--sn|SN_external|setvcp|10|80"
        )]
    );
}

#[test]
fn enabled_smooth_transition_queues_the_existing_fade() {
    let monitors = vec![test_monitor("external", BackendKind::Ddc)];
    let policy = test_policy(vec![("external", 80)]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();
    state.record_apply_success("external", 20, 1);
    let mut fade_engine = crate::runtime::fade::FadeEngine::new();
    let settings = ApplySettings {
        smooth_transition: true,
        ..fast_settings()
    };

    let summary = apply_policy_with_runner_monitors(
        &monitors,
        settings,
        &policy,
        &mut state,
        Some(&mut fade_engine),
        &runner,
        100,
        None,
    );

    assert_eq!(summary.succeeded, 1);
    assert!(fade_engine.is_fading());
    assert!(runner.calls().is_empty());
}

// =========================================================================
// TEST 1 — Four monitors with 500ms simulated latency each.
//
// Sequential execution would take 4 × 500ms = 2000ms.
// Concurrent dispatch must complete in roughly max(latency) + overhead.
// We assert completion under 1400ms to leave generous headroom for CI
// environments and ARM build servers while still proving concurrency.
// =========================================================================
#[test]
#[cfg(target_os = "linux")]
fn test_four_monitor_extreme_latency() {
    // Use Backlight backend: no i2c flock, so threads don't serialize.
    // DDC has a per-call flock(LockExclusive) by design; testing concurrency
    // with DDC would require a lock-free mock path.
    let monitors = vec![
        test_monitor("mon1", BackendKind::Backlight),
        test_monitor("mon2", BackendKind::Backlight),
        test_monitor("mon3", BackendKind::Backlight),
        test_monitor("mon4", BackendKind::Backlight),
    ];
    let policy = test_policy(vec![("mon1", 50), ("mon2", 60), ("mon3", 70), ("mon4", 80)]);
    let runner = SlowRunner::new(Duration::from_millis(500));
    let mut state = RuntimeState::default();

    let start = Instant::now();
    let summary = apply_policy_with_runner_monitors(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        None,
        &runner,
        1000,
        None,
    );
    let elapsed = start.elapsed();

    // All 4 monitors must succeed.
    assert_eq!(summary.succeeded, 4, "all 4 monitors should succeed");
    assert_eq!(summary.failed, 0, "no failures expected");
    assert_eq!(runner.calls(), 4, "runner called once per monitor");

    // Concurrency proof: 4 × 500ms sequential = 2000ms.
    // Concurrent ceiling: 500ms + generous overhead = 1400ms.
    assert!(
        elapsed < Duration::from_millis(1400),
        "concurrent dispatch should complete in <1400ms, took {elapsed:?}"
    );
}

// =========================================================================
// TEST 2 — Mixed hardware: healthy backlight, hanging DDC, missing monitor.
//
// Proves that one monitor's failure does not block or corrupt others.
// =========================================================================
#[test]
#[cfg(target_os = "linux")]
fn test_mixed_hardware_failure() {
    let monitors = vec![
        test_monitor("healthy", BackendKind::Backlight),
        test_monitor("broken_ddc", BackendKind::Ddc),
        // "ghost" has no MonitorConfig — exercises missing-config path
    ];
    let policy = test_policy(vec![("healthy", 75), ("broken_ddc", 80), ("ghost", 90)]);

    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --noverify")
        .with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "healthy",
                "set",
                "75%",
            ],
            "",
        )
        // DDC retry: two timeouts (first attempt + one retry)
        .with_timeout(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_broken_ddc",
                "setvcp",
                "10",
                "80",
            ],
            Duration::from_secs(5),
            "i2c bus timeout",
        )
        .with_timeout(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_broken_ddc",
                "setvcp",
                "10",
                "80",
            ],
            Duration::from_secs(5),
            "i2c bus timeout retry",
        );

    let mut state = RuntimeState::default();
    let summary = apply_policy_with_runner_monitors(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        None,
        &runner,
        2000,
        None,
    );

    // Healthy backlight succeeds.
    assert_eq!(summary.succeeded, 1, "healthy backlight must succeed");
    // Both broken_ddc and ghost fail.
    assert_eq!(summary.failed, 2, "broken_ddc and ghost must fail");
    assert_eq!(summary.records.len(), 3, "3 records total");

    let healthy = summary
        .records
        .iter()
        .find(|r| r.logical_id == "healthy")
        .unwrap();
    assert_eq!(healthy.status, ApplyStatus::Succeeded);
    assert_eq!(healthy.applied_percent, 75);

    let broken = summary
        .records
        .iter()
        .find(|r| r.logical_id == "broken_ddc")
        .unwrap();
    assert_eq!(broken.status, ApplyStatus::Failed);
    // DDC retries transient failures once, so attempts >= 2.
    assert!(
        broken.attempts >= 2,
        "DDC backend retries transient timeouts"
    );

    let ghost = summary
        .records
        .iter()
        .find(|r| r.logical_id == "ghost")
        .unwrap();
    assert_eq!(ghost.status, ApplyStatus::Failed);
    assert!(ghost.detail.contains("no matching monitor"));
}

// =========================================================================
// TEST 3 — ARM resource-constrained simulation: 10,000 ticks.
//
// Proves no unbounded allocation accumulates across ticks.
// Each tick must produce exactly N records (one per target), and all
// previous-tick data is dropped. This test would OOM or timeout on a
// Raspberry Pi if records were accumulated across ticks.
// =========================================================================
#[test]
#[cfg(target_os = "linux")]
fn test_arm_resource_constrained_simulation() {
    let monitors = vec![test_monitor("panel", BackendKind::Backlight)];
    let policy = test_policy(vec![("panel", 50)]);

    // Register exactly one runner response (for the first tick write only).
    // All subsequent ticks will be skipped by hysteresis (same percent, no delta).
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --noverify")
        .with_success(
            "brightnessctl",
            &[
                "--quiet",
                "--class",
                "backlight",
                "--device",
                "panel",
                "set",
                "50%",
            ],
            "",
        );
    let mut state = RuntimeState::default();

    let start = Instant::now();
    for tick in 0u64..10_000 {
        let summary = apply_policy_with_runner_monitors(
            &monitors,
            fast_settings(),
            &policy,
            &mut state,
            None,
            &runner,
            // Advance epoch each tick to avoid minimum-interval skip.
            // Hysteresis (same percent) takes over after first write.
            3600 + tick,
            None,
        );

        // KEY INVARIANT: each tick produces exactly 1 record.
        // If records were accumulated across ticks, this would grow.
        assert_eq!(
            summary.records.len(),
            1,
            "tick {tick}: summary must have exactly 1 record, not {}",
            summary.records.len()
        );

        if tick == 0 {
            assert_eq!(summary.succeeded, 1, "tick 0: first write must succeed");
        } else {
            // Same percent requested → hysteresis skip
            assert_eq!(
                summary.skipped, 1,
                "tick {tick}: same percent must be skipped by hysteresis"
            );
        }
    }

    // On a constrained device, 10k ticks of pure logic should be fast.
    // No hard assertion — but logs reveal if something is catastrophically slow.
    let _ = start.elapsed(); // could assert < 5s if needed
}

// =========================================================================
// Characterization tests: pin the exact per-target record shape, policy
// ordering and summary counters of the apply pipeline.
// =========================================================================

fn record_lines(summary: &ApplySummary) -> Vec<String> {
    summary
        .records
        .iter()
        .map(|r| {
            format!(
                "{}|{:?}|{:?}|{}|{}|{}|{:?}|{:?}|{:?}|{}",
                r.logical_id,
                r.backend,
                r.status,
                r.requested_percent,
                r.applied_percent,
                r.attempts,
                r.failure_kind,
                r.consecutive_failures,
                r.backoff_until_epoch_s,
                r.detail
            )
        })
        .collect()
}

fn summary_counters(summary: &ApplySummary) -> [usize; 7] {
    [
        summary.attempted,
        summary.skipped,
        summary.succeeded,
        summary.failed,
        summary.backoff_skips,
        summary.transient_failures,
        summary.persistent_failures,
    ]
}

#[test]
fn characterization_gate_matrix_keeps_policy_order_and_records() {
    let mut disabled = test_monitor("off", BackendKind::Ddc);
    disabled.enabled = false;
    let monitors = vec![
        disabled,
        test_monitor("steady", BackendKind::Ddc),
        test_monitor("recent", BackendKind::Ddc),
        test_monitor("backed", BackendKind::Ddc),
        test_monitor("held", BackendKind::Ddc),
        test_monitor("fade", BackendKind::Ddc),
    ];
    let policy = test_policy(vec![
        ("fade", 90),
        ("ghost", 140),
        ("off", 40),
        ("steady", 50),
        ("recent", 80),
        ("backed", 70),
        ("held", 60),
    ]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();
    state.record_apply_success("fade", 20, 100);
    state.record_apply_success("steady", 50, 900);
    state.record_apply_success("recent", 10, 995);
    state.record_apply_success("backed", 20, 100);
    state.record_apply_failure(
        "backed",
        BackendKind::Ddc,
        crate::backends::FailureKind::Transient,
        998,
    );
    let mut holds = BTreeMap::from([(
        String::from("held"),
        ExternalHold {
            since_epoch_s: 900,
            until_epoch_s: 2_000,
            observed_percent: 33,
        },
    )]);
    let mut fade_engine = crate::runtime::fade::FadeEngine::new();
    assert!(fade_engine.maybe_enqueue("held", 10, 60));
    let settings = ApplySettings {
        smooth_transition: true,
        min_apply_interval: Duration::from_mins(1),
        ..fast_settings()
    };

    let summary = apply_policy_with_runner_monitors_external(
        &monitors,
        settings,
        &policy,
        &mut state,
        Some(&mut fade_engine),
        &runner,
        1_000,
        None,
        &mut holds,
        None,
    );

    assert_eq!(
        record_lines(&summary),
        vec![
            "fade|Some(Ddc)|Succeeded|90|90|0|None|None|None|fading from 20 to 90",
            "ghost|None|Failed|100|100|0|None|None|None|no matching monitor configuration for policy target",
            "off|Some(Ddc)|SkippedDisabled|40|40|0|None|None|None|monitor is disabled",
            "steady|Some(Ddc)|SkippedHysteresis|50|50|0|None|None|None|delta 0 is below hysteresis threshold 1",
            "recent|Some(Ddc)|SkippedMinimumInterval|80|80|0|None|None|None|last apply was 5s ago; minimum apply interval is 60s",
            "backed|Some(Ddc)|SkippedBackoff|70|70|0|Some(Transient)|Some(1)|Some(1003)|Transient failure backoff active for 3s",
            "held|Some(Ddc)|SkippedHysteresis|60|33|0|None|None|Some(2000)|external brightness hold active for 1000s",
        ]
    );
    assert_eq!(summary_counters(&summary), [1, 5, 1, 1, 1, 0, 0]);
    assert!(runner.calls().is_empty());
    assert!(!fade_engine.active_fades.contains_key("held"));
    assert!(fade_engine.active_fades.contains_key("fade"));
    assert_eq!(holds["held"].until_epoch_s, 2_000);
}

#[test]
fn characterization_dry_run_records_skip_after_interval_gates() {
    let monitors = vec![
        test_monitor("a", BackendKind::Ddc),
        test_monitor("b", BackendKind::Backlight),
    ];
    let policy = test_policy(vec![("b", 30), ("a", 75)]);
    let runner = FakeRunner::new();
    let mut state = RuntimeState::default();
    state.record_apply_success("a", 40, 10);
    let settings = ApplySettings {
        dry_run: true,
        max_step_pct_per_tick: 10,
        smooth_transition: true,
        ..fast_settings()
    };

    let summary = apply_policy_with_runner_monitors(
        &monitors, settings, &policy, &mut state, None, &runner, 1_000, None,
    );

    assert_eq!(
        record_lines(&summary),
        vec![
            "b|Some(Backlight)|SkippedDryRun|30|30|0|None|None|None|dry_run is enabled",
            "a|Some(Ddc)|SkippedDryRun|75|50|0|None|None|None|dry_run is enabled",
        ]
    );
    assert_eq!(summary_counters(&summary), [0, 2, 0, 0, 0, 0, 0]);
    assert!(runner.calls().is_empty());
}

#[test]
#[cfg(target_os = "linux")]
fn characterization_override_bypasses_backoff_and_failure_records_backoff() {
    let monitors = vec![
        test_monitor("backed", BackendKind::Ddc),
        test_monitor("broken", BackendKind::Ddc),
    ];
    let policy = test_policy(vec![("broken", 55), ("backed", 80)]);
    let runner = FakeRunner::new()
        .with_success("ddcutil", &["--help"], "--noconfig --noverify")
        .with_success(
            "ddcutil",
            &[
                "--noconfig",
                "--noverify",
                "--sn",
                "SN_backed",
                "setvcp",
                "10",
                "26",
            ],
            "",
        );
    let mut state = RuntimeState::default();
    state.record_apply_success("backed", 20, 100);
    state.record_apply_failure(
        "backed",
        BackendKind::Ddc,
        crate::backends::FailureKind::Transient,
        990,
    );
    let override_settings = ApplySettings {
        smooth_transition: true,
        max_step_pct_per_tick: 6,
        ..fast_settings()
    };

    let summary = apply_policy_with_runner_monitors(
        &monitors,
        fast_settings(),
        &policy,
        &mut state,
        None,
        &runner,
        1_000,
        Some(override_settings),
    );

    assert_eq!(
        record_lines(&summary),
        vec![
            "broken|Some(Ddc)|Failed|55|55|1|Some(Persistent)|Some(1)|Some(1005)|ddcutil: unexpected command: ddcutil|--noconfig|--noverify|--sn|SN_broken|setvcp|10|55; backing off for 5s after 1 consecutive Persistent failure(s)",
            "backed|Some(Ddc)|Succeeded|80|26|1|None|None|None|applied via ddcutil using serial `SN_backed`",
        ]
    );
    assert_eq!(summary_counters(&summary), [2, 0, 1, 1, 0, 0, 1]);
    assert_eq!(
        state.monitor("backed").and_then(|m| m.last_applied_percent),
        Some(26)
    );
    assert_eq!(
        state
            .monitor("broken")
            .and_then(|m| m.backoff.as_ref())
            .map(|b| b.consecutive_failures),
        Some(1)
    );
}
