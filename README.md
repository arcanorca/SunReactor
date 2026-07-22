<div align="center">
  <img src="docs/images/banner.svg" alt="SunReactor Logo" width="95%">
  <br/>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/Built_with-Ratatui-000?logo=ratatui&logoColor=fff&labelColor=201a16&color=ffd970" alt="Built with Ratatui" /></a>
</div>


**SunReactor** is a lightweight, headless Rust daemon designed to automate monitor hardware brightness. By calculating solar elevation based on your exact geolocation and time, it generates a brightness curve that adapts to seasonal daylight shifts. Combined with real-time cloudiness data from the OpenWeather API and customizable limits via a dedicated TUI, the daemon manages your displays in the background.

## // PREVIEW

<div align="center">
  <img src="docs/images/SunReactor_1.png" alt="Dashboard" width="48%" style="margin: 0.5%; border-radius: 8px;" />
  <img src="docs/images/SunReactor_2.png" alt="Monitor Focus" width="48%" style="margin: 0.5%; border-radius: 8px;" />
  <br/>
  <img src="docs/images/SunReactor_3.png" alt="Theme Menu" width="31%" style="margin: 0.5%; border-radius: 8px;" />
  <img src="docs/images/SunReactor_4.png" alt="Weather Chart" width="31%" style="margin: 0.5%; border-radius: 8px;" />
  <img src="docs/images/SunReactor_5.png" alt="Settings" width="31%" style="margin: 0.5%; border-radius: 8px;" />
</div>
<br/>

## // THE AUTOMATION

A fixed clock schedule (like dimming the screen exactly at 8:00 PM) falls out of sync because daylight hours shift between seasons in many regions.  SunReactor uses the sun's elevation above the horizon to calculate brightness change.

```text
        ☀ (Solar Noon) --> Max Brightness
       /  \
     /      \ (Smoothly dimming via gamma curve)
   /          \
- 0° (Horizon) ------------------------------
                \
                  \ ☾ (Night) --> Min Brightness
```

> ### 🧮 The Math
> **1. Smoothstep:** `t = (Elevation - NightFloor) / (DayPeak - NightFloor)`  
> **2. Gamma Curve:** `curve = t^γ`  
> **3. Projection:** `Brightness = Min% + (Max% - Min%) * curve * weather_multiplier`

- **Local by Default:** SunReactor does all the daylight math locally. Basically, it generates an adaptive brightness curve based on your selected city’s sunrise/sunset times for the current date. Since that is deterministic, it can work completely offline, with the sole exception of the optional weather integration.

- **Multi-Monitor Support:** 50% brightness on an IPS panel looks different than 50% on a VA or OLED. You can set distinct minimum, maximum, gamma curvature, and gain values for each display. The daemon calculates each monitor's brightness independently.

## // ARCHITECTURE & CONSTRAINTS

SunReactor is built to be predictable and stay out of the way:

- **Hardware Control:** Adjusts the actual backlight via `ddcutil` (external) and `sysfs` / `brightnessctl` (internal).
- **Synchronous:** No async runtime. It executes a simple synchronous loop: wake, compute, write to hardware, sleep..
- **Unprivileged:** Runs as a systemd user service. No root access or dbus required.
- **Idle Sync:** Includes its own automatic screen dimming feature by integrating directly with Wayland/X11 idle protocols. This allows you to turn off native DE power management to prevent conflicting brightness states, ensuring displays wake up directly to the latest solar calculation rather than a cached value.
- **Optional Weather:** If you provide a free OpenWeather API key, the daemon reads cloud cover and slightly dims your displays on overcast days. This acts only as a multiplier over the base calculation. Additionally, the TUI provides a view of current weather conditions.

## // Installation

### Prerequisites

SunReactor relies on the following tools being installed on your system to control hardware brightness:

- **For External Monitors (DDC/CI):** Ensure `ddcutil` is installed.
  - Arch: `sudo pacman -S ddcutil`
  - Fedora: `sudo dnf install ddcutil`
  - Ubuntu/Debian: `sudo apt install ddcutil`
- **For Laptop Panels:** Ensure `brightnessctl` is installed.
  - Arch: `sudo pacman -S brightnessctl`
  - Fedora: `sudo dnf install brightnessctl`
  - Ubuntu/Debian: `sudo apt install brightnessctl`

> [!NOTE]
> Do not add every user to `i2c` or `video` by default. Run `sunreactorctl doctor`
> after installation. It tests actual device access and distinguishes active
> access, stale login sessions, and permission failures.

---

### Option A: Automated Installer

