# Changelog

All notable changes to the **SunReactor** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.13.0] - Unreleased

### Added
- **Theme registry (`src/theme/`)**: a theme is one TOML file with `id`, `name`,
  `aliases`, `family` (`pixel` | `instrument` | `terminal`), `extends`, a ten-colour
  `[palette]`, `[tui]` and `[widget]` tokens. Validation reports every problem in a
  file at once, with the file name and key. Family defaults fill the widget tokens
  a file leaves out. Lookup accepts an id or an alias, ignores case, treats `-` and
  spaces as `_`, and ignores `_` when that is unambiguous (`vfdhifi` finds `vfd_hifi`).
  Unknown names get the nearest matches as suggestions.
- **40 built-in themes**, embedded from `src/theme/builtin/*.toml`. They include
  the flagship themes (Nothing, Casio Digital, Commodore 64, Amiga, Cyberpunk,
  Synthwave '84, Hacker Green, Windows XP, macOS, PlayStation 3, Nintendo Wii and
  Nintendo DS; the last five are new in this release) and the retro and editor
  palettes. This registry replaces the 0.12.x Wardrobe catalogue of 35 themes.
- **User themes** in `~/.config/sunreactor/themes/<id>.toml`. The file stem must
  equal `id`, and a user file shadows the built-in with the same id. A broken user
  file is logged (`theme_file_skipped`), skipped, and never stops the daemon.
- **`sunreactorctl theme`**: `list [--json]`, `show <id> [--json]`, `set <id>`,
  `validate [file]` and `export <id>`. `theme set` writes the canonical id into
  `tui.theme` and reloads a running daemon; it also works while the daemon is
  offline. `themes` is accepted as an alias of `theme`.
- **IPC additions** (backward compatible): `list_themes` (theme summaries),
  `get_theme {id}` (one fully resolved theme), `day_curve {monitor_id?}` (today's
  policy curve for the widget's light cycle). `status` gains `theme` (palette and
  widget tokens) and `theme_warning`, which is set when `tui.theme` cannot be used
  as configured and the daemon has fallen back.
- **Validation tooling**: `config validate` rejects an unusable `tui.theme`;
  `doctor` reports `SR-THEME-UNRESOLVED` (error) and `SR-THEME-FILE-INVALID`
  (warning per file).
- **Documentation**: `docs/THEMES.md` (format, user themes, reload behaviour, the
  widget's follow, pin and override settings, the built-in and flagship list, a
  worked example) and `docs/README.md`, an index of the documents in `docs/`.
