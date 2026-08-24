# SunReactor Phase 10 — Owner Brief

## Status

**CLOSED.** Phase 10 is complete. Phase 11 and TUI work are not started.

## What changed

Capability observation now runs in one bounded background worker. The control loop remains available for IPC while observation runs. A scheduled cycle waits for a complete capability snapshot before policy/apply; it never applies that cycle with an older snapshot or without a snapshot. A completed snapshot is consumed immediately by the loop and triggers that pending cycle's apply.

Worker publication uses a capacity-one channel and non-blocking `try_send`. An atomic active flag prevents overlapping observations. An RAII guard clears the active flag on normal completion, panic recovery, or worker unwinding. Failed observation clears the pending cycle without hardware apply; a later scheduled cycle can retry.

## Bus-only finding

A bus-only DDC selector identifies a current I2C topology slot, not a persistent physical display. Deterministic snapshots prove: A on bus 7 is `Present`; bus 7 empty is `TemporarilyUnavailable`; a different B on bus 7 is again a raw `Present` match under the current selector semantics. Without a policy gate this can redirect a write to B.

The approved per-monitor `allow_topology_retargeting` field defaults to false. Existing enabled bus-only configurations therefore fail closed with a clear diagnostic; the user must explicitly set `allow_topology_retargeting = true` to accept topology-slot retargeting. Generated bus-only candidates are disabled and include the same warning.

Compatibility: new binary + old config works; new binary + new field works; old binary + new field is rejected by `deny_unknown_fields` (downgrade incompatibility, not an old-config migration failure).

## Recovery model

Periodic refresh remains the correctness fallback and safety net. Default cadence is 60 seconds. Event-driven DRM/uevent support was not added. Events, if introduced later, are only triggers: `event → fresh observation → reconcile → apply`.

## Evidence

- Deterministic sequential A/B/C disconnect/reconnect tests pass.
- Bus-reuse A/B/C test passes and records the unsafe current behavior.
- Ambiguous reconnect remains fail closed.
- DDC-primary/proven ddcci suppression remains covered.
- Slow observation test proves IPC status handling does not wait for observation and no partial snapshot is published.
- Full matrix is green; real physical unplug/replug was not performed in this environment.

## Next action

Phase 10 is closed. Do not start Phase 11 or TUI work yet. The next technical product task is a separately scoped post-Phase-10 task; this closure does not authorize it.
