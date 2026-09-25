# Changelog

All notable changes to the **SunReactor** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased] - 2026-09-25

### Added
- **KDE Plasma 6 Native Panel Integration (`plasma/org.sunreactor.plasma/`)**:
  - Native QML desktop panel widget adhering to KDE Human Interface Guidelines (HIG).
  - C++ Unix domain socket IPC client (`SunReactorClient`) with non-blocking `QLocalSocket` architecture.
  - Data-only IPC boundary: passes raw primitives and epoch timestamps to QML for localized presentation.
  - Mode precedence state machine: `Offline` > `Paused` > `Manual` > `IdleDimmed` > `Automatic`.
  - Comprehensive C++ QtTest suite (`test_sunreactorclient`) covering framing, chunked streams, 64 KiB guards, and timeout recovery (15/15 tests passing).
- **Hardware-Truth Probe Schedule (`src/runtime/wake.rs`)**:
  - Signal-independent physical register polling running on a steady 15s cadence.
  - Signal-free wake derivation (`WakeReason::MonitorAnswered`): detects display power-on when an unreachable monitor first answers.
  - 60-second fast-recovery window with 2-second pulse intervals to aggressively lock brightness back to astronomical policy after compositor (e.g., KWin) wake restores.
  - Sub-millisecond zero-overhead sleep pulses querying sysfs attributes without waking hardware.
- **DRM Kernel Power Gating (`src/backends/ddc.rs`)**:
  - Pre-dispatch connector state validation querying `/sys/class/drm/<connector>/status` and `dpms`.
  - Immediate fail-fast (`attempts: 0`) when display is in standby, suspend, or disconnected, completely eliminating I2C bus lockouts and driver timeout penalties.
  - Direct bus targeting (`--bus <N>`) via sysfs EDID matching, lowering DDC transaction latency to ~40 ms.
- **Failure Classification & Exponential Backoff (`src/state/transitions.rs`)**:
  - Per-monitor error classification isolating `Transient` (I2C busy, timeout, hotplug churn) from `Persistent` (missing binaries, invalid selectors) faults.
  - Exponential backoff delay schedule: $\text{Delay} = \min(5 \cdot 2^{n-1}, 300)\text{s}$ (`5s, 10s, 20s, 40s, 80s, 160s, 300s`).
  - Independent backoff tracking: unresponsive or disconnected monitors do not block, delay, or starve healthy displays.
- **Cooperative IPC Drain Quantum (`src/runtime/loop.rs`)**:
  - Enforced bounded drain limit: maximum 16 accepted requests or 8 ms wall-clock execution per daemon main-loop iteration (`IPC_DRAIN_MAX_REQUESTS`, `IPC_DRAIN_MAX_DURATION`).
  - Guarantees incoming IPC requests cannot monopolize the thread or delay scheduled fade steps.
- **Defensive IPC Framing & Deadlines (`src/ipc/transport.rs`)**:
  - Strict 64 KiB message payload boundary (`MAX_IPC_MESSAGE_BYTES`) with newline-delimited framing (`\n`).
  - Absolute operation deadlines for both frame reads and writes; slow-drip transmissions cannot extend timeout windows.
  - Non-destructive stale socket cleanup validating file metadata and active listeners before unlinking.
- **Atomic Config Reloads & Rollback (`src/runtime/ipc.rs`, `src/config/io.rs`)**:
  - Full schema and monitor ID validation before applying config updates in memory.
  - Atomic rollback restoring previous configuration, monitor states, and weather schedulers if state persistence fails during reload.
  - Automatic stale monitor state pruning upon daemon bootstrap and reload.
- **Phase 10 Policy C+ Bus-Only Retargeting Guard (`src/runtime/topology.rs`)**:
  - Bus-only monitor selectors fail closed (`ReconcileAction::TopologyRetargetingDisabled`) unless explicitly permitted via `allow_topology_retargeting = true`, preventing accidental brightness writes when I2C bus indices shift.
- **Orthographic Globe Projection & Terminator Map (`src/tui/globe.rs`)**:
  - Vector mathematics for orthographic geographic projection (`project_orthographic`, `unproject_orthographic`).
  - Real-time astronomical solar terminator computation mapped over RLE-compressed land mask bitmaps.
- **Custom Display Fonts & Theme Blending (`src/tui/ui/fonts.rs`, `src/tui/theme.rs`)**:
  - 3-row half-block weather digits and rounded Unicode box-drawing numerals for automation readouts.
  - Centralized linear RGB color blending (`mix()`) supporting 28 complete built-in themes.
- **Updated High-Resolution Previews (`docs/images/SunReactor_{1..5}.png`)**:
  - 5 fresh TUI preview screenshots reflecting the latest modular workspaces across Amber, Terminal, Commodore 64, Cyberpunk, and Synthwave '84 themes.

### Changed
- **TUI Architecture Modularization**:
  - Decomposed monolithic TUI code into domain workspaces (`Monitors`, `Automation`, `Location`, `Weather`, `Settings`).
  - Extracted application state into dedicated slices: [`environment.rs`](src/tui/app/environment.rs), [`input_editing.rs`](src/tui/app/input_editing.rs), [`persistence.rs`](src/tui/app/persistence.rs), [`preview.rs`](src/tui/app/preview.rs), and [`milestones.rs`](src/tui/app/milestones.rs).
  - Split update reducers into specialized domain reducers in [`src/tui/update/`](src/tui/update/).
  - Colocated over 8,000 lines of unit and integration tests directly within respective domain modules.
- **Readback Reconciliation Policy (`src/apply/engine.rs`)**:
  - Periodic integrity deadlines verify hardware state before writing: matching values skip I2C bus traffic (`ApplyStatus::SkippedHysteresis`), while observed external drift is actively corrected back to astronomical policy.
- **Toolchain Pinning**:
  - Pinned Rust compiler toolchain strictly to `1.97.1` in [`rust-toolchain.toml`](rust-toolchain.toml) with native vectorization support (`target-cpu=native`).

---

## [0.1.0] - 2026-07-11

### Added
- Initial release of SunReactor adaptive brightness daemon and CLI.
- Astronomical solar elevation calculation based on geographic coordinates.
- Multi-monitor DDC/CI (`ddcutil`) and laptop backlight (`sysfs` / `brightnessctl`) support.
- OpenWeather cloud cover integration and bounded weather multipliers.
- Interactive terminal user interface (`ratatui`) with IPC daemon control.
- Systemd user service integration and zero-sudo automated installer.
- Windows DXVA2 and physical monitor backend architecture.
