# Changelog

All notable changes to **SunReactor** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.14.2] - 2026-11-27

### Added
- **Plasma desktop widget mode**: scalable live `DesktopRepresentation` (`0.55×`–`2.5×`) with zoom-aware theme picker anchoring.

### Changed
- **Nothing (`nothing`)**: replaced dot-grid dial with a polar 36-LED circular OLED dial (`NothingDial.qml`), refined `Doto` + `JetBrains Mono` `%` typography, and expanded the curve to 12 rows.
- **Nintendo DS (`nds`)**: solar-cycle 9-step 4-cell battery gauge (`NdsStatusRules.js`) and compact right-aligned status bar (`[Signal] [Battery] [Clock]`).
- **Nintendo Wii (`wii`)**: rebuilt textures, dock, buttons, and weather icons from authentic upstream Wii assets (`tools/vendor/wii/`).
- **Xbox 360 (`xbox360`)**: replaced text `Dark`/`Light` switcher with an icon-only metallic orb toggle.
- **Widget icon**: redesigned angled-split `org.sunreactor.plasma.svg` and monochrome `-symbolic.svg` with night stars.

### Fixed
- Auto-closes an open `ThemePicker` popover when the Plasma panel popup collapses.

## [0.14.1] - 2026-11-27

### Changed
- **Plasma skins**: removed the `NDS` top-bar badge from `nds`; rebuilt `xbox360` and `nothing` skins from authentic upstream assets (`tools/vendor/`).

### Fixed
- **Laptop eDP discovery**: ignored expected `Invalid display` DDC records for internal panels (`eDP`, `LVDS`, `DSI`) and missing `ddcutil` in TUI setup.
- **Installer (`install.sh`)**: fixed piped `curl | bash` runs to fetch and build the Plasma widget from the matching release tag archive.

## [0.14.0] - 2026-11-27

### Added
- **Xbox 360 skin & bundled fonts**: `xbox360` theme/skin, `Chakra Petch` & `Doto` fonts, `sunreactorctl hold clear`, configurable cloud dimming (`0..=50%`), `sunreactorctl doctor --report`, XDG autostart / OpenRC / runit / AUR packaging, `CommitThrottle.qml`, `admit_write`, and `CLOCK_BOOTTIME` step compensation.

### Changed & Fixed
- Hardened DDC readback trust (`±1%` tolerance, VCP max scaling, 15m probe cadence, `flock` queueing), `flock` control socket locking, headless Xvfb/QML QA tooling, and per-display range controls across all 15 skins.

## [0.13.0] - 2026-10-16

### Added & Changed
- **Shared TOML theme registry & Plasma redesign**: 40 built-in themes + user overrides (`~/.config/sunreactor/themes/`), `sunreactorctl theme` CLI/IPC commands, modular Plasma skin/pixel architecture, and automated Xvfb layout QA suite.

## [0.12.2] - 2026-09-28

### Added & Changed
- Added per-monitor `min_pct`/`max_pct` sliders in the Plasma popup, glare-free retro dark substrates, and offline first-poll status reporting.

## [0.12.1] - 2026-09-27

### Fixed
- Aligned release archive naming, binary version validation, `sunreactord.service` bundling, static musl builds, and atomic installer rollback.

## [0.12.0] - 2026-09-25

### Added & Changed
- **Runtime hardening & Plasma integration**: DRM sysfs power gating, 15 s wake probes, per-device exponential backoff (`5s`–`300s`), bounded IPC framing (`64 KiB`), atomic config reloads, orthographic globe projection, modular TUI workspaces, and the initial Plasma 6 widget.

## [0.1.0] - 2026-07-11

### Added
- Initial release of SunReactor daemon, CLI, `ratatui` TUI, OpenWeather cloud dimming, DDC/CI & sysfs/brightnessctl backends, and systemd user service installer.
