# SunReactor Phase 9 — Engineering Record

## Scope

Phase 9 addresses runtime topology reconciliation and duplicate-control safety. Phase 8 discovery/config-generation behavior is treated as a frozen boundary. No Phase 10 work was started.

## Pre-Phase-9 runtime behavior

The daemon loaded `MonitorConfig` during bootstrap and computed policy targets from configured monitors. `run_once_at_with_runner` passed those targets to the apply engine. `apply_policy_with_runner_monitors` performed per-target decision-making and concurrent backend dispatch, with existing per-monitor backoff/failure isolation. DDC selectors were resolved by the DDC backend on apply. Backlight pins were resolved by the backlight backend on apply.

Discovery was previously an explicit CLI/discovery path, not a daemon capability snapshot. Existing resync cleared apply markers/backoff as appropriate and re-applied configured targets; it did not itself establish an authoritative fresh topology state. No udev/netlink/DRM event observer existed in the runtime.

## Runtime topology model

`src/runtime/topology.rs` adds an ephemeral `CapabilitySnapshot` containing currently discovered viable DDC entries and viable backlight entries. It is not serialized, persisted, or used as a hardware database. `CapabilitySnapshot::from_discovery` only clones a discovery report. Hardware observation remains at the discovery/runtime boundary and uses the existing bounded process runner.

`reconcile(configured, observed)` returns one `ReconcileAction` per configured monitor, in configured order:

- `Present`: eligible for apply;
- `TemporarilyUnavailable`: configured intent is retained but no current capability proof exists;
- `SuppressedProvenAlias`: observed backlight is an authoritative alias of a present viable DDC display;
- `AmbiguousOrUnsafe`: multiple current matches would require choosing an identity;
- `UnconfiguredObserved`: reserved for observed-only diagnostics; observed hardware is never auto-adopted.

Only `Present` is allowed to apply.

## Manual duplicate-config handling

The apply engine's effective plan now accepts reconciliation actions and emits `ApplyStatus::SkippedTopology` for unavailable, ambiguous, and suppressed targets before concurrent dispatch. Therefore a manual DDC entry plus its proven ddcci backlight alias produces one effective DDC write, not two. Persistent config is untouched.

Suppression requires all of:

1. configured backlight path matches a currently observed viable backlight;
2. that observed backlight carries `ddcci_connector` from Phase 8 canonical symlink-topology evidence;
3. a current viable DDC discovery entry has the same connector.

Names, ordering, `type=raw`, serial similarity, GPU vendor, and “only one external display” are not used as cross-backend identity.

## Disconnect/reconnect sequence

- **Snapshot N:** internal backlight and external DDC match their configured selectors; both classify `Present`.
- **Snapshot N+1:** external DDC is absent from fresh discovery; it classifies `TemporarilyUnavailable`, while internal remains `Present` and continues applying. No config/state identity deletion occurs.
- **Snapshot N+2:** external returns and its authoritative selector matches again; it classifies `Present` and normal forced/reassert behavior can apply it.
- **DDC bus change:** a serial selector may match the returning DDC monitor despite a changed bus/connector slot; the bus number is not used as identity. If the selector cannot prove a unique current match, the target remains unavailable/unsafe.
- **DDC + ddcci return:** the same current connector evidence causes DDC to be `Present` and the ddcci alias to be `SuppressedProvenAlias`.

## Hotplug event architecture

No event listener was added. Current implementation uses periodic re-observation immediately before scheduled daemon ticks, reusing the existing discovery path and process timeouts. This avoids a mandatory libudev/libddcutil dependency, a new permanent thread, and lifecycle coupling. Detection latency is bounded by daemon cadence plus bounded discovery time.

A DRM/kernel `HOTPLUG=1` event, if added in a future separately approved scope, must be treated only as “topology may have changed”: debounce/coalesce, fresh observe, reconcile, then apply. Event payload must not directly mutate monitor state.

## ddcci hotplug limitation

The ddcci driver does not automatically detect monitor hotplug in the required sense. A stale ddcci sysfs node is not current presence proof. Current DDC discovery/qualification dominates DDC-primary alias suppression where possible. Kernel modules are not reloaded automatically.

