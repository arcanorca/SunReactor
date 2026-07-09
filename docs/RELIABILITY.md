## sunreactor Reliability Strategy

sunreactor keeps the control loop simple:

- policy and solar computation stay deterministic and separate from hardware failures
- backend writes fail per monitor, not per tick
- repeated device failures back off exponentially per monitor/backend
- healthy monitors continue applying even when one device is broken

### Failure Classification

Apply failures are classified as either:

- `transient`
  - timeouts
  - busy or temporarily unavailable DDC devices
  - temporarily missing monitor or backlight devices after resume or hotplug churn
- `persistent`
  - invalid selectors
  - missing helper programs such as `ddcutil` or `brightnessctl`
  - stable command failures that do not look recoverable

This classification is used for logs and persisted backoff state.

### Backoff

Every configured monitor keeps independent failure backoff in runtime state.

- base delay: 5 seconds
- exponential growth: `5s, 10s, 20s, ...`
- maximum delay: 300 seconds
- success clears the monitor backoff immediately
- changing backend or failure class resets the backoff sequence for that monitor

Backoff applies to both transient and persistent apply failures. This keeps broken devices from
spamming logs or wasting subprocess work every tick.

### Temporary Device Loss

sunreactor does not remove configured monitors when hardware disappears temporarily.

- if a device vanishes after suspend/resume, hotplug churn, or a flaky DDC transaction, the write
  fails and enters transient backoff
- the daemon keeps evaluating solar/policy state and keeps applying healthy monitors
- when the device returns, the next successful write clears backoff automatically

### Stale State Repair

Runtime state is pruned against the current configured monitor set on:

- daemon bootstrap
- successful config reload

This removes:

- monitor apply/backoff state for deleted monitors
- per-monitor manual overrides for deleted monitors

Global overrides, suspend state, and weather cache remain intact because they are not tied to a
single monitor identity.

### Config Reload Safety

Reload is fail-safe:

- new config is loaded and validated before replacing the active config
- if reload fails, the daemon keeps the previous in-memory config
- successful reload resets weather refresh scheduling and prunes stale monitor state
- state save failures during reload roll both config and state back to the previous values

### IPC Safety

The control socket stays local-only and intentionally small.

- stale socket cleanup only removes an existing path when it is a Unix socket and no listener is
  reachable
- non-socket files at the socket path are never removed
- each IPC message has a bounded maximum size of 64 KiB
- malformed or oversized messages return protocol errors instead of crashing the daemon

### Logging

Tick logs include:

- tick duration
- monitors evaluated
- writes attempted, skipped, succeeded, failed
- transient failure count
- persistent failure count
- backoff skip count
- degraded monitor count

Per-monitor failure logs are emitted on actual apply failures, but not on every backoff-skipped
tick. Repeated failures are only re-logged on early counts and power-of-two counts to keep logs
readable on flaky hardware.
