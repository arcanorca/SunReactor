use super::dispatch::apply_monitor_target;
use super::types::{ApplyRecord, ApplySettings, ApplyStatus, ApplySummary};
use crate::backends::RealProcessRunner;
use crate::backends::{BackendError, BackendWrite, ProcessRunner};
use crate::config::{Config, MonitorConfig};
use crate::policy::{PerMonitorTarget, PolicyOutput};
use crate::state::{self, RuntimeState};
use std::collections::BTreeMap;

pub fn apply_policy(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    ddcutil_profile: Option<&crate::ddcutil::DdcutilProfile>,
) -> ApplySummary {
    apply_policy_at(config, policy, state, ddcutil_profile, state::current_epoch_s())
}

pub fn apply_policy_at(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    ddcutil_profile: Option<&crate::ddcutil::DdcutilProfile>,
    now_epoch_s: u64,
) -> ApplySummary {
    apply_policy_with_runner(config, policy, state, ddcutil_profile, &RealProcessRunner, now_epoch_s)
}

pub(crate) fn apply_policy_with_runner<R: ProcessRunner + Sync>(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    ddcutil_profile: Option<&crate::ddcutil::DdcutilProfile>,
    runner: &R,
    now_epoch_s: u64,
) -> ApplySummary {
    apply_policy_with_runner_settings(config, policy, state, ddcutil_profile, None, runner, now_epoch_s, None)
}

/// Apply policy with an optional settings override. When `settings_override`
/// is `Some`, the provided settings are used instead of deriving them from
/// config. This lets IPC `RunOnce` bypass throttles (hysteresis, step limit,
/// minimum interval, and failure backoff) so manual brightness overrides apply
/// immediately.
pub(crate) fn apply_policy_with_runner_settings<R: ProcessRunner + Sync>(
    config: &Config,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    ddcutil_profile: Option<&crate::ddcutil::DdcutilProfile>,
    fade_engine: Option<&mut crate::runtime::fade::FadeEngine>,
    runner: &R,
    now_epoch_s: u64,
    settings_override: Option<ApplySettings>,
) -> ApplySummary {
    apply_policy_with_runner_monitors(
        &config.monitors,
        ApplySettings::from_config(config),
        policy,
        state,
        ddcutil_profile,
        fade_engine,
        runner,
        now_epoch_s,
        settings_override,
    )
}

