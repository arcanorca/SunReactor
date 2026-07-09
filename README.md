<div align="center">
  <pre style="background: transparent; border: none; font-weight: bold; line-height: 1.2;">
<span style="color: #FFCA28">            ✧ · ⋆ . ˚ ·  .  *  .            </span>
<span style="color: #FFB300">         *   \  |  /   *   · ⋆  .       </span>
<span style="color: #FF8F00">       .  -   \ | /   -  .      .      *</span>
<span style="color: #D84315">          _____                ____                  __           </span>
<span style="color: #9B111E">      *  / ___/__  ______     / __ \___  ____ ______/ /_____  _____ .</span>
<span style="color: #C21F24">         \__ \/ / / / __ \   / /_/ / _ \/ __ `/ ___/ __/ __ \/ ___/ *</span>
<span style="color: #E13B29">     .  ___/ / /_/ / / / /  / _, _/  __/ /_/ / /__/ /_/ /_/ / /     .</span>
<span style="color: #F8791D">       /____/\__,_/_/ /_/  /_/ |_|\___/\__,_/\___/\__/\____/_/   *</span>
<span style="color: #FFB300">       .  -   / | \   -  .      *      .</span>
<span style="color: #FFCA28">         *   /  |  \   *   .  ⋆ .   *   .</span>
<span style="color: #FFE082">            .  *  .  . *     ˚ · . .  *  .</span>
  </pre>

**Automated Monitor Brightness, Synced with the Sun.**

`☀ -> 🖥 -> 💡`
</div>

---

**SunReactor** is a headless Rust daemon that automates your monitor brightness. By calculating the sun's exact elevation for your city and the current date, the brightness curve adapts to seasonal daylight shifts and dynamically dims based on real-time cloud cover. You set your hardware limits via the built-in terminal UI, and the daemon orchestrates your displays in the background.

## // PREVIEW

<div align="center">
  <img src="docs/images/SunReactor_1.png" alt="Dashboard" width="48%" style="margin: 0.5%;" />
  <img src="docs/images/SunReactor_2.png" alt="Monitor Focus" width="48%" style="margin: 0.5%;" />
  <br/>
  <img src="docs/images/SunReactor_3.png" alt="Theme Menu" width="31%" style="margin: 0.5%;" />
  <img src="docs/images/SunReactor_4.png" alt="Weather Chart" width="31%" style="margin: 0.5%;" />
  <img src="docs/images/SunReactor_5.png" alt="Settings" width="31%" style="margin: 0.5%;" />
</div>
<br/>

## // THE AUTOMATION

Static brightness schedules fail because daylight shifts with the seasons. SunReactor uses the sun's physical elevation above the horizon to drive a fully automated pipeline.

```text
        ☀ (Solar Noon) ──▶ Max Brightness
       /  \
     /      \ (Smoothly dimming via gamma curve)
   /          \
─ 0° (Horizon) ──────────────────────────────
                \
                  \ ☾ (Night) ──▶ Min Brightness
```

- **Offline City Database:** SunReactor includes a built-in, offline database of thousands of cities. Select your city in the TUI, and the daemon calculates the solar math entirely locally.
- **Multi-Monitor Orchestration:** 50% brightness on a VA panel does not look the same as 50% on an OLED or IPS screen. You can set distinct minimum, maximum, and gain values for each display independently. The daemon calculates the global solar progression and maps it to each monitor's unique hardware curve.

## // ARCHITECTURE & CONSTRAINTS

SunReactor operates with strict boundaries to remain predictable and lightweight:

- **Hardware Level Control:** Adjusts actual hardware backlight via `ddcutil` (external displays) and `sysfs` / `brightnessctl` (internal screens). No artificial color filters.
- **Deterministic Math Core:** The policy engine calculates solar math offline. Zero network calls, zero state mutations, zero subprocesses.
- **Synchronous & Lightweight:** Wakes up, computes the math, writes to the hardware, and sleeps. No heavy async runtime.
- **Unprivileged Execution:** Runs as an isolated systemd user service. No root access or dbus integration is required.
- **Dynamic Weather Modifier:** You can add a free OpenWeather API key. The system reads real-time cloud cover and dims your displays on overcast days, acting strictly as a multiplier over the base solar logic.

## // INSTALLATION

**Dependencies:** `ddcutil` (for external displays) and `brightnessctl` (for laptops).

```bash
sudo pacman -S ddcutil brightnessctl

# Clone and build from source
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor
cargo install --path .
```

Generate the initial state and discover your connected monitors:
```bash
sunreactorctl config init
sunreactorctl discover
```

Start the daemon:
```bash
mkdir -p ~/.config/systemd/user
cp contrib/systemd/sunreactord.service ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```
*(Note: If installed via Cargo, ensure the `ExecStart` path in the unit file points to `%h/.cargo/bin/sunreactord`)*

## // INTERFACE & CONTROL

Configuration and monitoring are handled entirely through the built-in terminal interface (`ratatui`). It functions as a thin client over a secure local IPC socket.

```bash
sunreactorctl tui
```
The TUI provides 24 themes, real-time monitoring, dynamic weather charts, and full control over your monitor limits and city selection.

For scripting or quick overrides, the CLI provides direct commands:
```bash
sunreactorctl status               # View current solar state and monitor levels
sunreactorctl suspend --minutes 60 # Temporarily pause automation
sunreactorctl set desk 50          # Manually override a specific monitor
sunreactorctl clear-override       # Resume automatic solar policy
```

## // UNDER THE HOOD

While you configure everything via the TUI, the resulting state is cleanly saved in `~/.config/sunreactor/config.toml`. Example structure:

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
