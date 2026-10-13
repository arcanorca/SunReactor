# SunReactor Plasma widget — theme engine and instrument redesign

Status: contract for the `feat/theme-engine` branch. Supersedes the "wardrobe"
approach (10 hand-coded skins behind `isHandheld`-style branches).
`UX_PLAN.md` still holds the Plasma-convention rules (HIG, no product name in
the popup, unknown stays unknown, no scroll-wheel brightness, i18n in QML).

## 0. Why

- The daemon is a careful instrument; the widget showed sliders and a weather
  icon. The policy (solar curve, weather dimming, per-monitor limits, what
  happens next) was invisible.
- The TUI has 35 themes (`src/tui/theme.rs`, hardcoded Rust). The widget
  collapsed them to 10 archetypes by fragile alias matching and ignored half
  of each palette. Users could not add a theme without recompiling.
- Pixel art blurred: the desktop runs at 1.3x / 1.35x and every pixel element
  was a fractional-position `Rectangle` / non-integer font size / smoothed SVG.

## 1. Principles

1. **One theme source of truth, owned by the daemon crate**, shared by TUI and
   widget. Themes are data (TOML), not code.
2. **The widget never reimplements policy.** Curves, milestones and theme
   resolution come from the daemon over IPC.
3. **Follow by default, override locally.** Widget follows the TUI theme;
   a plasmoid instance may pin another theme or override single tokens. The
   widget never writes the daemon's config.
4. **Pixel art is a system.** Integer device-pixel grid, nearest-neighbour,
   bitmap fonts rendered 1:1 — verified by automated screenshots.
5. **Every theme gets a good result for free** (family default); hand-built
   skins exist only for flagships.
6. **Honest data first, decoration second.** The popup is an instrument.

## 2. Theme file format (daemon side)

`~/.config/sunreactor/themes/<id>.toml` shadows a built-in of the same id.
Built-ins are embedded (`include_str!`) from `src/theme/builtin/*.toml`.

```toml
id      = "handheld"            # == file stem, [a-z0-9_]+ ; value of tui.theme
name    = "Handheld"
aliases = ["gameboy", "dmg"]    # legacy names; "workbench" -> amiga etc.
family  = "pixel"               # pixel | instrument | terminal

[palette]                        # all required, "#rrggbb"
bg = "#0f380f"  fg = "#9bbc0f"  accent = "#8b1d42"  secondary_accent = "#306230"
border_active = "#9bbc0f"  border_inactive = "#306230"  text_muted = "#8bac0f"
success = "#9bbc0f"  warning = "#8bac0f"  error = "#8b1d42"

[tui]                            # optional
glyph_style = "pixel"           # rounded | pixel

[widget]                         # optional; every key optional, family defaults fill gaps
renderer   = "pixel"            # pixel | vector | terminal
font       = "bitmap8"          # bitmap8 | bitmap_vga | mono | sans | display
pixel_scale = 0                 # 0 = auto from DPR; 1..6 = preferred art-pixel size in logical px
shades     = ["#0f380f","#306230","#8bac0f","#9bbc0f"]   # dark->light, 2..16; derived if absent (see below)
density    = "compact"          # compact | comfortable
border     = "line"             # none | line | bevel | inset | double
radius     = 0                  # 0..16 logical px (ignored by pixel renderer)
dither     = "bayer4"           # none | bayer2 | bayer4
glow       = 0.0                # 0..1 phosphor/tube bloom
modules    = ["telemetry","curve","displays","automation","weather","forecast","suntimes"]
```

Family defaults (resolved by the daemon, so TUI-less consumers get them too):

| family | renderer | font | border | radius | dither | glow | density |
|---|---|---|---|---|---|---|---|
| pixel | pixel | bitmap8 | line | 0 | bayer4 | 0 | compact |
| instrument | vector | sans / mono | line | 4 | none | 0 | comfortable |
| terminal | terminal | mono | line (box-drawing) | 0 | none | 0 | compact |