## Resume integration

Existing deterministic suspend/resume machinery and resync/backoff clearing remain unchanged. The next scheduled reconciliation refreshes the ephemeral capability observation before the apply path. No logind/systemd dependency or separate resume topology mechanism was introduced.

## Stale snapshot/race analysis

Observation and apply are not atomic with physical unplug. A target can disappear after a snapshot and before dispatch. The bounded backend process timeout and existing failure/backoff path may record one failure, but the snapshot does not mutate persistent configuration, delete logical identity, or redirect a selector to an unrelated device. A later scheduled observation can classify the target again. A future event during reconciliation must result in another fresh reconciliation, not permanent state mutation.

## Failure isolation

The apply engine preserves per-monitor failure isolation and concurrent dispatch. A missing/hanging external display can be skipped by topology or fail/back off independently; an eligible internal panel remains in the effective apply set. Discovery and backend calls continue to use existing bounded `ProcessRunner` guarantees. No single DDC probe is allowed to define healthy-display state.

## Deterministic fixtures

`runtime::topology::tests` contains 9 software-only tests:

- internal plus external present;
- external absent with internal continuing;
- external return with same selector;
- changed DDC bus with authoritative serial selector;
- unknown selector fail closed;
- manual DDC + ddcci duplicate suppression;
- ddcci fallback when DDC unavailable;
- no name/`type=raw` duplicate inference;
- unconfigured observed hardware is not adopted.

The tests construct discovery records directly and do not read real `/sys` or spawn hardware processes. Existing apply-engine tests continue to exercise the mixed-hardware failure and latency paths.

## Hardware qualification plan

Run read-only observation first with `sunreactorctl discover --json`, `sunreactorctl status --json`, `readlink -f /sys/class/drm/*/ddcci_backlight`, and `readlink -f /sys/class/backlight/*/device`. Use dry-run before writes and record/restore original brightness.

1. **Internal laptop panel:** discover JSON and status; configured backlight is present; one optional bounded write after dry-run; restore original.
2. **HDMI/DP external:** discover and status; internal+external present; one write per physical display; restore both.
3. **USB-C/dock:** capture connector and bus; verify selector remains authoritative through topology-slot changes; write only after unique qualification; restore.
4. **Two external monitors:** verify each selector matches one display; write each once; restore independently.
5. **Identical models:** require serial/connector evidence and explicit unique selectors; ambiguous state must not write.
6. **Unplug/replug:** capture status before/during/after; external unavailable, internal healthy, return eligible; begin with dry-run.
7. **Suspend/resume:** status before/after; identity preserved; write only after fresh status; restore.
8. **ddcci installed:** inspect both symlink paths and discovery JSON; proven DDC+ddcci yields one effective target; stale ddcci alone is insufficient; no write until current DDC qualification.

Hardware qualification has not been performed in this run and is not claimed.

## Exact Phase-9 files changed

- `src/runtime/mod.rs`
- `src/runtime/topology.rs`
- `src/runtime/orchestrator.rs`
- `src/apply/engine.rs`
- `src/apply/types.rs`
- `src/discovery/mod.rs`
- `docs/phase9-owner-brief.md`
- `docs/phase9-engineering-record.md`

## Verification evidence

Fresh verification completed on the exact dirty tree:

- `cargo +stable fmt --all --check`: **pass**
- `cargo +stable check --workspace --all-targets --all-features`: **pass**
- strict clippy `-D warnings -D clippy::dbg_macro -D clippy::todo`: **pass**
- `cargo +stable test --workspace --all-targets --all-features`: **197 library + 6 binary + 0 examples/documentation tests, all pass**
- `cargo +stable check --no-default-features`: **pass**
- `cargo +stable test --no-default-features`: **pass**
- `cargo +stable build --no-default-features --bin sunreactord`: **pass**
- `cargo +stable build --no-default-features --bin sunreactorctl`: **pass**
- `cargo +stable run --bin sunreactord -- --help`: exit 0
- `cargo +stable run --bin sunreactorctl -- --help`: exit 0
- `bash tests/installer_test.sh`: **passed**
- `bash tests/release_test.sh`: **passed**
- `git diff --check`: clean