pub(crate) fn apply_policy_with_runner_monitors<R: ProcessRunner + Sync>(
    monitors: &[MonitorConfig],
    default_settings: ApplySettings,
    policy: &PolicyOutput,
    state: &mut RuntimeState,
    ddcutil_profile: Option<&crate::ddcutil::DdcutilProfile>,
    mut fade_engine: Option<&mut crate::runtime::fade::FadeEngine>,
    runner: &R,
    now_epoch_s: u64,
    settings_override: Option<ApplySettings>,
) -> ApplySummary {
    let bypass_backoff = settings_override.is_some();
    let settings = settings_override.unwrap_or(default_settings);
    let monitor_index = monitors
        .iter()
        .map(|monitor| (monitor.logical_id.as_str(), monitor))
        .collect::<BTreeMap<_, _>>();

    // -------------------------------------------------------------------------
    // Phase 1 — DECIDE (sequential, read-only on state)
    //
    // For each policy target we determine whether to skip or dispatch.
    // All work items are collected into a Vec so we can fan them out
    // concurrently in Phase 2 without holding a &mut borrow on state.
    // -------------------------------------------------------------------------
    enum WorkItem<'cfg> {
        /// Pre-built record — no hardware call needed.
        Skip(ApplyRecord),
        /// Needs a hardware dispatch call.
        Dispatch {
            monitor: &'cfg MonitorConfig,
            target: &'cfg PerMonitorTarget,
            applied_percent: u8,
            requested_percent: u8,
        },
    }

    let mut work: Vec<WorkItem<'_>> = Vec::with_capacity(policy.targets.len());

    for target in &policy.targets {
        let requested_percent = target.percent.min(100);
        let Some(monitor) = monitor_index.get(target.logical_id.as_str()).copied() else {
            work.push(WorkItem::Skip(ApplyRecord {
                logical_id: target.logical_id.clone(),
                backend: None,
                requested_percent,
                applied_percent: requested_percent,
                attempts: 0,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::Failed,
                detail: String::from("no matching monitor configuration for policy target"),
            }));
            continue;
        };

        if !monitor.enabled {
            work.push(WorkItem::Skip(ApplyRecord {
                logical_id: monitor.logical_id.clone(),
                backend: Some(monitor.backend),
                requested_percent,
                applied_percent: requested_percent,
                attempts: 0,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::SkippedDisabled,
                detail: String::from("monitor is disabled"),
            }));
            continue;
        }

        let monitor_state = state
            .monitor(&monitor.logical_id)
            .cloned()
            .unwrap_or_default();

        let skip_hysteresis = state::should_skip_hysteresis(
            monitor_state.last_applied_percent,
            requested_percent,
            settings.min_write_delta_pct,
        );
        if skip_hysteresis
            && !reassert_due(
                monitor_state.last_applied_at_epoch_s,
                now_epoch_s,
                settings.apply_reassert_interval.as_secs(),
            )
        {
            let delta = monitor_state
                .last_applied_percent
                .map_or(0, |last| requested_percent.abs_diff(last));
            work.push(WorkItem::Skip(ApplyRecord {
                logical_id: monitor.logical_id.clone(),
                backend: Some(monitor.backend),
                requested_percent,
                applied_percent: requested_percent,
                attempts: 0,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::SkippedHysteresis,
                detail: format!(
                    "delta {delta} is below hysteresis threshold {}",
                    settings.min_write_delta_pct
                ),
            }));
            continue;
        }

        let applied_percent = state::limit_step_size(
            monitor_state.last_applied_percent,
            requested_percent,
            settings.max_step_pct_per_tick,
        );

        if !bypass_backoff {
            if let Some(backoff) = monitor_state.backoff.as_ref().filter(|backoff| {
                backoff.backend == monitor.backend
                    && backoff
                        .suppress_until_epoch_s
                        .is_some_and(|until| until > now_epoch_s)
            }) {
                let remaining = state::backoff_remaining(
                    monitor_state.backoff.as_ref(),
                    monitor.backend,
                    now_epoch_s,
                )
                .unwrap_or_default();
                work.push(WorkItem::Skip(ApplyRecord {
                    logical_id: monitor.logical_id.clone(),
                    backend: Some(monitor.backend),
                    requested_percent,
                    applied_percent,
                    attempts: 0,
                    failure_kind: Some(backoff.failure_kind),
                    consecutive_failures: Some(backoff.consecutive_failures),
                    backoff_until_epoch_s: backoff.suppress_until_epoch_s,
                    status: ApplyStatus::SkippedBackoff,
                    detail: format!(
                        "{:?} failure backoff active for {}s",
                        backoff.failure_kind,
                        remaining.as_secs()
                    ),
                }));
                continue;
            }
        }

        if state::write_interval_active(
            monitor_state.last_applied_at_epoch_s,
            now_epoch_s,
            settings.min_apply_interval,
        ) {
            let elapsed_s = now_epoch_s
                .saturating_sub(monitor_state.last_applied_at_epoch_s.unwrap_or(now_epoch_s));
            work.push(WorkItem::Skip(ApplyRecord {
                logical_id: monitor.logical_id.clone(),
                backend: Some(monitor.backend),
                requested_percent,
                applied_percent,
                attempts: 0,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::SkippedMinimumInterval,
                detail: format!(
                    "last apply was {}s ago; minimum apply interval is {}s",
                    elapsed_s,
                    settings.min_apply_interval.as_secs()
                ),
            }));
            continue;
        }

        if settings.dry_run {
            work.push(WorkItem::Skip(ApplyRecord {
                logical_id: monitor.logical_id.clone(),
                backend: Some(monitor.backend),
                requested_percent,
                applied_percent,
                attempts: 0,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::SkippedDryRun,
                detail: String::from("dry_run is enabled"),
            }));
            continue;
        }

        let start_percent = monitor_state
            .last_applied_percent
            .unwrap_or(requested_percent);
        let fade_queued = if let Some(engine) = fade_engine.as_mut() {
            engine.maybe_enqueue(&monitor.logical_id, start_percent, applied_percent)
        } else {
            false
        };

        if fade_queued {
            work.push(WorkItem::Skip(ApplyRecord {
                logical_id: monitor.logical_id.clone(),
                backend: Some(monitor.backend),
                requested_percent,
                applied_percent,
                attempts: 0,
                failure_kind: None,
                consecutive_failures: None,
                backoff_until_epoch_s: None,
                status: ApplyStatus::Succeeded,
                detail: format!("fading from {start_percent} to {applied_percent}"),
            }));
            continue;
        }

        work.push(WorkItem::Dispatch {
            monitor,
            target,
            applied_percent,
            requested_percent,
        });
    }

    // -------------------------------------------------------------------------
    // Phase 2 — DISPATCH (concurrent)
    //
    // Fan out all Dispatch work items in parallel using thread::scope.
    // Each thread calls apply_monitor_target independently — no shared mutable
    // state. Results are collected into a Vec indexed by work-item position so
    // Phase 3 can stitch them back in order.
    //
    // Skipped items keep their pre-built ApplyRecord and are never dispatched.
    // -------------------------------------------------------------------------
    // Pre-allocate results: None means "skip item, no dispatch result needed".
    let mut dispatch_results: Vec<Option<Result<BackendWrite, BackendError>>> = work
        .iter()
        .map(|item| match item {
            WorkItem::Skip(_) => None,
            WorkItem::Dispatch { .. } => Some(Ok(BackendWrite {
                // placeholder — overwritten below
                backend: crate::backends::BackendKind::Backlight,
                applied_percent: 0,
                attempts: 0,
                detail: String::new(),
            })),
        })
        .collect();

    std::thread::scope(|scope| {
        // Collect (index, thread_handle) pairs for Dispatch items only.
        let handles: Vec<(
            usize,
            std::thread::ScopedJoinHandle<'_, Result<BackendWrite, BackendError>>,
        )> = work
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if let WorkItem::Dispatch {
                    monitor,
                    target,
                    applied_percent,
                    ..
                } = item
                {
                    let handle = scope.spawn(move || {
                        apply_monitor_target(
                            runner,
                            monitor,
                            target,
                            *applied_percent,
                            &settings,
                            ddcutil_profile,
                        )
                    });
                    Some((i, handle))
                } else {
                    None
                }
            })
            .collect();

        for (i, handle) in handles {
            // join() on a scoped thread cannot panic from the scope itself;
            // if the spawned closure panics, join returns Err. We convert that
            // into a CommandError-style IO failure so the backoff system handles
            // it gracefully rather than propagating the panic.
            let result = handle.join().unwrap_or_else(|_| {
                Err(BackendError::Io {
                    backend: crate::backends::BackendKind::Backlight,
                    program: String::from("apply_monitor_target"),
                    message: String::from("thread panicked during hardware dispatch"),
                    attempts: 0,
                })
            });
            dispatch_results[i] = Some(result);
        }
    });

    // -------------------------------------------------------------------------
    // Phase 3 — COMMIT (sequential, mutates state)
    //
    // Walk work items in order. Skip items contribute their pre-built record.
    // Dispatch items use the result collected in Phase 2.
    // -------------------------------------------------------------------------
    let mut summary = ApplySummary::default();

    for (item, dispatch_result) in work.into_iter().zip(dispatch_results.into_iter()) {
        match item {
            WorkItem::Skip(record) => {
                summary.push(record);
            }
            WorkItem::Dispatch {
                monitor,
                requested_percent,
                applied_percent,
                ..
            } => {
                let result =
                    dispatch_result.expect("Dispatch item must have a result after Phase 2");
                match result {
                    Ok(write) => {
                        state.record_apply_success(
                            &monitor.logical_id,
                            write.applied_percent,
                            now_epoch_s,
                        );
                        summary.push(ApplyRecord {
                            logical_id: monitor.logical_id.clone(),
                            backend: Some(write.backend),
                            requested_percent,
                            applied_percent: write.applied_percent,
                            attempts: write.attempts,
                            failure_kind: None,
                            consecutive_failures: None,
                            backoff_until_epoch_s: None,
                            status: ApplyStatus::Succeeded,
                            detail: write.detail,
                        });
                    }
                    Err(error) => {
                        let failure_kind = error.failure_kind();
                        let attempts = error.attempts();
                        let backoff = state.record_apply_failure(
                            &monitor.logical_id,
                            monitor.backend,
                            failure_kind,
                            now_epoch_s,
                        );
                        let detail = format!(
                            "{}; backing off for {}s after {} consecutive {:?} failure(s)",
                            error,
                            backoff
                                .suppress_until_epoch_s
                                .unwrap_or(now_epoch_s)
                                .saturating_sub(now_epoch_s),
                            backoff.consecutive_failures,
                            backoff.failure_kind
                        );
                        summary.push(ApplyRecord {
                            logical_id: monitor.logical_id.clone(),
                            backend: Some(monitor.backend),
                            requested_percent,
                            applied_percent,
                            attempts,
                            failure_kind: Some(backoff.failure_kind),
                            consecutive_failures: Some(backoff.consecutive_failures),
                            backoff_until_epoch_s: backoff.suppress_until_epoch_s,
                            status: ApplyStatus::Failed,
                            detail,
                        });
                    }
                }
            }
        }
    }

    summary
}

