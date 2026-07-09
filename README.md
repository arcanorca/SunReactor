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
</div>

---

**SunReactor** isn't another software gamma filter that tints your screen red. It is a headless, zero-root Rust daemon that uses astronomical trigonometry to physically adjust your monitor's backlight voltage based on the true solar elevation at your exact coordinates.

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

Most brightness tools (and OS night lights) rely on arbitrary clock schedules ("dim at 22:00"). But clock schedules are fundamentally flawed—they drift with seasons, daylight saving time, and geographical latitude.

SunReactor ditches the clock. It continuously computes **Solar Elevation** (the physical angle of the sun relative to your horizon) and maps it to a highly tunable brightness curve:

```text
     [Astronomical State]            [Hardware Backlight]
      +90° (Solar Noon)  ──────────▶  100% (Customizable Max)
             ...                               ...
        0° (Horizon)     ──────────▶  Interpolated Curve (Gamma-Aware)
             ...                               ...
      -18° (Night/Dusk)  ──────────▶    5% (Customizable Min)
```

**Absolute Control & Fine-Tuning:**
You aren't locked into a rigid algorithm. SunReactor is designed for extreme fine-tuning per monitor:
* **Floor & Ceiling:** Set absolute `min_pct` and `max_pct` boundaries. The daemon will never blind you or turn the screen entirely black.
* **Curve Sensitivity (Gamma/Gain):** Adjust how aggressively the brightness ramps up or down as the sun moves. You control the mathematical curve, not just the endpoints.
* **Weather Modifier:** Optional cloud cover data (via OpenWeather) dynamically dims the screen on overcast days, but acts strictly as a bounded multiplier that never violates your minimum floor.

## // HOW IT'S BUILT

I hate bloated daemons, so SunReactor is designed to be as predictable and dumb as possible:

- **Real hardware dimming.** It doesn't slap a red filter over your pixels like f.lux. It uses `ddcutil` and `sysfs` to physically lower the backlight voltage.
- **Offline math.** The core logic doesn't need the internet. It just uses your GPS coordinates and the system clock to calculate solar angles.
- **Zero async bloat.** No `tokio`. The daemon wakes up every 60 seconds, does the math, writes to the `i2c` bus, and goes back to sleep. Zero idle CPU.
- **No root, no dbus.** Runs entirely as a systemd user service. Everything communicates over a simple local unix socket (`0600`).
- **Optional weather.** If you drop in an OpenWeather key, it dynamically dims the screen on cloudy days. But it acts strictly as a dumb multiplier on top of the base math.

## // CLI & TUI CONTROL

SunReactor ships with a built-in `ratatui` terminal interface acting as a thin client over the local IPC socket.

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

## // INSTALLATION & CONFIGURATION (Arch / CachyOS)

**Dependencies:** `ddcutil` (for external displays) and `brightnessctl` (for laptops).

```bash
sudo pacman -S ddcutil brightnessctl
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor
cargo install --path .
```

Generate a starter config, discover your monitors, and start the daemon:
```bash
sunreactorctl config init
sunreactorctl discover

# Enable the systemd user service
mkdir -p ~/.config/systemd/user
cp contrib/systemd/sunreactord.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```

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
# gain = 1.0 (Optional curve tuning)

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
