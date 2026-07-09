<div align="center">

```text
   _____             _____                 _             
  / ____|           |  __ \               | |            
 | (___  _   _ _ __ | |__) |___  __ _  ___| |_ ___  _ __ 
  \___ \| | | | '_ \|  _  // _ \/ _` |/ __| __/ _ \| '__|
  ____) | |_| | | | | | \ \  __/ (_| | (__| || (_) | |   
 |_____/ \__,_|_| |_|_|  \_\___|\__,_|\___|\__\___/|_|   
                                                         
```
**Adaptive Hardware Brightness Driven by Pure Solar Mathematics.**

`☀ → 🖥 → 💡`

</div>

---

**SunReactor** isn't another software gamma filter that tints your screen red. It is a headless, zero-root Rust daemon that uses pure astronomical trigonometry to physically adjust your monitor's backlight voltage based on the actual sun's angle hitting your window.

No `dbus` bloat. No async runtime overhead. Just mechanical sympathy and circadian awareness.

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

## // THE AHA! MOMENT

Most brightness tools (and OS night lights) use arbitrary clock schedules ("dim at 22:00"). But the world isn't static. Clock schedules drift with seasons, daylight saving time, and geographical latitude.

SunReactor ditches the clock entirely. Instead, it computes **Solar Elevation**:
```text
dawn → sunrise → solar noon → sunset → dusk
 ↑                                        ↑
 brightness ramps up              brightness ramps down
```
When the sun physically sets at your exact GPS coordinates, your backlight physically dims. 

## // WHY IT'S DIFFERENT (THE ANTI-FEATURES)

We hate bloated daemons as much as you do. SunReactor is built on a philosophy of extreme minimalism paired with a ridiculously luxurious TUI.

* **Hardware-First:** We don't tint pixels. We command the actual hardware via DDC/CI (external monitors) and `sysfs` (laptop panels) to save power and preserve contrast.
* **Offline Pure Math:** The core policy engine is a pure mathematical function. Zero network requests, zero state mutations, zero subprocesses. It just works.
* **No `Tokio` / Async Bloat:** It's a daemon that wakes up every 60 seconds, does some trig, writes to an `i2c` bus, and goes back to sleep. A synchronous loop is deterministic and takes 0 idle CPU cycles.
* **No `dbus` or `root`:** Everything runs as an isolated systemd user service. Control happens via a secure (`0600`) local Unix socket using JSON payloads.
* **Weather as a Bounded Modifier:** Optional cloud cover data (via OpenWeather) dynamically dims the screen on overcast days, but *never* overrides the base solar policy.

## // INSTALLATION (Arch / CachyOS)

**Dependencies:** `ddcutil` (for external displays) and `brightnessctl` (for laptops).

```bash
sudo pacman -S ddcutil brightnessctl

# Clone and install
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor
cargo install --path .
```

Generate a starter config and probe your hardware:
```bash
sunreactorctl config init
sunreactorctl discover
```

Copy the discovered monitors into your config (`~/.config/sunreactor/config.toml`), set your coordinates, and enable the daemon:
```bash
mkdir -p ~/.config/systemd/user
cp contrib/systemd/sunreactord.service ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```
*(Note: If installed via Cargo, edit `ExecStart` in the unit file to point to `%h/.cargo/bin/sunreactord`)*

## // CLI & TUI CONTROL

SunReactor ships with a built-in `ratatui` terminal interface. It's a thin client over the local IPC socket.

```bash
# Launch the mesmerizing interface:
sunreactorctl tui
```
**Features:** 24 built-in themes (Catppuccin, Gruvbox, Tokyo Night, etc.), dynamic weather charts, and live daemon controls.

Prefer scripts? The CLI handles everything over the Unix socket:
```bash
sunreactorctl status              # View daemon state
sunreactorctl suspend --minutes 30 # Pause all hardware writes
sunreactorctl set desk 50         # Manual brightness override
sunreactorctl reload-config       # Hot-reload config.toml
```

## // CONFIGURATION

Everything lives in `~/.config/sunreactor/config.toml`:

```toml
[location]
latitude = 41.0
longitude = 29.0
timezone = "Europe/Istanbul"

[[monitors]]
logical_id = "desk"
backend = "ddc"
min_pct = 20
max_pct = 90

[[monitors]]
logical_id = "laptop"
backend = "backlight"
min_pct = 5
max_pct = 100
sysfs_path = "/sys/class/backlight/amdgpu_bl1"
```

## // CREDITS & LICENSE

- **Developer:** arcanorca
- **License:** GPL-3.0-or-later
- **Stack:** Rust | ratatui | systemd (user) | Unix IPC | ddcutil | brightnessctl