fn reassert_due(
    last_applied_at_epoch_s: Option<u64>,
    now_epoch_s: u64,
    reassert_interval_seconds: u64,
) -> bool {
    last_applied_at_epoch_s.is_none_or(|last_applied_at| {
        now_epoch_s.saturating_sub(last_applied_at) >= reassert_interval_seconds
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{testutil::FakeRunner, BackendKind};
    use crate::config::MonitorSelector;
    use crate::policy::PerMonitorTarget;
    use crate::process::{CommandError, CommandOutput, ProcessRunner};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    // -------------------------------------------------------------------------
    // SlowRunner — simulates hardware with configurable per-call latency.
    // Sync-safe via AtomicUsize; suitable for thread::scope dispatch.
    // -------------------------------------------------------------------------
    struct SlowRunner {
        delay: Duration,
        call_count: Arc<AtomicUsize>,
    }

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

    fn fast_settings() -> ApplySettings {
        ApplySettings {
            // 1% delta threshold: same-percent requests on subsequent ticks
            // will be skipped by hysteresis (delta=0 < threshold=1).
            min_write_delta_pct: 1,
            max_step_pct_per_tick: 100,
            min_apply_interval: Duration::ZERO,
            dry_run: false,
            // Large reassert interval (1 billion seconds ≈ 31 years) so it
            // never fires during the 10k-tick simulation test.
            apply_reassert_interval: Duration::from_secs(1_000_000_000),
            ddc_timeout: Duration::from_secs(5),
            backlight_timeout: Duration::from_secs(2),
        }
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
            "concurrent dispatch should complete in <1400ms, took {:?}",
            elapsed
        );
    }

    // =========================================================================
    // TEST 2 — Mixed hardware: healthy backlight, hanging DDC, missing monitor.
    //
    // Proves that one monitor's failure does not block or corrupt others.
    // =========================================================================
    #[test]
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
}
