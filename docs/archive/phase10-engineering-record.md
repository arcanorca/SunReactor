# SunReactor Phase 10 — Engineering Record

## Scope and final status

Phase 10 closes the async-observation semantic correction and implements the approved bus-only safety policy. No DRM/udev hotplug subsystem, async runtime, TUI work, Phase 11, or commit was introduced. The per-monitor config schema includes `allow_topology_retargeting`.

Status: **CLOSED**. Phase 10 is complete. Phase 11 and TUI work are not started.

## Exact final control flow

At daemon loop startup, each iteration drains bounded IPC, maintains idle state, computes `LoopAction`, then calls `publish_completed_capability_snapshot()`.

- If a complete successful snapshot is received while `capability_refresh_pending` is true: clear pending, call `execute_scheduled_tick`, then continue. That tick performs policy, reconciliation against the newly published snapshot, and apply.
- If a failed observation result is received while pending: clear pending and continue; no topology-dependent apply occurs.
- If pending but no result exists: wait/poll in a 16 ms slice and continue; no `perform_loop_action` runs.
- If a scheduled tick becomes due and no refresh is pending: set pending, start one worker, wait/poll in 16 ms slices, and continue.
- Other loop actions (fade/sleep) retain their existing path.

Therefore:

1. **Startup with no snapshot:** scheduled apply cannot reach hardware. First scheduled cycle starts observation and waits for a complete result.
2. **Existing snapshot N:** next scheduled cycle starts observation and does not apply with stale N. It waits for N+1.
3. **Snapshot N+1 completion:** it is consumed on the next responsive loop iteration and immediately triggers the pending cycle; it does not wait for another full tick.

## Observation worker lifecycle

`CapabilityRefreshResult` is either `Completed(CapabilitySnapshot)` or `Failed`.

- One `AtomicBool` compare-exchange allows at most one worker.
- `sync_channel(1)` prevents unbounded completed-result accumulation.
- Worker publication uses `try_send`, so a full/disconnected receiver cannot block worker shutdown.
- `CapabilityRefreshGuard` clears active state in `Drop`; normal completion and `catch_unwind` failure both clear the flag.
- Worker body is wrapped in `catch_unwind`; panic becomes `Failed`, pending cycle is cleared, and future cycles can retry.
- Receiver disconnection is treated as a failed result by `publish_completed_capability_snapshot`; no apply is performed.
- There is no generation field because at most one worker can exist and capacity is one. Thus an older result cannot overtake a newer worker result. A result that cannot be consumed is dropped rather than spawning another worker.
- The active observation `JoinHandle` is retained. On runtime drop, the daemon joins the one active worker; the worker's existing bounded `ProcessRunner` child lifecycle is therefore allowed to finish/reap before exit. The worker's result publication uses `try_send`, so a closed/full channel cannot leave it blocked.

## Deterministic tests

- `runtime::topology::tests::sequential_snapshots_disconnect_and_reconnect_without_changing_configured_identity`: A present → A absent while sibling remains present → A returns; configured logical IDs/selectors unchanged.
- `runtime::topology::tests::bus_reuse_classifies_different_occupant_as_present`: A on bus 7 → bus absent → different B on bus 7. Current raw selector semantics return `Present` for C and allow apply.
- `runtime::topology::tests::reconnect_with_non_unique_selector_stays_fail_closed`: duplicate matching evidence returns `AmbiguousOrUnsafe`.
- `runtime::topology::tests::ddc_primary_remains_single_effective_target_across_snapshot`: DDC + proven ddcci alias yields one allowed target.
- `runtime::orchestrator::tests::slow_capability_refresh_does_not_block_ipc_or_publish_partial_snapshot`: slow worker, IPC status completes under 50 ms, no partial snapshot, overlap suppressed, completed result published.
- `src/discovery/tests.rs`: detect timeout keeps backlight; one capability timeout does not remove a viable sibling.
- Existing DDC, topology, apply-gate, failure-isolation, IPC, process-timeout, suspend/resync tests remain green.

The current slow test exercises worker/IPC semantics directly. The control-loop sequencing is also enforced by the state machine: pending blocks `perform_loop_action` and completion calls `execute_scheduled_tick` exactly once.

## Bus-only policy

