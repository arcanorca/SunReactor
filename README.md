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
> Make sure your user is in the `i2c` group for `ddcutil` to work without root: `sudo usermod -aG i2c $USER` (requires a reboot).

---

### Option A: Automated Installer

The easiest way to install SunReactor is using our automated installation script. It downloads the latest pre-built binary, sets up the systemd background daemon, and launches the dashboard automatically.

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash
```

*The installer places the executables securely in `~/.local/bin` and does **not** require `sudo`.*

<details>
<summary><b>View Manual Installation Steps</b></summary>

1. Download the latest pre-built binary from Releases:
```bash
curl -LO https://github.com/arcanorca/SunReactor/releases/latest/download/sunreactor-v0.1.0-linux-x86_64.tar.gz
tar xzf sunreactor-v0.1.0-linux-x86_64.tar.gz
```

2. Move the binaries to your local PATH:
```bash
mkdir -p ~/.local/bin
install -m 755 sunreactord sunreactorctl ~/.local/bin/
```

3. Start the daemon:
```bash
mkdir -p ~/.config/systemd/user
cp sunreactord.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```
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

To completely remove SunReactor and its background daemon from your system, simply run:

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash -s -- --uninstall
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
