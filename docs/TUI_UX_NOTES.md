# SunReactor TUI — Navigation & Keybindings

## Global

| Key             | Action                  |
|-----------------|-------------------------|
| `Tab` / `← →`  | Switch main tab         |
| `q`             | Quit TUI                |
| `?`             | Toggle help overlay     |

## Monitors Tab

| Key     | Action                            |
|---------|-----------------------------------|
| `↑ ↓`   | Select monitor                    |
| `a`     | Toggle automation advanced        |
| `+` `-` | Override brightness ±1% (fine)    |
| `]` `[` | Override brightness ±5% (coarse)  |
| `r`     | Reset selected monitor → auto     |
| `R`     | Reset ALL monitors → auto         |

When automation advanced is open for the selected monitor:

- `← →` selects the current milestone row
- `+ -` adjusts the selected milestone by `±1` minute
- `] [` adjusts the selected milestone by `±5` minutes
- `r` resets the selected milestone back to its base solar-derived time
- `s` saves the draft offsets to `config.toml` and reloads the daemon

All override and reset actions use **optimistic UI**: the displayed state
updates on the same frame as the keypress, before the daemon roundtrip
completes. The daemon state reconciles on the next background poll (~2 s).

## Settings Tabs (Automation / Location / Weather)

| Key     | Action                                |
|---------|---------------------------------------|
| `↑ ↓`   | Select input field                    |
| `Enter` | Start editing                         |
| `Esc`   | Stop editing                          |
| `s`     | Save config to disk & reload daemon   |

Editing any field sets the header indicator to **DRAFT**. Pressing `s`
writes the config, sends `ReloadConfig` over IPC, and clears the indicator
back to **LIVE**.

## Diagnostics Tab

Read-only view of the raw `StatusResponse` struct from the daemon.