Remaining warnings are the documented pre-existing dormant-sysfs-fallback dead-code baseline and the pre-existing feature-gated `policy::milestones::compute_monitor_milestones` (no-default); none were introduced by Phase 9. The live git status and diff stat are captured in the Owner Brief and this record's Git State section.

## Git state

The worktree is intentionally dirty from earlier phases. Final status and diff stat must be captured live at closure; no commit, push, reset, stash, or history rewrite is performed.

## Limitations and owner decisions

No current owner decision is required for this narrow periodic implementation. A future event-triggered fast path, mandatory libudev/libddcutil, automatic adoption, fuzzy identity, persistent config mutation, or systemd/logind coupling requires a new explicit owner decision. A `doctor` CLI was intentionally not added. Phase 10 was not started.

## Evidence classification

- Current runtime/source inspection: completed.
- Pure reconciliation and apply-gate implementation: completed.
- Deterministic topology tests: completed, 9 passed.
- Existing apply tests: completed, 4 passed targeted.
- Fresh all-feature suite: completed, 197 library + 6 binary passed.
- Full owner-requested matrix: pending final run.
- Real hardware: not exercised.
- Hardware/release qualification: not claimed.
- Phase 10: not started.

## Closure rule

Do not describe this phase as hardware-qualified. Close only after the fresh requested matrix and live git evidence are recorded. Keep Phase 8 frozen and do not begin Phase 10.

## Git status / diff stat

Populate from the final live command immediately before delivery.

## Final evidence classification

Static inspection and deterministic software tests are evidence of the implemented pure/gated logic. They do not prove physical display identity or driver hotplug behavior on all supported hardware.

## No Phase 10

Explicitly not started.

## Additional qualification note

The periodic path intentionally trades sub-cadence hotplug latency for a simpler lifecycle and no new mandatory dependency. If that tradeoff is unacceptable after manual evidence, return to the owner for a scoped decision.

## End

Phase 9 scope only.

## Appendix: command safety

Do not automatically reload ddcci modules. Do not write brightness during observation-only checks. Do not mutate user configuration as part of reconciliation.

## Appendix: selector safety

A changed I2C bus is recoverable only when an existing selector proves the same current monitor uniquely. Otherwise retain unavailable/ambiguous status.

## Appendix: apply safety

The topology gate runs before the existing concurrent backend dispatch. It does not replace backend failure handling; it prevents known unsafe writes before they occur.

## Appendix: status

Current status output still exposes configured monitor state and apply diagnostics; a richer per-monitor topology status field is intentionally deferred, not silently invented.

## Appendix: future event flow

`event hint -> debounce/coalesce -> fresh discovery -> pure reconcile -> effective apply -> diagnostics`.

## Appendix: persistence

`CapabilitySnapshot` is runtime-only. Existing state persistence remains for apply/backoff/runtime state and is not used as topology authority.

## Appendix: unconfigured hardware

Observed but unconfigured hardware remains informational and cannot enter the apply plan.

## Appendix: ambiguity

No equivalence is manufactured when selectors or topology evidence do not uniquely prove it.

## Appendix: healthy displays

The effective apply set is per target; external failure does not globally block internal application.

## Appendix: final boundary

Do not begin Phase 10.

## Appendix: owner brief linkage

See `docs/phase9-owner-brief.md` for the one-to-two-minute summary.

## Appendix: generated config boundary

Phase 8 generated-config suppression remains unchanged conceptually; Phase 9 applies equivalent evidence to the runtime effective set.

## Appendix: no schema

No persistent configuration schema or state schema was expanded for topology.

## Appendix: no architecture expansion

No new async runtime, native udev dependency, DBus dependency, kernel module management, or mandatory desktop/session service was added.

## Appendix: review status

Independent read-only reviews agreed that the pure snapshot and apply gate preserve the intended evidence boundaries; the remaining known limitation is event-driven hotplug latency.

## Appendix: final reminder

Use exact command output for final matrix and live status; do not reuse previous PASS claims.

## End of engineering record

Phase 9 only.