As implemented: `instrument` defaults to `font = "sans"`; `pixel_scale` defaults
to 0 and `modules` to all seven in the order above. `shades` is always
resolved for every family (so a renderer override still has a ramp): when
absent it is `[bg, border_inactive, text_muted, fg]` sorted by luminance.
Validation: `pixel_scale` 0..=6, `radius` 0..=16, `glow` 0.0..=1.0, modules
unique; unknown keys anywhere are errors. Name/alias matching is
case-insensitive and treats `-`/space as `_` (and `vfdhifi` == `vfd_hifi`).

Classification of the 35 built-ins: **pixel** = handheld, commodore64, amiga,
classicmacintosh, casiodigital, nothing, cyberpunk, synthwave84, hackergreen;
**instrument** = braun, vfd_hifi, nixie, oscilloscope, thinkpad,
unixworkstation, phosphorblue, amber, grayscale; **terminal** = all editor
palettes (dracula, nord, gruvbox, catppuccinmocha, tokyonight, onedark,
ayudark, ayumirage, solarizeddark, everforest, kanagawa, zenburn, monokai,
nightowl, materialocean, rosepine, terminal). Built-in palettes must be
byte-identical to today's `Theme::palette()` values (a test enforces it).

User themes may set `extends = "dracula"` and override only what they change.

Config: `tui.theme` becomes a string validated against the registry
(built-ins + user dir + aliases). Old values keep working; unknown value is a
validation error that lists nearest matches. `Default` stays `amber`.

Theme problems are cosmetic and never stop automatic brightness:
- Daemon start / `reload_config`: an invalid or unreadable user theme file is
  logged (warning, `theme_file_skipped`) and skipped; all valid themes and
  built-ins still load. If `tui.theme` cannot be used as-is (unknown id, or
  its user file is broken) the daemon keeps running on the built-in with the
  same id if one exists, else on `amber`, and reports why in
  `status.theme_warning` (§3).
- Strict reporting lives in the tools: `sunreactorctl config validate`
  (unusable `tui.theme` → error, exit non-zero; unrelated broken files →
  listed as warnings), `sunreactorctl theme validate` (any broken file →
  exit non-zero) and `doctor` (`SR-THEME-UNRESOLVED` error → exit non-zero,
  `SR-THEME-FILE-INVALID` warning per file). All name the file and reason.

## 3. IPC additions (backward compatible)

Existing: one JSON request, one JSON response per connection, tag `request` /
`response`. Additions use `#[serde(default)]` so older plugins/daemons still
parse; a plugin talking to an older daemon must degrade (see §5).

`status.theme` (extend `ThemeStatus`, keep legacy flat fields; the new keys
are omitted — not `null` — by an older daemon, and old clients ignore them):

```json
"theme": {
  "name": "Handheld", "accent": "#8b1d42", "secondary_accent": "#306230",
  "bg": "#0f380f", "fg": "#9bbc0f", "text_muted": "#8bac0f",
  "id": "handheld", "family": "pixel", "glyph_style": "pixel",
  "palette": { "bg":"", "fg":"", "accent":"", "secondary_accent":"",
               "border_active":"", "border_inactive":"", "text_muted":"",
               "success":"", "warning":"", "error":"" },
  "widget": { "renderer":"pixel", "font":"bitmap8", "pixel_scale":0,
              "shades":["","","",""], "density":"compact", "border":"line",
              "radius":0, "dither":"bayer4", "glow":0.0,
              "modules":["telemetry","curve","displays","automation","weather","forecast","suntimes"] }
}
```

New requests / responses:

| request | response |
|---|---|
| `{"request":"list_themes"}` | `{"response":"themes","themes":[{"id","name","family","bg","fg","accent","secondary_accent"}]}` |
| `{"request":"get_theme","id":"nord"}` | `{"response":"theme","theme":{ <same object as status.theme> }}` |
| `{"request":"day_curve","monitor_id":null}` | see below |

`day_curve` reuses the TUI's `build_policy_previews` / `build_monitor_milestones`
(moved to the library as `policy::preview`, shared with the TUI):

