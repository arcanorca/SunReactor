# SunReactor TUI — Interaction and Visual Grammar

## Header

Under the wordmark, one line of practical state only: the daemon badge
(`Live`, `Suspended`, `Idle dimmed`, `Offline`), each monitor's applied
brightness under a short name (`Mi 38% · Lenovo 64%`, `—` before the first
write), the current temperature when weather is enabled, and the clock.
Segments that do not fit are dropped.

## Visual grammar

Every workspace uses the same small vocabulary (`src/tui/ui/kit.rs`):

| Symbol / treatment | Meaning |
|--------------------|---------|
| Rounded panel      | One functional region. The focused region's border and title use the accent colour. |
| `❯`                | Keyboard cursor (focused row, field, or list item). |
| `›`                | Selected item in a list that does not currently have focus. |
| `‹ value ›`        | Focused value that `←` / `→` adjust. |
| Accent capsule     | Focused editable value. Editing switches to a second capsule colour with `│ … │` edges and the terminal cursor. |
| Muted label + value | Read-only information; never focusable. |
| `●` `○` `▲`        | Available / current · inactive or unavailable · needs attention. |
| `⎿`                | Detail for the row above. |

Colour always reinforces a glyph or border change; it is never the only signal.

## Keyboard rules

- Arrows follow the layout: rows that run left to right use `←` `→`, lists
  that run top to bottom use `↑` `↓`.
- `↑` from the first element of a workspace moves to the tab bar; `↓`,
  `Enter`, or `Esc` on the tab bar move back into the workspace's first region
  (on Monitors, always the monitor list).
- `Esc` always steps out one level.

## Global

| Key                                  | Action                                  |
|--------------------------------------|-----------------------------------------|
| `Tab` / `Shift+Tab`                  | Next / previous workspace               |
| `Ctrl+Tab` / `Ctrl+Shift+Tab`        | Same, in terminals that report them     |
| `Ctrl+PgDn` / `Ctrl+PgUp`            | Same, in every terminal                 |
| `1`–`5`                              | Jump to a workspace                     |
| `←` `→` on the tab bar               | Move between workspaces                 |
| `?`                                  | Help for the focused context            |
| `q`                                  | Quit (pending changes are saved first)  |

SunReactor enables the kitty keyboard protocol when the terminal supports it,
which is what lets `Ctrl+Tab` differ from `Tab`. Other terminals send the
same code for both, so `Shift+Tab` and `Ctrl+PgUp/PgDn` are always available.

## Monitors (1)

The list on the left owns selection; the right pane shows the selected monitor.

| Focus  | Key            | Action |
|--------|----------------|--------|
| List   | `↑↓` / `jk`    | Select monitor (`↑` on the first goes to the tab bar) |
| List   | `→` / `Enter`  | Edit the brightness range |
| List   | `a`            | Open Automation for this monitor |
| List   | `s` / `r`      | Suspend / resume display writes |
| Range  | `↑↓` / `jk`    | Choose Minimum or Maximum |
| Range  | `←→`           | Adjust by 1% |
| Range  | `Enter`        | Enter an exact value |
| Range  | `Esc`          | Back to the list |

On narrow terminals the list becomes a one-line `‹ name ›` switcher above the
detail pane; there `←→` select the monitor and `↓` enters the controls.

The range track is one absolute 0–100 instrument: the band between the two
handles is the allowed range and `●` marks the applied brightness when the
daemon has confirmed it. `Applied` shows `Unknown` rather than `0%` when no
write has been confirmed. `Target` is the value the shared policy engine
computes now (after weather), and appears only when that computation exists.

## Automation (2)

Three columns, following one reading order: what is being controlled (left),
what it does today (middle), and when it changes (right).

- **Left:** the monitor field; **Solar curvature → Gamma** on a logarithmic
  track (1.0 in the middle); and **Output**, the brightness the monitor is at,
  in a three-row digit font. Output shows the value the daemon last wrote;
  before the first write it shows the computed target, muted, as "not
  written yet". A caption says whether brightness is rising, falling, or
  steady over the next quarter hour. With Full effects a new value counts up
  or down from the previous one and briefly glows.
- **Middle — Today's light cycle:** an area chart of the brightness target on
  the absolute 0–100 % scale. The area uses eighth-cell block tops, so the
  silhouette stays smooth, and shades from bright to deep with height. The
  vertical line and `●` mark now; `◆` marks the milestone selected in the
  schedule. Under the chart come sunrise, solar noon (with the peak target),
  and sunset; a band coloured by the sun's height (night → twilight → day)
  with phase names and `▲ Now`; and, when there is room, the location and day
  length.
- **Right:** the schedule table with the monitor's brightness **Range** as a
  reference line (it is edited on Monitors), and a small **Next event** frame
  with the next milestone and the time until it.

On shorter terminals, spacing and the location/day-length strip give way first,
so the chart stays readable.

