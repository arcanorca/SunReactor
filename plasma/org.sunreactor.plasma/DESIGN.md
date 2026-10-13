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
the theme button; it never means brightness automation is affected.

Saved-only propagation: the TUI theme modal previews locally; only a saved
theme reaches status.

`sunreactorctl theme list|show <id>|set <id>|validate [file]|export <id>`.
`theme set` edits `tui.theme` and reloads, like the TUI does.

## 4. Widget architecture

As built on `feat/theme-engine` (foundation; skins and the final redesign
build on it):

```
package/contents/ui/
  engine/                  the only place tokens and pixel rules live
    ThemeEngine.qml        follow | pinned + local overrides -> `theme`
    Theme.qml              typed tokens: palette, shades, renderer, font roles,
                           metrics, `grid` (PixelGrid), helpers (mix, alpha,
                           onColor, stateColor)
    PixelSurface.qml       art-resolution layer, integer nearest upscale
    BitmapText.qml         bitmap font 1:1 (CPU raster), vector-mono fallback
    Dither.qml             ordered dither over the shade ramp (palette quantiser)
    Frame.qml              none / line / bevel / inset / double, pixel or vector
  modules/                 one file per layout module (contract below)
    Modules.js             id -> file registry; ModuleSettings.qml
    Telemetry.qml  LightCycle.qml  Displays.qml  Automation.qml
    Weather.qml  Forecast.qml  SunTimes.qml
  components/              rows shared by modules: DisplayItem, ThemeSlider,
                           WeatherText; ThemeButton + ThemePicker (tab row);
                           PopupFooter (Update now)
  skins/                   pixel/ (this branch), instrument/, terminal/
  FullRepresentation.qml   Repeater + Loader over theme.modules
  CompactRepresentation.qml  sprite + temperature (panel modes still to come)
  config/ConfigGeneral.qml   "Look" section: follow/pin + swatch list, accent,
                           density, pixel size
plugin/ (C++, module org.sunreactor.plasma)
  SunReactorClient         theme, themeList, pinnedThemeId/pinnedTheme/
                           pinnedThemeError, fetchTheme(id) -> themeFetched /
                           themeFetchFailed(id, unsupported|not_found|offline),
                           dayCurve, refreshThemeList(), refreshDayCurve(),
                           themeRegistry / dayCurveSupport (Capability enum)
  themenormalize.*         JSON -> normalised theme map (camelCase), family
                           defaults, legacy exact-name table (old daemons only)
  PixelGrid                window-effective DPR, deviceScale k, unit, snap(),
                           cells(), deviceRect(), harness flag
  PixelText / PixelDither  CPU-rasterised art-resolution content, nearest node
```

Normalised theme map (what `client.theme`, `client.pinnedTheme` and
`themeFetched` carry; Theme.qml consumes it): `{ id, name, family, glyphStyle,
legacy, palette{bg, fg, accent, secondaryAccent, borderActive, borderInactive,
textMuted, success, warning, error}, widget{renderer, font, pixelScale, shades,
density, border, radius, dither, glow, modules} }`. Colours are lower-case
`#rrggbb`; every omitted or invalid widget key holds the family default;
`shades` is empty unless the daemon sent 2..8 valid colours (Theme.qml then
derives them: {bg, borderInactive, textMuted, fg} sorted by luminance, so every
shade is a palette colour). No usable bg/fg -> empty map -> Plasma colours.

Day curve refresh policy (client side): on popup open, local date change,
reload_config / set_monitor_limits / refresh_weather / run_once acks, a change
of the followed theme id, and every 10 min - never per status poll. A rejected
(`invalid_request`) `day_curve` / `list_themes` / `get_theme` marks the
capability unsupported until the connection drops or the status shape shows a
newer daemon; nothing else in the client state changes.

### Module contract

Every file in `modules/` declares `required property Theme theme`,
`required property SunReactorClient client`, `required property ModuleSettings
settings` and `readonly property bool available` (false hides it), takes its
width from the popup and reports implicit height. Modules read colours, fonts
and metrics from `theme` only - never `Plasmoid.configuration` or Kirigami
colours directly - so a skin can replace one file. Module ids are those of
`widget.modules`: telemetry, curve (LightCycle.qml), displays, automation,
weather, forecast, suntimes.