```json
{"response":"day_curve","day_curve":{
  "date":"2026-10-13","step_minutes":10,"generated_at_epoch_s":0,
  "weather_multiplier":0.88,
  "sun":{"dawn":0,"sunrise":0,"solar_noon":0,"sunset":0,"dusk":0},
  "monitors":[{"monitor_id":"mi-monitor","min_pct":1,"max_pct":50,
     "samples":[{"t":0,"pct":3,"pct_no_weather":3}],
     "milestones":[{"id":"rise_50","label":"Rise 50%","t":0,"pct":34}]}]}}
```

`t` is Unix seconds. Samples cover local midnight to the next local midnight,
both included (145 samples; 139 / 151 on DST days). `pct_no_weather`
lets the widget shade what the weather is costing; `pct` applies the current
`weather_multiplier` (`null` when weather is inactive, then `pct ==
pct_no_weather`). `sun.*` may be `null` at polar edges (structured, no
invented values).

Implementation notes (deviations from the first draft of this contract):
- `milestones[].id` was added (stable `rise_start` … `night_floor`) so QML can
  translate labels; `label` stays the TUI's English text.
- Milestones are the schedule of the cycle containing today's solar noon (so a
  request at 03:00 still gets today's milestones). If the schedule cannot be
  built (polar day/night) `milestones` is `[]` and an extra
  `"milestones_error": "<reason>"` key is present; samples stay valid.
- Errors: unknown `monitor_id` → `{"response":"error","code":"invalid_request"}`;
  a nonexistent/ambiguous local midnight → `internal_error` with the solar
  error text. `get_theme` accepts ids and aliases; unknown → `invalid_request`
  with nearest-match suggestions in `message`.
- The daemon caches the last answer keyed by (local date, location, policy
  config, selected monitors, weather multiplier); `generated_at_epoch_s` is
  the time it was computed.
- `PROTOCOL_VERSION` stays 1: all additions are new request/response tags or
  optional keys.

`status.theme_warning` (optional string, omitted when there is nothing to
report): set when the configured `tui.theme` could not be used as-is and
`status.theme` is a fallback, e.g. `"theme file …/mine.toml is invalid: …;
using the default `amber` theme instead"`. Clients should show it next to
the theme chip; it never means brightness automation is affected.

Saved-only propagation: the TUI theme modal previews locally; only a saved
theme reaches status.

`sunreactorctl theme list|show <id>|set <id>|validate [file]|export <id>`.
`theme set` edits `tui.theme` and reloads, like the TUI does.

## 4. Widget architecture

```
package/contents/ui/
  engine/
    ThemeEngine.qml      resolves follow/pinned + overrides -> Theme object
    Theme.qml            typed tokens: palette, family, renderer, fonts, metrics
    PixelGrid.qml        singleton: dpr, cell size, snap helpers
    PixelSurface.qml     renders children at art resolution, integer nearest scale
    BitmapText.qml       bitmap font text at exact 1:1 sizes
    Dither.qml / shaders ordered-dither ramp, palette quantiser
    Frame.qml            border styles: line / bevel / inset / double / box-drawing
  modules/               one file per layout module, themable via Theme only
    Telemetry.qml  LightCycle.qml  Displays.qml  Automation.qml
    Weather.qml  Forecast.qml  SunTimes.qml
  skins/                 only where a family default is not enough (flagships)
    pixel/  instrument/  terminal/
  FullRepresentation.qml  composes modules in theme.modules order
  CompactRepresentation.qml  panel modes (see §6)
  config/                 settings pages
plugin/ (C++)             SunReactorClient exposes: theme (QVariantMap),
                          themeList, dayCurve, themeMode handled in QML
```

### Pixel renderer rules (the part that failed before)

1. `dpr = Screen.devicePixelRatio`. One art pixel = `k` **device** pixels,
   `k = max(2, round(pixel_scale_or_auto * dpr))`, i.e. an integer. Its logical
   size is `k / dpr`; every pixel-theme geometry is a multiple of it.
