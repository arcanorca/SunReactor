<div align="center">
  <img src="docs/images/banner.svg" alt="SunReactor Logo" width="95%">
  <br/>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/Built_with-Ratatui-000?logo=ratatui&logoColor=fff&labelColor=201a16&color=ffd970" alt="Built with Ratatui" /></a>
  <img src="https://img.shields.io/badge/Language-Rust_1.80+-orange?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Platform-Linux_|_Windows-blue" alt="Platform" />
  <img src="https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg" alt="License" />
</div>

<br/>

**SunReactor** sets display brightness from the sun's elevation at your location. It runs as an unprivileged user service written in Rust and controls external monitors over DDC/CI and laptop panels through backlight devices. Weather is an optional correction, limited to 25% dimming. There is a terminal UI, a CLI, and a KDE Plasma 6 widget.

<div align="center">
  <img src="docs/images/SunReactor_2.png" alt="Terminal UI: automation and light cycle" width="48%" />
  <img src="docs/images/SunReactor_3.png" alt="Terminal UI: location and daylight globe" width="48%" />
</div>

## Features

- Brightness follows the sun, with min/max and an easing curve per display.
- Optional weather correction (at most 25% dimmer).
- Manual override and pause, both expire on their own.
- A monitor that sleeps or fails is skipped; the others keep working.
- Terminal UI and a Plasma widget, with 40 colour themes you can extend.

## Install

```bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash
```

This puts `sunreactord` and `sunreactorctl` in `~/.local/bin` and enables the user service. To build from source, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

External monitors need `ddcutil` 1.4.1+ and access to `/dev/i2c-*`. Laptop panels use sysfs or `brightnessctl`.

## Use

```bash
sunreactorctl discover             # find monitors and print a config snippet
sunreactorctl status               # daemon state, solar angles, per-display targets
sunreactorctl tui                  # terminal UI
sunreactorctl set desk 75          # manual brightness for one monitor
sunreactorctl suspend --minutes 90 # pause for 90 minutes
sunreactorctl resume               # back to the solar curve
sunreactorctl theme set nord       # change the theme
```

## Configure

`~/.config/sunreactor/config.toml`, created with `sunreactorctl config init`:

```toml
[location]
latitude = 41.01384
longitude = 28.94966
timezone = "Europe/Istanbul"

[[monitors]]
logical_id = "desk-primary"
backend = "ddc"               # or "backlight"
min_pct = 5
max_pct = 60
connector = "card1-DP-1"

[weather]
enabled = true
api_key_env = "OPENWEATHER_API_KEY"

[tui]
theme = "amber"               # any id from `sunreactorctl theme list`
```

All options are in [examples/config.toml](examples/config.toml). Themes are TOML files: [docs/THEMES.md](docs/THEMES.md).

## Plasma widget

<div align="center">
  <img src="docs/images/widget-themes.png" alt="Plasma widget, brightness page: macOS, PS3, Nothing, Synthwave 84, Hacker Green, Windows XP" width="85%" />
  <br/>
  <img src="docs/images/widget-weather.png" alt="Plasma widget, weather page: macOS, Wii, Casio Digital" width="85%" />
</div>

A Plasma 6 panel widget with a popup: day curve, a slider per display, automation mode and weather. It follows the terminal UI's theme or a pinned one.

`./install.sh` installs it from a clone; then restart plasmashell. More in [docs/PLASMA.md](docs/PLASMA.md).

## More

[docs/](docs/README.md) · [CHANGELOG](CHANGELOG.md) · GPL-3.0-or-later · by [arcanorca](https://github.com/arcanorca)