- **Plasma widget redesign (`plasma/org.sunreactor.plasma/`)**:
  - *Module architecture.* The popup is assembled from `contents/ui/modules` in the
    theme's `modules` order. The module contract is written at the top of
    `modules/Modules.js`, which also holds the skin table. `engine/` resolves tokens,
    derived shades and fonts. `panel/` draws the panel item.
  - *Pixel renderer.* `engine/PixelSurface.qml` and the C++ primitives in `plugin/`
    (`pixelart`, `pixelsprite`, `pixelitems`, `glyphtables`, `dotmatrix`) draw glyph
    faces, dot-matrix digits, the day curve and recoloured sprites. Art pixels are
    sized from the device pixel ratio.
  - *Flagship skins.* Twelve themes have their own skin folders: Amiga Workbench,
    Casio Digital, Commodore 64, Cyberpunk, Hacker Green, macOS, Nintendo DS,
    Nothing, PlayStation 3, Synthwave '84, Nintendo Wii and Windows XP. Five
    instrument themes get their own identity inside the shared instrument skin
    (Braun, Nixie, Oscilloscope, ThinkPad, VFD Hi-Fi). Pixel and terminal families
    have shared skins.
  - *Weather icon styles.* `weatherIconStyle` is `auto` (each theme's own art),
    `modern` (full colour, the same in every theme), `line`, `solid` or `pixel`
    (16 px sprites). The panel and the popup follow the same setting.
  - *Panel modes.* `sky`, `readout`, `sparkline` and `ring`, with optional outside
    temperature and sun elevation.
  - *Theme picker.* Follow the terminal UI's theme or pin one, with a swatch
    preview. The theme button sits in the tab row, with a dot while following and
    a lock while pinned.
  - *Settings.* Theme mode and pin, token overrides (accent, density, pixel scale,
    renderer), module order and visibility, weather icon style, panel mode and
    display, and an "Open themes folder" action.
  - *Popup.* Brightness and Weather tab pages when a theme lists modules for both.
    A status lamp in the tab row stands in for the status line, which starts
    hidden. The automation mode switch replaces the footer Pause button. The popup
    window fits its page on screen, so a stored popup height no longer keeps the
    window the wrong size.
  - *QA tooling.* A headless screenshot harness (`tools/shot.sh`) runs the widget
    under Xvfb with a private D-Bus and a fake daemon (`tools/fake_daemon.py`). It
    also has a layout overflow audit across themes, pages, device pixel ratios and
    font DPI (`overflow_audit.sh`, `overflow_summarize.py`, `harness/Audit.qml`),
    pixel conformance checks (`pixel_proof.sh`, `conformance.py`), gallery sheets
    (`glyph_gallery.sh`, `dot_gallery.sh`, `icon_sheet.sh`), `contact_sheet.py`, and
    fixtures generated from the real CLI (`gen_fixtures.py`). Tests run through
    CTest: `test_sunreactorclient`, `test_dotmatrix`, and the QML suites
    `tst_engine.qml` and `tst_picker.qml`.
- **Installer**: an optional Plasma widget step. `--with-plasma` and `--no-plasma`
  choose it explicitly and `--dry-run` prints the plan. It builds the plugin with
  CMake, copies the plugin and QML module under `~/.local`, replaces the widget
  package, and keeps the previous widget in `~/.local/share/sunreactor-backups/`.
  It never restarts plasmashell and never fails the installation.

### Changed
- **TUI**: palettes come from the registry (`src/tui/theme.rs`). The theme picker
  lists built-in and user themes. The light-cycle preview uses the day-curve
  sampler that the daemon also uses (`policy::preview`). The large digits of the
  Automation tab follow `[tui] glyph_style`.
- **Families**: `casiodigital`, `cyberpunk` and `synthwave84` are now instrument
  themes, and `hackergreen` is a terminal theme. Nothing became an instrument
  theme and draws its dot-matrix primitives in the widget.
- **Plasma widget**: the popup and the panel render from the daemon's theme and
  day curve. The 0.12.x Wardrobe directory (`contents/ui/wardrobe/`, with
  `ThemeCatalog.js`, `ThemeTokens.qml` and `WardrobeCard.qml`) is removed.
- **Weather icons**: redrawn for Nintendo Wii, Nintendo DS, Windows XP (Luna)
  and the handheld theme, which uses the four DMG shades only.
- **Documentation**: README gains a Plasma widget section and links to the theme
  guide; the 0.12.2 Wardrobe notes stay as history.

### Removed
- The Wardrobe texture set, 12 SVG files under
  `plasma/org.sunreactor.plasma/package/contents/images/` (Game Boy, Amiga boing
  ball, Nixie mesh, VFD grid and statoscope reticles). Nothing loaded them, and
  the CMake package no longer installs that directory. The empty Wardrobe
  directories are gone too.

### Fixed
- A broken user theme never stops automatic brightness. The daemon falls back to
  the built-in with the same id (or `amber`) and reports `theme_warning`.
- The widget's theme picker fills in as the daemon's theme list arrives.
- The instrument light cycle no longer hangs on its first curve. The terminal
  light cycle percent rows use the fill scale.
- Popup content no longer overflows the popup's right edge. Layout overflows and
  collisions in the NDS, Synthwave '84, pixel and Amiga skins are fixed.

## [0.12.2] - 2026-09-28

### Added
- **KDE Plasma 6 Thematic Wardrobe Engine (`plasma/org.sunreactor.plasma/`)**:
  - Introduced the Wardrobe architecture inspired by Statoscope: dynamic token resolution across 35 themes categorized into 10 distinct visual archetypes (`ThemeCatalog.js`, `ThemeTokens.qml`, `WardrobeCard.qml`).
  - Bespoke archetype delegates: Nothing OS dot-matrix tracks & lucent frosted glass, Handheld 4-shade LCD pixel displays & DMG-01 frames, Amiga Workbench 3D bevels & Boing Ball pulse indicators, Braun ET66 concave calculator buttons & DIN status lights, VFD Hi-Fi smoked glass & cyan-green segment bar gauges, Nixie cold-cathode neon glow & anode wire mesh, Unix Workstation Motif bevels, and ThinkPad Raven styling.
  - Bundled curated typography in `contents/fonts/`: Geist Sans/Mono, Press Start 2P, VT323, Sysfont, IBM Plex Mono, and Orbitron.
  - Bundled vector assets and textures in `contents/images/`: Game Boy DMG/Pocket/Color SVG vectors, Amiga Boing Ball vector, Nixie wire mesh tiles, VFD fluorescent grid tiles, and reticle patterns.
- **Interactive Day/Night Limit Controls (`min_pct` / `max_pct`)**:
  - Extended IPC control protocol with `SetMonitorLimits { monitor_id, min_pct, max_pct }` and exposed `min_pct` / `max_pct` in `MonitorStatus`.
  - Automatic persistence to `config.toml` upon limit adjustment with immediate forced daemon tick re-evaluating solar projection and applying hardware brightness.
  - Interactive GNOME HIG drawer per monitor in `DisplayItem.qml` with dual sliders for Night Min % and Day Max %.
  - One-click "Auto" button to instantly clear manual overrides and restore natural solar curve tracking.

### Changed
- **Glare-Free Dark Substrate Palettes**:
  - Overhauled retro themes (`Handheld`, `ClassicMacintosh`, `Amiga`, `Commodore64`, `CasioDigital`) to eliminate blinding light backgrounds, replacing them with authentic dark LCD matrix substrates (`#141C12`), 1-bit CRT carbon (`#121212`), and Amiga slate chassis (`#141820`) with Boing Ball / Guru Meditation red accents (`#EE2233`).
  - TUI automation output typography dynamically switches between retro pixel-art digits and modern rounded digits depending on the active theme archetype.

### Fixed
- Report a failed first daemon status poll as offline, so the TUI no longer remains stuck on "Connecting" when the service is stopped.
- Relaxed pipe buffer timeout deadline in `src/process.rs` to 400ms to eliminate multi-core thread scheduling race conditions.

## [0.12.1] - 2026-09-27

### Fixed
- Validate release binaries against the Cargo version before packaging and against the selected tag before installation, preventing stale executables from being published or installed under a newer release.
- Include the systemd user service template in every release archive and render it from the downloaded archive.
- Align `v`-prefixed Git tags, unprefixed archive names, SHA-256 manifests, and ABI metadata between packaging, CI, and the installer.
- Build musl release binaries without a dynamic loader and reject musl artifacts that still request an interpreter or shared libraries.
- Restore the previous binaries and service unit after a failed update, restart an already active daemon, and verify IPC readiness before reporting service setup as successful.

## [0.12.0] - 2026-09-25

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