2. Pixel content is drawn inside a `PixelSurface`: a `layer.enabled` item of
   `W_art x H_art` art pixels, `layer.textureSize` fixed, `layer.smooth:false`,
   `layer.mipmap:false`, displayed at `W_art*k/dpr` logical px. Text inside is
   rendered 1:1 at the font's native grid (bitmap8 → 8/16/24 art px), so
   fractional DPR never reaches the glyph rasteriser. `Text.NativeRendering`,
   no font smoothing, hinting full.
3. Sliders/tracks are N discrete cells, never `width * progress`.
4. No SVG for pixel art; baked PNG atlases generated from indexed masks
   (extend `generate_weather_icons.py` — add palette-index output and recolour
   by the theme's `shades`). `Image.Nearest` semantics via
   `layer`/`smooth:false`.
5. **Conformance test:** headless screenshots of pixel themes at DPR 1.0,
   1.35, 2.0 must contain only colours from the theme's palette/shades (dither
   included). Any intermediate colour fails the test.

### Follow / pin / override model (per plasmoid instance)

```
theme file (built-in < user dir)  ->  [daemon resolves]  ->  status.theme
    + plasmoid: themeMode = follow | pinned, pinnedTheme = id   (get_theme)
    + plasmoid overrides: accent, density, renderer, pixelScale, modules
    = ThemeEngine.theme
```
Pinned theme is fetched with `get_theme`. If the daemon is older (no
`get_theme`), the widget derives a theme from the flat legacy fields + the
legacy-name archetype table (kept as fallback only).

## 5. Degradation

Older daemon (no `palette`/`widget`/`day_curve`): theme from legacy fields with
family guessed from the name; LightCycle module hidden. Offline: PlaceholderMessage
as today. Missing font: fall back to `mono`/`sans`, never to a wrong-sized
bitmap font (prefer vector fallback over blur).

## 6. Experience (popup and panel)

Popup, top to bottom (order and presence come from `modules`):

1. **Telemetry** — the TUI's status line: `● LIVE  MI 5% · LEN 9% · 15°C · 06:30`.
   Dot colour = daemon state (live/paused/manual/offline) via `success/warning/error`.
2. **Light cycle (hero)** — today's policy curve for the selected display
   (tabs when ≥2 displays, plus "All"). Now marker, applied level, shaded area
   between `pct_no_weather` and `pct` = what the weather costs, sunrise/noon/
   sunset ticks, min/max band. Hover scrubs: time → target %. It does **not**
   set brightness (accident-proof).
3. **Displays** — per display: name, applied %, target %, override state;
   slider = override (with "until HH:MM"); min/max limit handles; keyboard
   nav; unknown stays "—".
4. **Automation** — mode, reason, and **what happens next** ("08:54 · Rise
   50% → 34%"), way back to automatic.
5. **Weather / Forecast / Sun times** — compact, only when present.
6. Footer — Pause (durations), Update now, theme chip: shows the active
   theme, a lock when pinned; click opens a swatch list (follow / pin).

Panel (`CompactRepresentation`) modes, user-selectable: **sky sprite**
(family-styled), **readout** (brightness % in the theme's font / segment
digits), **sparkline** (mini day curve with now dot), **ring** (arc gauge).
Plus optional temperature. Vertical and horizontal panels both supported.

Settings: follow/pin with swatch picker and live preview, token overrides,
module toggles and order, density, pixel scale, panel mode, "Open themes
folder", validation message for bad user themes.

## 7. Verification

- Rust: unit tests for registry, alias compat, user theme load/extends,
  invalid files, palette parity with the legacy table, IPC round-trips and
  old-client compatibility; `cargo fmt/clippy/test` per `Makefile`.
- Plugin: `ctest` with a stand-in server speaking the new responses and the
  old ones.
- Visual: `tools/shot.sh` — fake daemon + `plasmoidviewer` under Xvfb/offscreen,
  matrix = {35 themes} x {DPR 1.0, 1.35, 2.0} x {popup, panel h/v}. Pixel
  conformance check (§4.5). Contact sheet PNG for review.
- No windows on the user's desktop; always headless.