The installer downloads the checksum-verified x86-64 GNU/Linux artifact built on
Ubuntu 22.04, smoke-tests it before replacing anything, and verifies the user unit,
config, daemon startup, doctor result, and IPC. It never installs Rust or changes
system groups. Failed upgrades restore the previous binaries and unit.

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash
```

The installer writes to `~/.local/bin` and does not require `sudo`. If no compatible
artifact exists, it leaves the installation unchanged and prints separate source-build
instructions.

<details>
<summary><b>View Manual Installation Steps</b></summary>

1. Download the latest x86-64 GNU/Linux archive and matching `.sha256` from
   [Releases](https://github.com/arcanorca/SunReactor/releases), then verify it:

```bash
sha256sum -c sunreactor-VERSION-linux-x86_64-gnu.tar.gz.sha256
tar xzf sunreactor-VERSION-linux-x86_64-gnu.tar.gz
./sunreactorctl --version
./sunreactord --help
```

2. Move the binaries to your local PATH:
```bash
mkdir -p ~/.local/bin
install -m 755 sunreactord sunreactorctl ~/.local/bin/
```

3. Render both service paths from the same install directory and start the daemon:
```bash
mkdir -p ~/.config/systemd/user
sed "s|@BINDIR@|$HOME/.local/bin|g" sunreactord.service > ~/.config/systemd/user/sunreactord.service
systemd-analyze --user verify ~/.config/systemd/user/sunreactord.service
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```
</details>

### Option B: Build from Source

If you have Rust installed, you can build from source:

```bash
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor

# Build and install explicitly to ~/.cargo/bin
cargo build --release --locked
install -Dm755 target/release/sunreactord ~/.cargo/bin/sunreactord
install -Dm755 target/release/sunreactorctl ~/.cargo/bin/sunreactorctl

# Render the unit for this distinct source-build path
mkdir -p ~/.config/systemd/user
sed "s|@BINDIR@|$HOME/.cargo/bin|g" contrib/systemd/sunreactord.service > ~/.config/systemd/user/sunreactord.service
systemd-analyze --user verify ~/.config/systemd/user/sunreactord.service
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```

---

## Uninstallation

This removes the binaries and user unit while preserving configuration and state:

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash -s -- --uninstall
```

## // HOW TO USE

After installation, add every viable monitor with one safe, idempotent command:

```bash
sunreactorctl discover --apply
```

It validates the updated configuration, avoids duplicate monitor entries, reloads
the daemon, and rolls back if verification fails. Running it again is a no-op for
monitors that are already configured.

Then open SunReactor:

```bash
sunreactorctl
```

No subcommand is required—the TUI is the normal interface. Use **Tab** or the arrow
keys to move between pages, **Enter** to edit or toggle a setting, and **q** to save
and quit. Go to **Location**, search for your city, and select it; changes are
saved and reloaded automatically. The **Monitors** and **Limits** pages let you tune
each display, while **Weather** and **Settings** contain optional features.

Smooth hardware fades are disabled by default because some external DDC/CI monitors
flicker or lose signal when sent rapid commands. You can opt in later from
**Settings → Smooth Transitions** if your monitor handles them reliably.

If the TUI reports a hardware or daemon problem, run:

```bash
sunreactorctl doctor
```

The remaining CLI commands are optional and useful for scripting or quick overrides:
```bash
sunreactorctl status               # View current solar state and monitor levels
sunreactorctl suspend --minutes 60 # Temporarily pause automation
sunreactorctl set desk 50          # Manually override a specific monitor
sunreactorctl clear-override       # Resume automatic solar policy
```

## // UNDER THE HOOD

The TUI writes your settings to a standard TOML file at `~/.config/sunreactor/config.toml`. Here is an example:

```toml
[location]
city = "Istanbul"
timezone = "Europe/Istanbul"

[daemon]
# Safe default for DDC/CI monitors: one direct brightness command per change.
# Set true only if your display reliably handles rapid multi-step fades.
smooth_transition = false

[[monitors]]
logical_id = "desk"
backend = "ddc"
min_pct = 20
max_pct = 90
gain = 1.0

[[monitors]]
logical_id = "laptop"
backend = "backlight"
min_pct = 5
max_pct = 100
gain = 1.2
sysfs_path = "/sys/class/backlight/amdgpu_bl1"

[weather]
enabled = true
provider = "openweather"
api_key_env = "OPENWEATHER_API_KEY"
```

## // DETAILS

- **Developer:** arcanorca
- **License:** GPL-3.0-or-later
- **Stack:** Rust | ratatui | systemd (user) | Unix IPC | ddcutil | brightnessctl
