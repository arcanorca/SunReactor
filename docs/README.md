# SunReactor documentation

Start with the [README](../README.md) for installation and the command line.
The documents below go deeper.

## For users

| Document | What it covers |
|---|---|
| [THEMES.md](THEMES.md) | Theme file format, user themes, `sunreactorctl theme`, and how the Plasma widget follows, pins and overrides a theme |
| [PLASMA.md](PLASMA.md) | Plasma widget: install, settings, popup size, troubleshooting |
| [systemd-user-unit.md](systemd-user-unit.md) | The user service template, how its paths are rendered, and why each directive is there |
| [DISTRIBUTION_COMPATIBILITY.md](DISTRIBUTION_COMPATIBILITY.md) | The release-qualified Linux build and installer contract |
| [RELIABILITY.md](RELIABILITY.md) | Reliability strategy: hardware failures stay per monitor and out of the policy |
| [TUI_UX_NOTES.md](TUI_UX_NOTES.md) | Interaction and visual conventions of the terminal UI |
| [WINDOWS_PORT.md](WINDOWS_PORT.md) | Windows architecture and port milestones |
| [testing/real-hardware.md](testing/real-hardware.md) | Regression evidence from real displays |

The Plasma widget has its own design record:
[`plasma/org.sunreactor.plasma/DESIGN.md`](../plasma/org.sunreactor.plasma/DESIGN.md)
(theme format, IPC use, widget architecture, verification) and
[`UX_PLAN.md`](../plasma/org.sunreactor.plasma/UX_PLAN.md) (the audit and rules for
Plasma conventions).

## For contributors

| Document | What it covers |
|---|---|
| [DEVELOPMENT.md](DEVELOPMENT.md) | Toolchain, local workflow, lint policy and CI parity |
| [`archive/`](archive/) | Historical phase records (`phase8-*`–`phase10-*`), owner briefs, and TUI refactor notes |
