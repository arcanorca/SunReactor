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
> **1. Smoothstep:** `t = (Elevation - NightFloor) / (DayPeak - NightFloor)`, then `s(t) = t²(3 - 2t)`
> **2. Response exponent:** `curve = s(t)^γ`; `transition_gamma` is a user-tunable easing/response exponent, not a calibrated perceptual model
> **3. Projection:** map the curve into each monitor's `[min_pct, max_pct]`, then apply monitor gain and the bounded weather multiplier with clamping

- **Local by Default:** SunReactor does all the daylight math locally. Basically, it generates an adaptive brightness curve based on your selected city’s sunrise/sunset times for the current date. Since that is deterministic, it can work completely offline, with the sole exception of the optional weather integration.

- **Multi-Monitor Support:** 50% brightness on an IPS panel looks different than 50% on a VA or OLED. You can set distinct minimum, maximum, gamma curvature, and gain values for each display. The daemon calculates each monitor's brightness independently.

## // ARCHITECTURE & CONSTRAINTS

SunReactor is built to be predictable and stay out of the way:

- **Hardware Control:** Adjusts the actual backlight via `ddcutil` (external) and `sysfs` / `brightnessctl` (internal).
- **Synchronous:** No async runtime. It executes a simple synchronous loop: wake, compute, write to hardware, sleep..
- **Unprivileged:** Runs as a systemd user service. No root access or dbus required.
- **Idle Sync:** Includes its own automatic screen dimming feature by integrating directly with Wayland/X11 idle protocols. This allows you to turn off native DE power management to prevent conflicting brightness states, ensuring displays wake up directly to the latest solar calculation rather than a cached value.
- **Optional Weather:** If you provide a free OpenWeather API key, the daemon reads cloud cover from the OpenWeather 5-day / 3-hour forecast endpoint and slightly dims your displays on overcast days. The first returned value is a forecast interval, not a current meteorological observation; it is retained as a bounded forecast-derived product heuristic and acts only as a multiplier over the base solar calculation. Additionally, the TUI provides forecast rows and charts from the remaining forecast intervals.

## // Installation

### Release artifacts and verification

Linux archives use explicit architecture and libc names: `x86_64-gnu`, `aarch64-gnu`, `x86_64-musl`, and `aarch64-musl`. The installer detects architecture and libc, downloads the matching archive plus `SHA256SUMS`, verifies SHA-256 before extraction, validates archive members, and rejects unknown libc environments. Checksums protect transfer integrity but do not independently prove provenance; release archives are additionally covered by GitHub artifact attestations. Optional verification:

```sh
gh attestation verify sunreactor-<version>-linux-x86_64-gnu.tar.gz -R arcanorca/SunReactor
```

Release builds use the pinned Rust toolchain, `Cargo.lock`, `--locked`, and native x86_64/aarch64 runners where available. GNU/musl claims are limited to tested targets and environments; `ddcutil` and `brightnessctl` remain optional backend-specific runtime dependencies.

### Supported Linux Distributions

SunReactor officially supports modern systemd-based x86_64 Linux distributions across Wayland and X11 desktop environments, including:
* **Arch Linux / CachyOS / EndeavourOS** (Rolling)
* **Ubuntu 26.04 LTS & 24.04 LTS** (and compatible derivatives such as Linux Mint 22.x, Pop!_OS)
* **Debian 13 (Trixie)** (Stable)
* **Fedora 44 & 43** (and Nobara Linux)
* **openSUSE Tumbleweed**

Prebuilt release binaries are compiled against a conservative `GLIBC_2.34` baseline and run on glibc >= 2.34 systems (including Ubuntu 22.04 LTS). For detailed distribution matrices, ddcutil version policies, and session qualification notes, see [Distribution Compatibility](docs/DISTRIBUTION_COMPATIBILITY.md).

### Prerequisites

SunReactor relies on standard userspace utilities to control hardware brightness:

- **For External Monitors (DDC/CI):** Ensure `ddcutil` is installed (minimum v1.4.1, recommended >= 2.2.0).
  - Arch: `sudo pacman -S ddcutil`
  - Fedora: `sudo dnf install ddcutil`
  - Ubuntu/Debian: `sudo apt install ddcutil`
  - openSUSE: `sudo zypper install ddcutil`
- **For Laptop Panels:** Ensure `brightnessctl` is installed (or rely on sysfs fallback).
  - Arch: `sudo pacman -S brightnessctl`
  - Fedora: `sudo dnf install brightnessctl`
  - Ubuntu/Debian: `sudo apt install brightnessctl`
  - openSUSE: `sudo zypper install brightnessctl`

