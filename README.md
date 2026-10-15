<div align="center">
  <img src="docs/images/banner.svg" alt="SunReactor Logo" width="95%">
  <br/>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/Built_with-Ratatui-000?logo=ratatui&logoColor=fff&labelColor=201a16&color=ffd970" alt="Built with Ratatui" /></a>
  <img src="https://img.shields.io/badge/Language-Rust_1.80+-orange?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Platform-Linux_|_Windows-blue" alt="Platform" />
  <img src="https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg" alt="License" />
</div>

<br/>

**SunReactor** is a background daemon that sets display brightness from the sun's elevation at your location. It runs as an unprivileged user service, written in Rust. Weather is an optional, bounded correction on top of the solar curve. It drives external monitors over DDC/CI and laptop panels through backlight devices, and it ships with a terminal UI, a CLI and a KDE Plasma 6 widget.

---

## // PREVIEW (TUI WORKSPACES & THEMES)

SunReactor features an immediate-style, terminal UI built with `ratatui` with 40 built-in themes (and your own TOML themes), a daylight globe and live device status:

<div align="center">
  <img src="docs/images/SunReactor_1.png" alt="Monitors Workspace (Amber Theme)" width="48%" style="margin: 0.5%; border-radius: 8px;" />
  <img src="docs/images/SunReactor_2.png" alt="Automation & Light Cycle (Terminal Theme)" width="48%" style="margin: 0.5%; border-radius: 8px;" />
  <br/>
  <img src="docs/images/SunReactor_3.png" alt="Solar Location & Globe (Commodore 64 Theme)" width="31%" style="margin: 0.5%; border-radius: 8px;" />
  <img src="docs/images/SunReactor_4.png" alt="Weather Forecast & Temperature (Cyberpunk Theme)" width="31%" style="margin: 0.5%; border-radius: 8px;" />
  <img src="docs/images/SunReactor_5.png" alt="Theme Selector Modal (Synthwave '84 Theme)" width="31%" style="margin: 0.5%; border-radius: 8px;" />
</div>
<br/>

---

## // RECENT CHANGES

Details are in [CHANGELOG.md](CHANGELOG.md). The main points:

**Next release (0.13.0)**
- Themes are TOML data shared by the terminal UI and the Plasma widget. 40 are built in; you can add your own in `~/.config/sunreactor/themes/` ([docs/THEMES.md](docs/THEMES.md)).
- New `sunreactorctl theme` commands, and new IPC requests (`list_themes`, `get_theme`, `day_curve`) used by the widget.
- The Plasma widget was rewritten around that theme data: see [Plasma widget](#-plasma-widget).

**0.12.0**
- Monitor power state is read from DRM sysfs before DDC commands are sent, so sleeping or disconnected displays are skipped.
- A periodic hardware probe and a short fast-retry window after wake re-apply the target brightness when the compositor resets it.
- Failures are tracked per display with exponential backoff (5 s up to 300 s), so one slow monitor does not delay the others.
- Each daemon tick serves a limited number of IPC requests (16, or 8 ms), so a busy client cannot disturb fades.
- IPC frames are newline-delimited with a 64 KiB limit and read/write deadlines.
- Config reloads are validated before they are applied, and a bad edit is rolled back.
- Terminal UI split into Monitors, Automation, Location, Weather and Settings workspaces.
- First version of the Plasma widget.

---

## // ARCHITECTURE

The runtime is built around failure isolation and non-blocking hardware access, with few dependencies.

```text
 ┌────────────────────────────────────────────────────────────────────────┐
 │                         SunReactor Core Daemon                         │
 │                                                                        │
 │  ┌───────────────────────┐              ┌───────────────────────────┐  │
 │  │ Solar Ephemeris Math  │              │ Weather Client (Forecast) │  │
 │  │ Deterministic Curves  │              │ Bounded Attenuation Coeff │  │
 │  └───────────┬───────────┘              └─────────────┬─────────────┘  │
 │              │                                        │                │
 │              └───────────────────┬────────────────────┘                │
 │                                  ▼                                     │
 │                     ┌─────────────────────────┐                        │
 │                     │ Target Policy Evaluator │                        │
 │                     └────────────┬────────────┘                        │
 │                                  │                                     │
 │  ┌───────────────────────────────┴───────────────────────────────┐     │
 │  │         Hardware Truth Reconciliation & Probe Engine          │     │
 │  │  - DRM Sysfs Power Gate (/sys/class/drm/*/dpms, status)       │     │
 │  │  - Connector Direct Bus Addressing (Zero DDC scan bus locks)  │     │
 │  │  - Readback Verification (Prevents compositor/KWin thrashing) │     │
 │  │  - Per-Device Exponential Backoff (5s -> 300s)                │     │
 │  └───────────────┬───────────────────────────────┬───────────────┘     │
 │                  │                               │                     │
 └──────────────────┼───────────────────────────────┼─────────────────────┘
                    ▼                               ▼
       ┌────────────────────────┐      ┌─────────────────────────┐
       │ External Monitors (DDC)│      │ Internal Laptop Panels  │
       │  I2C / ddcutil bus     │      │   Linux sysfs backlight │
       └────────────────────────┘      └─────────────────────────┘
```

### 1. Hardware state and monitor wake
- Before sending DDC commands, the daemon reads the connector state from the kernel (`/sys/class/drm/*/status` and `dpms`) and skips displays that are asleep or gone.
- Wake notifications can be lost, so a probe runs every `probe_seconds` (default 15 s). After a monitor comes back, a 60 s window of faster probes re-applies the target if the compositor reset it.

### 2. Failures and backoff
- Errors are tracked per monitor, so one slow display does not delay the others.
- A failing monitor backs off exponentially (5 s, 10 s, 20 s, up to 300 s) and recovers on its first successful write.

### 3. IPC limits
- Each daemon tick serves at most 16 requests or 8 ms of IPC, so clients cannot disturb fades.
- Messages are newline-delimited JSON, up to 64 KiB, with read and write deadlines.
- Stale sockets are only removed after checking that nothing is listening.

### 4. Fail-Safe Configuration & State Pruning
- A config reload is validated first; if it fails, the previous configuration stays active.
- State left over from displays removed from `config.toml` is cleared on reload.

---

## // MATHEMATICAL FORMULATION

SunReactor computes target brightness deterministically without requiring constant network connectivity:

```text
        ☀ (Solar Noon) --> Max Brightness (DayPeak)
       /  \
     /      \ (Smooth transition via parametric gamma curve)
    /        \
- 0° (Horizon) -----------------------------------------------
                \
                  \ ☾ (Astronomical Night) --> Min Brightness (NightFloor)
```

1. **Normalized Solar Position ($t$):**
   $$t = \text{clamp}\left(\frac{\theta - \theta_{\text{night}}}{\theta_{\text{day}} - \theta_{\text{night}}}, 0, 1\right)$$
   Where $\theta$ represents the instantaneous solar elevation angle calculated from the local latitude, longitude, and UTC timestamp.

2. **Smoothstep Hermite Interpolation ($s(t)$):**
   $$s(t) = t^2 (3 - 2t)$$

3. **Per-Monitor Curvature Easing:**
   $$\text{Curve}(t) = [s(t)]^\gamma$$
   Where $\gamma$ is a user-tunable easing exponent configured independently per display (`transition_gamma`).

4. **Multi-Factor Clamped Projection:**
   $$\text{Brightness}_{\text{target}} = \text{clamp}\Big(\text{MinPct} + (\text{MaxPct} - \text{MinPct}) \cdot \text{Curve}(t) \cdot \text{Gain} \cdot W_{\text{mult}}, \;\text{MinPct}, \;\text{MaxPct}\Big)$$
   Where $W_{\text{mult}} \in [0.75, 1.0]$ is the bounded cloud-cover multiplier derived from OpenWeather forecast intervals.

---

## // ECOSYSTEM & INTEGRATIONS

### 1. Interactive Terminal UI (`sunreactorctl tui`)
- Full keyboard-driven navigation with browser-style shortcuts (`1`-`5` tabs, `Tab`/`Shift+Tab`).
- Real-time ASCII light cycle and 24-hour solar trajectory visualizer.
- Interactive orthographic globe with real-time solar terminator rendering.
- 40 built-in themes including Amber, Terminal, Commodore 64, Cyberpunk, Synthwave '84, Gruvbox, Nord, Tokyo Night, and Catppuccin Mocha; themes are shared with the Plasma widget and can be extended with user TOML files (see [Themes](#themes)).

### 2. Native KDE Plasma 6 Desktop Integration
A KDE Plasma 6 panel widget lives in [`plasma/`](plasma/). It shows the current brightness, the day curve and the weather, and lets you pause automation or set a manual brightness per display.
- Installation, settings and troubleshooting are in [Plasma widget](#-plasma-widget).

### 3. Scriptable Control CLI (`sunreactorctl`)
```bash
sunreactorctl status               # Inspect live daemon state, solar angles, and device topology
sunreactorctl discover             # Probe and identify all connected DDC and sysfs monitors
sunreactorctl set desk 75          # Apply a manual brightness override to a single monitor
sunreactorctl set --global 60      # Temporarily lock all displays to 60%
sunreactorctl suspend --minutes 90 # Pause brightness updates for 90 minutes
sunreactorctl resume               # Clear overrides and return to the active solar curve
sunreactorctl reload-config        # Atomically validate and reload config.toml
sunreactorctl theme list           # Built-in and user themes (* marks the configured one)
sunreactorctl theme set nord       # Set tui.theme and reload the daemon
```

---

## // INSTALLATION & COMPATIBILITY

### System Requirements
- **OS:** Linux (Kernel $\ge 5.15$, glibc $\ge 2.34$ or musl) or Windows 10/11.
- **Arch / CachyOS / Fedora / Debian / Ubuntu / openSUSE** fully qualified.
- **Hardware Backends:**
  - External Displays: `ddcutil` ($\ge 1.4.1$, recommended $\ge 2.2.0$) with user access to `/dev/i2c-*` (`uaccess` or `i2c` group).
  - Laptop Panels: Linux `sysfs` or `brightnessctl`.

### Quick Automated Installation
```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash
```

*The installer verifies the release SHA-256 manifest and installs unprivileged binaries to `~/.local/bin`. When a systemd user manager is available, it also installs, enables, and starts `sunreactord.service`, then verifies that the daemon answers on IPC. Use `--no-service` to install without starting brightness automation.*

*From a clone, the installer also builds the Plasma widget when `plasmashell` and `kpackagetool6` are installed. `--no-plasma` skips it, `--with-plasma` tries anyway, and `--dry-run` prints the widget plan and changes nothing. See [Plasma widget](#-plasma-widget).*

### Building from Source (Optimized Native Profile)
```bash
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor

# Build with maximum CPU vectorization and locked dependencies
RUSTFLAGS="-C target-cpu=native" cargo build --release --locked

# Install binaries to ~/.local/bin
install -m 755 target/release/sunreactord target/release/sunreactorctl ~/.local/bin/

# Enable user daemon
mkdir -p ~/.config/systemd/user
cp contrib/systemd/sunreactord.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```

---

## // CONFIGURATION SPECIFICATION

The daemon configuration is stored in standard TOML format at `~/.config/sunreactor/config.toml`:

```toml
[daemon]
tick_seconds = 60
dry_run = false
desktop_idle_sync = false
apply_reassert_minutes = 2
ddc_timeout_seconds = 4
probe_seconds = 15

[location]
city = "Istanbul, TR"
latitude = 41.01384
longitude = 28.94966
timezone = "Europe/Istanbul"

[solar_policy]
twilight_elevation_start = -6.0
day_elevation_full = 20.0
use_adaptive_zenith = true
max_step_pct_per_tick = 6
min_write_delta_pct = 1

[[monitors]]
logical_id = "desk-primary"
backend = "ddc"
enabled = true
min_pct = 5
max_pct = 60
gain = 1.0
transition_gamma = 0.65
connector = "card1-DP-1"

[[monitors]]
logical_id = "laptop-internal"
backend = "backlight"
enabled = true
min_pct = 2
max_pct = 100
gain = 1.1
transition_gamma = 0.5
sysfs_path = "/sys/class/backlight/amdgpu_bl1"

[weather]
enabled = true
provider = "openweather"
api_key_env = "OPENWEATHER_API_KEY"
refresh_minutes = 30
min_multiplier = 0.75

[tui]
fps = 60
theme = "amber"        # any id or alias from `sunreactorctl theme list`
effects = "instrument"
show_logo = true
```

### Themes

Themes are TOML files shared by the TUI and the Plasma widget. 40 are built in. To add your own, put a file in `~/.config/sunreactor/themes/` (the file name must match `id`) and set `tui.theme`. The format is explained in [docs/THEMES.md](docs/THEMES.md).

```toml
# ~/.config/sunreactor/themes/pocket_night.toml
id = "pocket_night"
name = "Pocket Night"
aliases = ["gbp_night"]
extends = "handheld"      # inherit everything, override only what changes
family = "pixel"          # pixel | instrument | terminal (widget defaults)

[palette]                 # bg fg accent secondary_accent border_active border_inactive
bg = "#0b120a"            # text_muted success warning error, all "#rrggbb"
accent = "#c23b5f"

[tui]
glyph_style = "pixel"     # rounded | pixel

[widget]                  # optional; family defaults fill every missing key
glow = 0.25
modules = ["telemetry", "curve", "displays", "automation"]
```

```bash
sunreactorctl theme show handheld            # resolved palette and widget tokens
sunreactorctl theme export dracula > ~/.config/sunreactor/themes/mine.toml
sunreactorctl theme validate                 # check every user theme file
sunreactorctl theme validate ./mine.toml     # check one file (extends / aliases included)
```

---

## // PLASMA WIDGET

A Plasma 6 panel widget with a popup: day curve, a slider per display, automation mode and weather. It follows the terminal UI's theme or a pinned one. It is a QML package with a small C++ plugin that talks to the daemon's socket, and it does no brightness logic of its own.

<div align="center">
  <img src="docs/images/widget-themes.png" alt="Plasma widget, brightness page: macOS, PS3, Nothing, Synthwave 84, Hacker Green, Windows XP" width="95%" />
  <br/>
  <img src="docs/images/widget-weather.png" alt="Plasma widget, weather page: macOS, Wii, Casio Digital" width="95%" />
</div>

Top: brightness page in macOS, PS3, Nothing, Synthwave 84, Hacker Green and Windows XP. Bottom: weather page in macOS, Wii and Casio Digital. The full theme list is in [THEMES.md](docs/THEMES.md).

### Requirements
- Plasma 6 with KF6 and Qt 6, CMake 3.22+, `extra-cmake-modules`, a C++20 compiler
- `sunreactord` from this version (older daemons work, but without themes and the day curve)

### Install
From a clone, `./install.sh` builds and installs the widget (`--no-plasma` skips it). Then restart plasmashell:

```bash
systemctl --user restart plasma-plasmashell.service
```

Manual install and file locations are in [docs/PLASMA.md](docs/PLASMA.md).

### Settings
Theme (follow or pin), panel mode (sky, readout, sparkline, ring), weather icon style, and which popup sections to show. Defaults work without changes.

### Troubleshooting
Wrong popup size, QML import errors and an old daemon are covered in [docs/PLASMA.md](docs/PLASMA.md).

---

## // DETAILS & LICENSING

- **Documentation:** [docs/README.md](docs/README.md) indexes every document in the repository, including [THEMES.md](docs/THEMES.md).
- **Author:** [arcanorca](https://github.com/arcanorca)
- **License:** GPL-3.0-or-later
- **Built with:** Rust, Ratatui, DRM sysfs, DDC/CI (`ddcutil`), Qt 6 QML and C++ for the Plasma widget
