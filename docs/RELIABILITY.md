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
- each connection carries exactly one newline-framed JSON request and one newline-framed JSON
  response; EOF and half-close are not message delimiters
- the 64 KiB limit counts JSON payload bytes and excludes the terminating LF delimiter; readers and
  writers enforce the same limit
- frame reads and writes use absolute operation deadlines; incremental peer progress does not extend
  either deadline
- malformed, unterminated, oversized, or deadline-exceeded messages return protocol errors instead
  of crashing or blocking the daemon indefinitely
- synchronous IPC draining uses a bounded request-count and wall-clock yield quantum (`16` accepted
  connections or `8 ms`, whichever comes first); queued connections are resumed on later main-loop
  iterations
- the drain quantum is cooperative, not preemptive: one already-running request may take longer
  than the quantum and can include its own frame-read, handler, and frame-write deadlines
- IPC cannot monopolize the main loop between connections, but no hard real-time tick/fade latency
  guarantee is made for an individual request or an unbounded arrival stream

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

### Integrity Reconciliation

The configured reassert interval is an integrity deadline, not permission to blindly overwrite
another controller. When an unchanged policy target reaches that deadline, the daemon reads the
current hardware value first. A matching value is left untouched; a differing value is treated as
an externally owned value for that integrity cycle and is also left untouched. If readback is
unavailable or fails, the existing write-and-backoff recovery path remains the fallback. Genuine
policy changes and explicit forced recovery actions retain their existing apply semantics.

The identity-less `ExternalBrightnessChange` IPC notification is observational only. It no longer
forces a full multi-monitor resync because the protocol does not identify a monitor or provide its
observed value; blindly reapplying policy in that case could overwrite an unrelated display.

### Linux Lifecycle Recovery

Brightness can be changed behind the daemon's back: desktop compositors such as KWin keep their
own saved DDC/CI value and write it when a display wakes, and a monitor that has just powered on
may come back at its own stored level. Wake signals are not dependable for this. DRM uevents,
logind `PrepareForSleep(false)`, and idle-resume exist only on some systems, and a DPMS transition
does not have to produce any of them, so a session can pass without a single hint.

The daemon therefore treats the hardware as the source of truth and runs a **probe schedule**
(`runtime/wake.rs`) that does not depend on any signal:

- Every `daemon.probe_seconds` (15 by default, `0` disables it) each enabled monitor is read back:
  DDC monitors over their EDID-verified connector bus (about 40 ms, no identity search), backlights
  through sysfs. A matching value produces no write; a different value is corrected at once.
- Before any DDC read, the connector's kernel power state (`status` and `dpms` under
  `/sys/class/drm`) is checked. A display the kernel reports as off or disconnected is counted as
  unreachable without any monitor traffic. This works the same on X11, Wayland, and a bare console.
- While a monitor is unreachable, probes pulse every two seconds. Those pulses are just kernel
  attribute reads, so waiting for a display to wake costs effectively nothing.
- When an unreachable monitor answers again, that **is** the wake, whatever the desktop is. It
  opens a one-minute fast window with probes every two seconds, which catches the value a
  compositor writes a few seconds after the display returns.
- A correction opens the same fast window, because whatever wrote once may write again.
- Wake signals, when a system does provide them, only open that window sooner. They are an
  accelerator, never a requirement.

Monitors without a verifiable connector cannot be probed cheaply and are left to regular ticks and
the periodic integrity deadline.

DRM events are filtered to display connector events; unrelated uevents are ignored. Logind and DRM
listener failure is non-fatal, and periodic capability observation/integrity reconciliation remains
the final safety net. A physical monitor DPMS transition does not necessarily generate a DRM hotplug
event, so the generic event sources do not claim complete detection of every monitor power-state
change. Existing idle/wake handling and periodic fallback cover that residual case where available.

The periodic integrity deadline remains authoritative for the active SunReactor policy. It reads
hardware, skips a matching value, and corrects a differing value to the current policy. It does not
adopt arbitrary external brightness as persistent ownership. Explicit SunReactor overrides remain
authoritative through their existing control path.