> [!NOTE]
> On modern systemd distributions, the `ddcutil` package installs udev rules that grant the active desktop user access automatically via `uaccess`. If your user lacks access, add your user to the `i2c` group (`sudo usermod -aG i2c $USER`), ensure the `i2c-dev` module is loaded (`sudo modprobe i2c-dev`), and relogin.

---

### Option A: Automated Installer

The easiest way to install SunReactor is using our automated installation script. It downloads the latest pre-built binary, installs the user-local files, and uses a systemd user service when a usable user manager is available.

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash
```

*The installer places the executables securely in `~/.local/bin` and does **not** require `sudo`.*

The installer honors `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, and `XDG_CACHE_HOME` (all must be absolute paths). Before enabling or starting the service, it asks the running systemd user manager for `FragmentPath` and requires the result to equal the unit file installed by SunReactor; this prevents a higher-precedence same-name unit from being mistaken for the installed unit. When no usable systemd user manager is available, files are still installed and the installer prints the command for running `sunreactord` manually. Use `--no-service` to explicitly skip service setup. The installer does not enable systemd linger.

The installer itself requires Bash. Automatic service integration currently means systemd user services only; OpenRC, runit, and s6 are not installed or configured automatically.

<details>
<summary><b>View Manual Installation Steps</b></summary>

1. Download the latest pre-built binary from [Releases](https://github.com/arcanorca/SunReactor/releases). Make sure to check for the latest version tag (e.g., `v0.1.0`) and choose the correct architecture (`x86_64` or `aarch64`):

**For x86_64 (Intel/AMD):**
```bash
curl -LO https://github.com/arcanorca/SunReactor/releases/latest/download/sunreactor-v0.1.0-linux-x86_64.tar.gz
tar xzf sunreactor-v0.1.0-linux-x86_64.tar.gz
```

**For ARM64 (aarch64):**
```bash
curl -LO https://github.com/arcanorca/SunReactor/releases/latest/download/sunreactor-v0.1.0-linux-aarch64.tar.gz
tar xzf sunreactor-v0.1.0-linux-aarch64.tar.gz
```

2. Move the binaries to your local PATH:
```bash
mkdir -p ~/.local/bin
install -m 755 sunreactord sunreactorctl ~/.local/bin/
```

3. Start the daemon (systemd user manager required for this manual path):
```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
sed -e "s|@CONFIG_HOME@|${XDG_CONFIG_HOME:-$HOME/.config}|g" \
    -e "s|@STATE_HOME@|${XDG_STATE_HOME:-$HOME/.local/state}|g" \
    -e "s|@CACHE_HOME@|${XDG_CACHE_HOME:-$HOME/.cache}|g" \
    -e "s|@BIN_DIR@|$HOME/.local/bin|g" \
    contrib/systemd/sunreactord.service > "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/sunreactord.service"
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```
Without a systemd user manager, run `~/.local/bin/sunreactord` in the foreground or use a service manager configured separately.
</details>

### Option B: Build from Source

If you have Rust installed, you can build from source:

```bash
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor

# Install binaries to ~/.cargo/bin
cargo install --path .

# Start the systemd daemon
mkdir -p ~/.config/systemd/user
cp contrib/systemd/sunreactord.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```

---

## Uninstallation

To remove SunReactor and its background daemon from your system, run:

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash -s -- --uninstall
```

Uninstallation removes the user service file and installed binaries. User configuration (`~/.config/sunreactor`) and runtime state (`~/.local/state/sunreactor`) are preserved by default. To completely purge all configuration, state, and cache data, add `--purge`:

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash -s -- --uninstall --purge
```

## // QUICK START

**1. Initialize config and discover monitors:**

```bash
sunreactorctl config init
sunreactorctl discover
```

The `discover` command detects your connected monitors and prints config snippets you can paste into `~/.config/sunreactor/config.toml`.

**2. Set your location** (via the TUI or by editing the config file directly):

```bash
sunreactorctl tui
```

Navigate to the **Location** tab and search for your city, or enter coordinates manually. Without a location set, the daemon defaults to the equator (0°, 0°) which gives a generic 12h/12h day-night cycle.

**3. Start the daemon:**

```bash
mkdir -p ~/.config/systemd/user
cp sunreactord.service ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```

> **Note:** If installed via `cargo install`, ensure the `ExecStart` path in the unit file points to `~/.cargo/bin/sunreactord`. If installed from the release tarball to `~/.local/bin/`, update it to `~/.local/bin/sunreactord`.

## // INTERFACE & CONTROL

You can configure and monitor the daemon using the built-in terminal interface (`ratatui`). It connects to the daemon over a local IPC socket.

```bash
sunreactorctl tui
```

The TUI includes real-time monitoring, weather charts, theme options, and config management.

The CLI also provides direct commands for scripting or quick overrides:
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