### Pixel renderer rules (the part that failed before)

Measured on Qt 6.12, RHI/OpenGL (Mesa llvmpipe under Xvfb), QT_SCALE_FACTOR
1.0 / 1.25 / 1.35 / 1.5 / 2.0 with `tools/pixel_proof.sh`.

1. `dpr` is the **window's effective** device pixel ratio (`PixelGrid`,
   updated on `ItemDevicePixelRatioHasChanged`), not `Screen.devicePixelRatio`:
   on Wayland with fractional scaling QScreen reports the integer output
   scale. One art pixel = `k` device pixels,
   `k = max(2, round(pixel_scale * dpr))`, `pixel_scale` 0 = auto = 2
   (1.0 -> k 2, 1.25/1.35/1.5 -> k 3, 2.0 -> k 4). Logical size `unit = k/dpr`.
2. Pixel content is drawn inside a `PixelSurface`: the inner item is
   `W_art x H_art` logical units, `layer.textureSize` = exactly that,
   `layer.smooth: false`, `layer.mipmap: false`, and the layered item carries
   `scale: unit` (transform origin top-left). Rectangles at whole art
   coordinates, `Image { smooth: false }` at natural size, and the C++
   PixelText/PixelDither are exact inside it at every ratio tested; the
   surface's position may be fractional (the nearest upscale by an integer k
   keeps every art pixel a k x k block regardless of offset).
   **Deviation from the original plan: QML `Text` is not 1:1 inside the
   layer.** Text nodes are rasterised at the *window* DPR and then resampled
   into the art texture (measured: 9-16 % off-palette pixels at fractional
   ratios, `Text.NativeRendering` or not). `BitmapText` therefore rasterises
   on the CPU (`PixelText`: QPainter on a QImage at the font's native size,
   `QFont::NoAntialias`, full hinting, no kerning) and draws the image with
   nearest filtering - `artScale` 1 inside a surface, `unit` standalone.
   Native grids: Press Start 2P 8/16/24 px, unscii 16/32 px; VT323 and Sysfont
   are not grid fonts and are never used as bitmap fonts. Vector text in pixel
   themes uses mono (never a bitmap font at a non-native size).
   Not allowed inside a surface: QML Text, Canvas, antialiased Shapes,
   gradients, opacity < 1, radius, MultiEffect.
