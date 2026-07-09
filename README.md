# SunReactor

<div align="center">
  <br/> <code>☀ → 🖥 → 💡</code>
  <br/> <br/>
</div>

A headless daemon that adjusts monitor brightness based on the sun's actual position in the sky. Runs as a systemd user service, talks DDC/CI to external monitors and sysfs/brightnessctl to laptop panels. No root, no D-Bus, no async runtime.

Built in Rust. ~19k lines across 67 source files.

## // HOW IT WORKS

Most brightness tools use fixed clock schedules ("dim at 22:00"). That drifts with seasons, latitude, and DST. SunReactor uses **solar elevation** — the sun's angle above the horizon at your coordinates — as the primary signal.

```
dawn → sunrise → solar noon → sunset → dusk
 ↑                                        ↑
 brightness ramps up              brightness ramps down
```

The daemon wakes up every N seconds, computes the sun's current elevation, maps it to a brightness percentage through a per-monitor min/max/gain curve, and writes the result to hardware. Optional weather data (cloud cover via OpenWeather) acts as a bounded multiplier on top.

### The Pipeline

```
solar elevation → daylight factor → gamma curve → per-monitor clamp → weather modifier → hardware write
```

- **Solar module** computes sunrise, sunset, dawn, dusk, solar noon, and elevation for any lat/lon/timezone.
- **Policy engine** is pure: no I/O, no process spawning, no state. Takes elevation in, brightness targets out.
- **Apply engine** dispatches to DDC (`ddcutil`) or backlight (`brightnessctl` / sysfs) backends.
- **Weather** is optional and bounded. Cloud cover can dim output but never overrides solar policy.

## // TUI

SunReactor includes a built-in terminal interface for live monitoring and control:

```bash
sunreactorctl tui
```

- Live per-monitor brightness, solar timeline, and weather data
- Manual brightness overrides and daemon suspend/resume
- 24h temperature trend chart with responsive layout (auto-switches horizontal/vertical based on terminal width)
- Persistent config editing (location, timezone, API keys, per-monitor limits)
- 24 built-in themes: Amber, Terminal, Dracula, Gruvbox, Rosé Pine, Catppuccin Mocha, Nord, Tokyo Night, One Dark, Ayu Dark, Ayu Mirage, Solarized Dark, Everforest, Kanagawa, Zenburn, Monokai, Night Owl, Material Ocean, Cyberpunk, Synthwave '84, Hacker Green, Phosphor Blue, Commodore 64, Grayscale

The TUI is a thin client over the same IPC socket and config file. It does not replace `sunreactorctl` or `config.toml`.

## // INSTALLATION

**Dependencies**: `ddcutil` (external monitors) and/or `brightnessctl` (laptop panels)

### From Source

```bash
git clone https://github.com/arcanorca/SunReactor.git
cd SunReactor
cargo install --path .
```

### Setup

```bash
# Install hardware helpers (Arch/CachyOS)
sudo pacman -S ddcutil brightnessctl

# Generate starter config
sunreactorctl config init

# Discover your monitors
sunreactorctl discover

# Copy the printed config snippet into ~/.config/sunreactor/config.toml
# then set your latitude, longitude, and timezone
```

### systemd User Service

```bash
mkdir -p ~/.config/systemd/user
cp contrib/systemd/sunreactord.service ~/.config/systemd/user/

# If installed via cargo (not /usr/bin), edit ExecStart:
# ExecStart=%h/.cargo/bin/sunreactord

systemctl --user daemon-reload
systemctl --user enable --now sunreactord.service
```

## // CONFIG

Config lives at `~/.config/sunreactor/config.toml`. See [`examples/config.toml`](examples/config.toml) for the full template.

```toml
[location]
latitude = 41.0
longitude = 29.0
timezone = "Europe/Istanbul"

[solar_policy]
twilight_elevation_start = -6.0
day_elevation_full = 20.0
max_step_pct_per_tick = 6

[[monitors]]
logical_id = "desk"
backend = "ddc"
enabled = true
min_pct = 20
max_pct = 90
gain = 1.0
connector = "DP-1"

[[monitors]]
logical_id = "laptop"
backend = "backlight"
enabled = true
min_pct = 10
max_pct = 100
gain = 1.0
sysfs_path = "/sys/class/backlight/amdgpu_bl1"

[weather]
enabled = false
provider = "openweather"
api_key_env = "OPENWEATHER_API_KEY"
```

## // CLI

```bash
sunreactorctl status              # daemon state + per-monitor brightness
sunreactorctl discover             # probe hardware, print config snippets
sunreactorctl discover --json      # machine-readable discovery
sunreactorctl suspend              # pause all writes indefinitely
sunreactorctl suspend --minutes 30 # pause for 30 min
sunreactorctl resume               # resume + force reapply
sunreactorctl set desk 50          # manual override for "desk" monitor
sunreactorctl set --global 70      # override all monitors
sunreactorctl clear-override       # clear overrides
sunreactorctl reload-config        # hot-reload config.toml
sunreactorctl tui                  # interactive terminal UI
```

**Precedence**: `suspend` > manual override > weather modifier > solar policy.

## // ARCHITECTURE

```
src/
├── bin/
│   ├── sunreactord.rs        # daemon entrypoint
│   └── sunreactorctl.rs      # CLI entrypoint
├── solar/                    # deterministic sun position math
├── policy/                   # pure brightness curve engine
├── discovery/                # hardware probing (ddcutil, brightnessctl, sysfs)
├── backends/                 # DDC and backlight write drivers
├── apply/                    # policy → hardware dispatch
├── runtime/                  # daemon event loop, idle/wake sync
├── config/                   # TOML schema, validation, migration
├── ipc/                      # Unix socket protocol (JSON, v1)
├── state/                    # runtime state persistence
├── weather/                  # optional OpenWeather integration
├── tui/                      # ratatui interactive interface
│   └── ui/                   # view rendering (monitors, weather, settings)
├── paths.rs                  # XDG path contract
└── lib.rs                    # shared library root
```

Design constraints:
- No async runtime
- No root daemon
- No D-Bus
- No busy polling
- Solar math is deterministic and side-effect free
- Policy engine is pure (no I/O, no spawning)
- Weather is bounded and optional
- All subprocesses use explicit `Command` args, no shell invocation
- IPC is local-only Unix socket, `0700`/`0600` permissions

## // TROUBLESHOOTING

<details>
<summary><b>I2C access for DDC/CI</b></summary>

```bash
sudo modprobe i2c-dev
echo i2c-dev | sudo tee /etc/modules-load.d/i2c-dev.conf
sudo usermod -aG i2c $USER
# Log out and back in
```
</details>

<details>
<summary><b>KDE PowerDevil conflict</b></summary>

If brightness flickers or reverts, disable automatic brightness in PowerDevil settings. Both services will fight for backlight control.
</details>

<details>
<summary><b>DDC bus number changes across reboots</b></summary>

Use `serial` and `model` selectors instead of `ddc_bus` in your config. Bus numbers are not stable.
</details>

<details>
<summary><b>Stale socket</b></summary>

```bash
systemctl --user restart sunreactord.service
```
The daemon auto-removes stale sockets when no listener is active.
</details>

## // DEVELOPMENT

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Or with Make:

```bash
make fmt && make clippy && make test
```

## // CREDITS

- **Developer:** arcanorca
- **License:** MIT
- **Stack:** Rust | ratatui | chrono | serde | toml | systemd (user) | ddcutil | brightnessctl