| Focus      | Key          | Action |
|------------|--------------|--------|
| Monitor    | `←→`         | Switch monitor |
| Monitor    | `↓` / `Enter`| Go to Solar curvature |
| Curvature  | `←→`         | Gamma ±0.05 |
| Curvature  | `Enter`      | Exact gamma |
| Curvature  | `↓` / `↑`    | Schedule / monitor field |
| Schedule   | `↑↓` / `jk`  | Select milestone (`↑` on the first returns to Solar curvature) |
| Schedule   | `←→`         | Shift the milestone by 1 minute |
| Schedule   | `r`          | Reset the milestone to its solar time |

`[` and `]` also switch monitor from any Automation focus.

**Gamma** is the configuration field `transition_gamma`: the exponent applied
to the solar daylight factor before it is mapped into the monitor's range,
`target = min + factor^gamma × (max − min)`. The factor rises in the morning
and falls in the evening, so gamma shapes both ends of the day the same way.
Values below 1 lift the curve: brightness rises sooner and stays up longer
into the evening. Values above 1 hold brightness low for longer on both
sides. It is unrelated to display colour gamma.

## Location (3)

| Key          | Action |
|--------------|--------|
| `↑↓` / `jk`  | Select field |
| `Enter`      | Edit; on City, search and accept a match (city, coordinates, and timezone change together) |
| `Esc`        | Cancel the edit and restore the previous values |

The Earth panel is an orthographic globe centred on the configured location.
Land comes from a 0.5° mask rasterised from Natural Earth 1:110m land polygons
(public domain). The day/night boundary uses the same solar position model as
the policy engine; `✻` marks the point where the sun is overhead.

## Weather (4)

Read-only. `r` refreshes (or retries after a failure) when the daemon reports
weather. Weather credentials live in Settings. Every value appears in one
place only.

- **Current conditions:** the pixel-art scene, the condition, the temperature
  in a 5×7 dot-matrix font (one Braille dot per font dot, evenly spaced like an
  LED display) coloured from cool blue to warm coral, then feels
  like, humidity, cloud cover, wind (km/h and compass point), pressure, and
  visibility, and the freshness line at the bottom. The panel title shows the
  time the forecast sample is for.
- **24 hour forecast:** a smooth temperature area on a tidy degree scale, with
  a dashed `Now` line. Under the hours, each 3-hour sample has a 7×2 cell
  pixel icon (condition, day or night), its temperature, and its chance of
  precipitation. Sunrise and sunset are not repeated here.
- **SunReactor impact:** a small sun icon with an arrow, one headline
  ("Lower solar input", "Full solar input", or "Weather not in use"), and one
  short sentence.
- **Additional conditions:** air quality (US EPA index from PM2.5), dew point,
  precipitation chance now, wind chill when it applies (≤ 10 °C and wind over
  4.8 km/h), and the moon phase at night.
- **Bottom strip:** sunrise, solar noon, sunset, daylight, location with
  coordinates, and the data source.

Readings come from OpenWeather's 3-hour forecast (`/data/2.5/forecast`), which
already carries feels like, humidity, pressure, wind, visibility, and the
chance of precipitation; the daemon also asks the Air Pollution API once per
weather refresh with the same key. A failed air-quality request only leaves
that row "Unavailable" and never fails the weather refresh. UV index is not
shown because it needs a paid One Call subscription.

Narrower terminals keep conditions and the forecast; the right column needs
about 110 columns and the bottom strip 90 columns and 26 rows.

The scenes are composed on 32×16 pixel canvases and shown through a 26×14
window (26×7 cells) drawn with half blocks, so every cell holds two
full-colour pixels. Clouds are hand-toned sprites (highlight, light, mid,
shadow, deep shadow) lit from the upper left with a soft underside and no hard
outline; the sun and moon are shaded discs, and scenes float without a ground
strip. Every pixel has a role (sun, moon, cloud, shade, rain, snow, fog) and
leans 30 % toward the theme colour for that role, so
the art keeps its natural palette while matching each theme; temperature
colours and the forecast icons follow the same rule. With Full effects the
sky moves once a second; Reduced and Off keep it still.

## Themes

Twenty-eight themes. Four follow real product identities:

| Theme | Source colours |
|-------|----------------|
| Classic Macintosh | Mac OS Platinum gray with black ink; rainbow Apple logo (#61BB46 #FDB827 #F5821F #E03A3E #963D97 #009DDC), darkened for text roles |
| Casio Digital | Reflective LCD panel and segments; Casio logo blue #003296 and gold lettering |
| Nothing | Black, white, and Nothing red #D71921 |
| ThinkPad | Chassis black, TrackPoint red (Lenovo red #E42022), IBM blue |

The two light themes (Classic Macintosh, Casio Digital) keep text roles at a
readable contrast on their light backgrounds.

## Effects

`Effects` has three levels (the config values `instrument`, `reduced`, `off`
are unchanged):

- **Full** — masthead power-on sweep, sliding tab indicator, globe rotation to
  the location with an arrival pulse, curve morph after a recompute, output
  value count, animated weather sky, temperature chart rising when a newer
  snapshot arrives, spinner and shimmer while a daemon
  request is pending, live heartbeat.
- **Reduced** — local confirmations only (range handle settle, milestone
  settle, live heartbeat, static spinner glyph).
- **Off** — no motion.

Animations are wall-clock based, bounded, and redraw at the animation rate only
while they run; idle screens redraw once per second.