3. Sliders/tracks are N discrete cells, never `width * progress`.
4. No SVG for pixel art; baked PNG atlases generated from indexed masks
   (extend `generate_weather_icons.py` — add palette-index output and recolour
   by the theme's `shades`). `Image.Nearest` semantics via
   `layer`/`smooth:false`. Done for the pixel skins:
   `tools/generate_weather_icons.py` also writes `icons/pixel/weather16.png`
   and `ui8.png` whose pixels are tone indices (0 none, 1-4 highlight..shadow,
   5/6 warm, 7 water, 8 generated outline, 9 ray); `PixelSprite` maps index i
   to the i-th colour the skin passes (PixelStyle.spriteColors, all theme
   tokens). The vector skins keep the SVGs.
6. Pixel skins (`skins/pixel/`, registered under SKINS.pixel): `PixelStyle`
   picks the hardware idiom by theme id only (handheld=dmg, commodore64=c64,
   classicmacintosh=mac, casiodigital=casio, amiga, nothing=dots, everything
   else neon) and maps palette/shade tokens to roles (paper, ink, dim, ghost,
   hot, cost, frame). Shared parts: PixelPanel + PixelChrome.js (window
   chrome as art-pixel rects), PixelLabel (built-in `PixelGlyphs` 5x7 / 3x5
   faces, falls back to the theme bitmap font for uncovered characters),
   PixelDigits (block / LCD cells / dot matrix / seven-segment / bitmap 2x),
   PixelCellSlider (discrete cells, keys, a11y value interface, throttled
   commits), PixelButton (button / tab / tag). LightCycle uses `PixelCurve`
   (C++ CPU raster: stepped line, patterned area, weather-cost band, limits,
   night, sun/hour ticks, now/target/applied markers, optional dot lattice).
   Every surface is opaque (the chrome paints the desk colour first).
5. **Conformance test:** `tools/conformance.py` checks that pixel regions
   contain only palette + shade colours (exact, tolerance 0). PixelSurfaces
   report their device-pixel rect when `SUNREACTOR_HARNESS` is set, so both
   the proof surface and real popup screenshots (`tools/shot.sh` writes
   `*.regions.json`) are checked region by region. Result: handheld,
   commodore64 and nothing pass at all five ratios (surface, standalone text,
   standalone dither; and the Telemetry module inside the real popup at
   1.0/1.35/2.0); the negative control (QML Text in a surface) fails at every
   fractional ratio (it happens to pass at integer 2.0).

### Follow / pin / override model (per plasmoid instance)

```
theme file (built-in < user dir)  ->  [daemon resolves]  ->  status.theme
    + plasmoid: themeMode = follow | pinned, pinnedTheme = id   (get_theme)
    + plasmoid overrides: accentOverride, densityOverride, pixelScale
      (ThemeEngine also takes renderer and modules overrides; no config keys yet)
    = ThemeEngine.theme
```
Pinned theme is fetched with `get_theme` and re-fetched on popup open, config
reload and every 10 min. While it is pending, missing (`not_found`) or the
daemon is older (`unsupported`), the engine keeps following and exposes the
reason in `ThemeEngine.notice`; an older daemon only describes its current
theme, so another palette cannot be derived locally. The legacy name table
(exact match, 35 built-ins, in `themenormalize.cpp`) only supplies the family
of the *followed* theme for an older daemon.

## 5. Degradation

Older daemon (no `palette`/`widget`/`day_curve`): theme from the legacy flat
fields, family from the exact-name table (unknown -> instrument), missing
palette entries derived; LightCycle hidden; Automation shows no next
milestone (and stays hidden in automatic mode, as before); pinning falls back
to following with a note in settings. Offline: PlaceholderMessage as today;
the last theme stays (cosmetic), the day curve is cleared (data). Missing
bitmap font: BitmapText falls back to vector mono at the equivalent size,
never to a wrong-sized bitmap font. No theme at all: Plasma colours and fonts.

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
6. Footer — Update now. The theme button sits at the right end of the tab
   row: a 2x2 swatch of the active theme, a dot while following the terminal
   app, a lock when pinned, a warning when the daemon fell back; click opens
   the picker (Follow SunReactor first, then the themes by family).

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
- Plugin: `ctest` - `test_sunreactorclient` (stand-in QLocalServer: framing,
  unknown-stays-unknown, normaliser, legacy table, old-daemon degradation,
  day-curve parsing and refresh policy, theme list / get_theme / pinning) and
  `test_engine` (`tests/tst_engine.qml` via qmltestrunner: tokens, derived
  shades, overrides, font roles, Plasma fallback, grid arithmetic, pin notice).
- Lint: `/usr/lib/qt6/bin/qmllint -I build/qml <file>` (note `/usr/bin/qmllint`
  is the Qt 5 one); clean apart from the `i18nd*` unqualified notes.
- Visual (all headless; `TMPDIR` must give a short path for the socket):
  - `tools/shot.sh --out DIR [--tag T] [--themes ...] [--scales ...]
    [--views popup panel-h panel-v] [--old] [--scenario ...]` - fake daemon +
    plasmoidviewer under Xvfb with a private D-Bus and XDG dirs, a shadowed
    viewer shell (no SDK toolbar), `tools/harness/containment` for the popup
    and `org.kde.panel` for panels; writes PNG, QML log, request log and pixel
    regions per shot. `--package/--qml` point it at another tree (baselines).
  - `tools/pixel_proof.sh --out DIR` - engine-only proof surface, all pixel
    themes x DPR 1.0/1.25/1.35/1.5/2.0, conformance per region.
  - `tools/conformance.py`, `tools/contact_sheet.py`, `tools/fake_daemon.py`
    (`--old` answers the new requests with `invalid_request`).
  - Remaining: the full 35-theme matrix needs the daemon branch's theme files
    as fixtures (the fake daemon has 7).
- No windows on the user's desktop; always headless. Qt logs to the journal
  when stderr is not a TTY: the scripts set `QT_FORCE_STDERR_LOGGING=1`.