The current config schema has an explicit per-monitor topology-retargeting opt-in. `MonitorConfig` uses `#[serde(default, deny_unknown_fields)]`; the field defaults safely to false.

Final behavior implements approved Policy C+:

- bus-only selector matches the current bus occupant only when `allow_topology_retargeting = true`;
- without opt-in, the configured target returns `TopologyRetargetingDisabled` and is skipped before backend dispatch;
- A→absent→B on the same bus remains raw `Present` only for an explicit opt-in target;
- existing enabled legacy bus-only configs default to false and therefore fail closed;
- discovery-generated bus-only candidates are disabled and include an explicit topology-slot warning;
- unrelated healthy monitors continue normally;
- existing config is never automatically rewritten.

Compatibility: new binary + old config is compatible; new binary + new field is compatible; old binary + config containing the field is rejected by `deny_unknown_fields` (downgrade incompatibility).

The owner-approved policy is implemented; no owner decision remains pending.

Identity wording: serial is identifying evidence, not guaranteed permanent identity; serial+model is stronger evidence, not universally unique; EDID is fingerprint evidence, not globally unique; I2C bus is current topology; DRM connector is current topology location.

## Refresh duration and recovery bound

The refresh worker performs, sequentially:

1. `ddcutil --noconfig --terse detect` — timeout 4 s;
2. for every reported DDC monitor, conditional capabilities probe — timeout 3 s each;
3. brightnessctl list — timeout 2 s;
4. local sysfs discovery and alias annotation.

Worst observation duration for N detected DDC entries is bounded by `4 s + 3N s + 2 s` plus local filesystem work, assuming each external call reaches its configured timeout. It is not a fixed 72 s number. The scheduled cycle's reaction latency is: remaining time until the next scheduled due point + observation duration + loop publication/IPC poll (normally ≤16 ms) + policy/apply duration. Once a scheduled cycle is due, completion-to-apply does not wait for another cadence interval. A physical change just after an observation can remain represented by the completed snapshot until the next scheduled cycle; periodic safety bound is one cadence interval plus the next bounded observation. Default cadence remains 60 s.

## Event architecture

No event listener was added. Linux DRM/kernel hotplug is a kernel/userspace uevent interface; it is not a GNOME/KDE/compositor feature, and `NETLINK_KOBJECT_UEVENT` exists without requiring libudev merely to establish an event source. Nonetheless, event payloads are not authoritative snapshots. Future implementation must be `event → fresh observation → reconcile → apply`. Periodic reconciliation remains the correctness fallback and event-triggered refresh is a latency optimization requiring separate product approval.

## Verification matrix

Fresh final-tree commands all exit 0:

- `cargo +stable fmt --all --check`
- `cargo +stable check --workspace --all-targets --all-features`
- `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings -D clippy::dbg_macro -D clippy::todo`
- `cargo +stable test --workspace --all-targets --all-features`: 210 library tests + 6 binary tests passed; examples/doc tests passed
- `cargo +stable check --no-default-features`
- `cargo +stable test --no-default-features`: 198 library + 6 binary tests passed
- `cargo +stable build --no-default-features --bin sunreactord`
- `cargo +stable build --no-default-features --bin sunreactorctl`
- both `cargo +stable run ... --help` commands
- `bash tests/installer_test.sh`
- `bash tests/release_test.sh`
- `git diff --check`

No new warnings in all-feature strict clippy. Headless builds retain the pre-existing `compute_monitor_milestones` dead-code warning.

## Real hardware

No physical unplug/replug was performed. Read-only discovery on the available host saw two viable DDC monitors; status showed no running daemon in this environment. No brightness writes, daemon restart, install, enable, config mutation, or state mutation was performed. Phase 9's separate two-monitor live qualification remains valid but is not repeated or claimed here.

## Exact Phase 10 closure files

- `src/runtime/orchestrator.rs`
- `src/runtime/topology.rs`
- `src/discovery/tests.rs`
- `docs/phase10-owner-brief.md`
- `docs/phase10-engineering-record.md`

The repository remains an intentionally dirty Phase 1–9 tree plus Phase 10 changes. No commit, reset, restore, stash, rebase, or clean was performed. Phase 11 and TUI were not started.

## Final closure

The approved bus-only safe-default policy is implemented and verified. Phase 10 is **CLOSED**. Phase 11 and TUI work were not started.
