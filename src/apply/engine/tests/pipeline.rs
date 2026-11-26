use super::super::*;
#[cfg(target_os = "linux")]
use super::support::SlowRunner;
use super::support::{fast_settings, test_monitor, test_policy};
use crate::apply::types::ApplyStatus;
use crate::backends::{testutil::FakeRunner, BackendKind};
#[cfg(target_os = "linux")]
use crate::config::{MonitorConfig, MonitorSelector};
use std::collections::BTreeMap;
use std::time::Duration;
#[cfg(target_os = "linux")]
use std::time::Instant;

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
        Duration::from_secs(2),
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

    assert_eq!(summary.succeeded, 4, "all 4 monitors should succeed");
    assert_eq!(summary.failed, 0, "no failures expected");
    assert_eq!(runner.calls(), 4, "runner called once per monitor");
    assert!(
        elapsed < Duration::from_millis(1400),
        "concurrent dispatch should complete in <1400ms, took {elapsed:?}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_mixed_hardware_failure() {
    let monitors = vec![
        test_monitor("healthy", BackendKind::Backlight),
        test_monitor("broken_ddc", BackendKind::Ddc),
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

    assert_eq!(summary.succeeded, 1, "healthy backlight must succeed");
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

#[test]
#[cfg(target_os = "linux")]
fn test_arm_resource_constrained_simulation() {
    let monitors = vec![test_monitor("panel", BackendKind::Backlight)];
    let policy = test_policy(vec![("panel", 50)]);

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
            3600 + tick,
            None,
        );

        assert_eq!(
            summary.records.len(),
            1,
            "tick {tick}: summary must have exactly 1 record, not {}",
            summary.records.len()
        );

        if tick == 0 {
            assert_eq!(summary.succeeded, 1, "tick 0: first write must succeed");
        } else {
            assert_eq!(
                summary.skipped, 1,
                "tick {tick}: same percent must be skipped by hysteresis"
            );
        }
    }

    let _ = start.elapsed();
}

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
