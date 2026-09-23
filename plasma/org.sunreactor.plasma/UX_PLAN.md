# Plasma widget: audit and redesign plan

Scope: `plasma/org.sunreactor.plasma` (Plasma 6 applet + QML/C++ plugin).
Reference points used for the audit:

- KDE HIG (text and labels, icons) — <https://develop.kde.org/hig/>
- Plasma widget docs (PlasmoidItem properties, representations, packaging)
- The in-tree Plasma components: `PlasmaExtras.Representation`,
  `PlasmoidHeading`, `DescriptiveLabel`, `PlaceholderMessage`
  (`/usr/lib/qt6/qml/org/kde/plasma/…`)
- KDE's own brightness applet (`powerdevil/applets/brightness`) as the closest
  first-party equivalent of this widget
- The installed Breeze icon theme (which names actually exist)

## 1. What is wrong today

### Branding and wording

| Where | Problem |
|---|---|
| `FullRepresentation.qml` | Renders "SunReactor" + "Solar Cockpit" as a title block. A Plasma popup does not repeat the widget name (KDE's brightness applet has no header at all), and the product name is not information. |
| `metadata.json` | `Name: "SunReactor Cockpit"`, description "cockpit". |
| `ConfigGeneral.qml` | Section "Cockpit Display", "SunReactor reactor logo". |
| `main.qml` | Contextual action "Launch Terminal Cockpit…", tooltip title "SunReactor". |
| Tooltip | Jargon: "Sun Elevation: +28.4° (Daylight)", "multiplier: 0.88×", "Global Brightness: 62% across 2 display(s)". None of it tells the user what the widget is doing. |

### Structure and Plasma conventions

- The popup is a hand-built `Item` + `ColumnLayout` with manual margins instead
  of `PlasmaExtras.Representation`, so it does not get Plasma's popup padding,
  scroll handling or footer behaviour.
- Every section is a `Kirigami.AbstractCard`. Cards are a Kirigami *app*
  idiom; Plasma popups use flat `PlasmaComponents3.ItemDelegate` rows. Five
  stacked cards inside a popup read as five boxes with borders.
- The header carries four `ToolButton`s (refresh weather / reload config /
  terminal / configure) duplicating `Plasmoid.contextualActions`.
  `PlasmaExtras.BasicPlasmoidHeading` already provides the actions menu and the
  configure button for free.
- Fixed popup size `gridUnit * 24 × 38` (≈432×684 px): far taller than the
  content needs with one or two monitors.
- No keyboard navigation at all (no `KeyNavigation`, no focus handling),
  where the reference applet wires every slider up and down.
- `Kirigami.Theme.neutralTextColor` is used as "muted grey" in six places. In
  Kirigami, *neutral* means warning (orange). Secondary text is
  `PlasmaExtras.DescriptiveLabel` (or `opacity: 0.75`).
- Arbitrary font scaling (`smallFont.pixelSize * 0.85`, `defaultFont * 1.75`)
  instead of the theme's font roles.
- Hand-composited colour badges (`Qt.rgba(themeColor…, 0.16)`) for the mode
  chip and the backoff chip.
- No offline state: when the daemon is down, the popup still draws sliders at a
  fabricated 50%. Plasma's answer is `PlasmaExtras.PlaceholderMessage`.

### Icons

- The applet icon is a 7 KB "tokamak reactor" with radial gradients and two
  Gaussian-blur filters. At 22 px it collapses into an orange blob; the dark
  filled disc fights every panel background. Breeze app icons are flat,
  geometric and legible at small sizes.
- The "symbolic" variant hard-codes `color: #dcdcdc`, so as a file URL it can
  not follow a light theme — it is a light-grey icon on a light panel.
- The panel shows a *full-colour pixel-art* weather SVG. HIG: 16 px and 22 px
  are symbolic, 32 px and up are full colour. The 24×24 pixel grid also scales
  to 22 px non-integrally, so it is visibly muddy.
- 13 bundled weather sprites (plus a 671-line generator) duplicate icons the
  Breeze theme already ships — including `-symbolic` and day/night variants —
  which recolour with the user's theme.
- `ActionFooter.qml` uses `weather-sunset` and `weather-sunset-up`, which do
  not exist in Breeze: two menu items render without an icon.
- Monitor rows use `video-display`, which only exists as a 64 px colour icon,
  drawn at 16 px.

### Data handling

- `last_applied_percent` is `Option<u8>` in the IPC contract; the plugin turns
  a missing value into `50`, i.e. it invents brightness and puts the slider
  somewhere the display never was.
- All user-visible strings live in C++ as `QStringLiteral` ("Daylight",
  "Manual Override", "New Moon", "2h 14m remaining", "21°C"): untranslatable,
  and locale-independent formatting for times, temperatures and durations.
- The widget reads only `condition_description` (provider English) and the
  `multiplier`, ignoring `day_phase`, `state` (why weather is unavailable) and
  the per-monitor override deadline.
- Moon phase and solar elevation in degrees: decoration in a brightness widget.

## 2. Decisions

1. **No product name anywhere in the UI.** No title block in the popup, no
   "cockpit" wording. The widget is named by function:
   `Name: "Adaptive Brightness"`, description "Screen brightness that follows
   the sun and the weather". "SunReactor" survives only in `Keywords` (so the
   widget is findable) and in the plugin id.
2. **Popup follows the brightness-applet pattern**: `PlasmaExtras.Representation`
   → `ScrollView` → `ColumnLayout` of `ItemDelegate` rows, plus a
   `PlasmoidHeading` footer for the two actions. No cards, no separators, no
   badges.
3. **Content = control + reason + schedule, no repetition:**
   - one slider per display; an *All displays* master row only when there are
     two or more;
   - one row saying what the automation is doing and, when the user has taken
     over, the button that gives control back;
   - one weather row (only when weather is on) that states the effect in
     words — "Clouds are dimming displays by 12%" — instead of "0.88×";
   - one sunrise / solar noon / sunset strip with the next event emphasised,
     which is the "what happens next" the old Milestone card failed to give;
   - no moon phase, no elevation in degrees, no backend chips, no
     "Connected Monitors (2)" heading.
4. **Unknown stays unknown.** A monitor with no applied value shows "—" and a
   disabled slider; global brightness shows "—" instead of a made-up 50%.
5. **Icons come from the theme.** Panel icon is symbolic and state-driven
   (sky icon → paused → manual → offline), all Breeze names verified to exist,
   `-symbolic` appended in the panel exactly like the brightness applet does.
   The 13 bundled sprites and the generator are removed.
6. **New logo.** Flat Breeze-style geometry, one idea: *the sun on the
   horizon* — the lower half below the line is muted, the upper half is lit,
   which is literally what the daemon reacts to. Colour version for the widget
   chooser (48 grid), monochrome `-symbolic` version (22 grid, `currentColor`)
   for anywhere a mask is wanted. No gradients, no blur filters.
7. **C++ carries data, QML carries language.** Every presentation string moves
   to QML with `i18nd`/`i18ndp`; the plugin exposes an enum, epochs and
   numbers. Times, durations and temperatures are formatted with the user's
   locale.
8. **Keyboard and accessibility**: up/down/tab navigation across the sliders,
   `Accessible` names on the controls, the tooltip as the compact
   representation's description.

## 3. What changed

```
package/contents/ui/
  main.qml                  rewritten  PlasmoidItem: title, state icon, tooltip, actions
  CompactRepresentation.qml rewritten  MouseArea + themed icon, wheel, middle-click
  FullRepresentation.qml    rewritten  Representation + ScrollView + footer, no title
  Icons.js                  new        condition token -> Breeze icon name (day/night)
  Format.js                 new        locale clock time
  components/
    DisplayItem.qml         new        ItemDelegate + slider, used for one display
                                       and for the "All displays" master row
    AutomationItem.qml      new        mode, reason, and the way back to automatic
    WeatherItem.qml         new        conditions and their effect, in words
    WeatherText.qml         new        translated condition names, shared by the
                                       popup and the panel tooltip
    SunTimesRow.qml         new        sunrise / solar noon / sunset, next in bold
    PopupFooter.qml         new        Pause (with durations) and Update Now
  config/ConfigGeneral.qml  rewritten  plain wording, spin boxes with units
removed:
  components/{AtmosphereCard,GlobalBrightnessCard,MilestoneTrack,
              MonitorSliderRow,ActionFooter}.qml
  WeatherIconResolver.js, icons/weather/*.svg (13), generate_weather_icons.py,
  screenshot_expanded.png (it was blank), __pycache__/
package/contents/icons/
  org.sunreactor.plasma.svg           redrawn: flat sun on the horizon, 48 grid
  org.sunreactor.plasma-symbolic.svg  redrawn: monochrome twin, 22 grid
package/metadata.json      "Adaptive Brightness", Keywords, FormFactors,
                           NotificationAreaCategory (system tray placement)
package/contents/config/main.xml   panel icon key reworked, two stale keys dropped
plugin/sunreactorclient.{h,cpp}    data-only surface, Mode enum, unknown stays unknown
tests/test_sunreactorclient.cpp    14 cases (was 7)
```

### Bugs found along the way

- `clearAllOverrides()` sent `clear_override` with `global: true`, which the
  daemon reads as "drop the global override only". Every override the popup
  sliders create is a per-display one, so the "Automatic" button could leave
  the widget in manual mode. It now sends `global: false`, which is the
  daemon's "drop all of them".
- An acknowledgement in reply to a status read made the client queue another
  status read, forever. A peer that answers everything with `ack` pinned a CPU.
  The client now only reads state back after a command that was not a status
  read (regression test: `testAcknowledgedStatusDoesNotLoop`).
- `monitor.topology === "laptop_internal"` never matched: `topology` carries
  reconciliation actions (`present`, `temporarily_unavailable`, ...), not a
  device class. Laptop panels are now recognised by their backend, and
  `temporarily_unavailable` is shown as "Unavailable".
- `ActionFooter` used `weather-sunset` and `weather-sunset-up`, which Breeze
  does not ship: two menu items had no icon.
- A monitor with no reported brightness was drawn as 50%.

## 4. Verification

- `qmllint` (Qt 6.11) with the plugin module resolvable: clean, apart from the
  `i18nd` unqualified-access notes every plasmoid produces.
- `ctest`: 14 passing cases against a stand-in local server.
- A QML harness loading every component against the running daemon: no
  warnings, and the live values parse (2 displays, weather ready, solar noon
  exactly between sunrise and sunset).
- `plasmoidviewer` with the popup forced open: no QML warnings, no binding
  loops; panel icon resolved to `weather-clear-night` at 03:00, tooltip read
  "mi-monitor at 5% / len-p24h-20-v305ptda at 8%" over "Following the sun ·
  19 °C · Clear".

## 5. Second pass, after using it

Feedback on the first pass, and what it changed.

| Asked for | Done |
|---|---|
| The icon is missing when the widget is added | The icon resolved fine, but `~/.local/share/icons/hicolor/icon-theme.cache` was older than the installed file, so lookups went through a cache that did not know about it. The cache is rebuilt after installing (`gtk-update-icon-cache`). |
| The widget should be called SunReactor | `Name: "SunReactor"`. The popup still shows no product name; the name lives where a name belongs, in the widget list and the tooltip title. |
| Drop the sun angle from the header | Gone from the popup entirely. It is now opt-in: *Show how high the sun stands* adds it beside the panel temperature and as a caption under the day track. |
| No brightness changes from scrolling | Wheel handling is gone, along with its two settings. A wheel event over a panel icon is too easy to trigger by accident, and the accident is every display changing at once. |
| "Duration" meant nothing | The old preset row is gone. Settings now say *Brightness you set by hand* and *Pausing*, each a list of plain durations ending in "Until I undo it", with one line of explanation. The popup states the consequence: "Set by hand · Until 19:40". |
| Weather in the panel: sprite, temperature | The panel shows the current sky as pixel art with the temperature beside it. Sprites are drawn at whole multiples of their 16px grid (16, 32, 48), because anything else turns pixel art into mush. |
| Improve the pixel art | Redrawn from scratch by `generate_weather_icons.py`, on the terminal cockpit's palette and shading rules: light from the upper left, five cloud tones, a crescent carved from a disc, plus-shaped snowflakes. 13 sprites, day and night variants, ~52 KB total. |

### Added in the same pass

- **Forecast strip** — the daemon already fetches a forecast for its own
  policy and the widget threw it away. The next five intervals now show as
  time, sprite and temperature.
- **Weather details on hover** — feels-like, humidity, wind and the US air
  quality index, each shown only when the provider sent it.
- **Day track** — a thin line under the sun times with the current moment
  marked, so "what happens next" has a place without another sentence.
- **Mode icons that do not repeat** — the automation row uses a clock, a
  document-edit, a pause or a dim-brightness icon, so no icon in the popup is
  used twice.

### Verified in this pass

- `ctest`: 15 cases, including forecast and details parsing, and that a
  reading the provider omitted stays absent instead of becoming zero.
- `qmllint` (Qt 6.11) with the plugin module resolvable: clean. It caught two
  real slips — stale `Icons.*` names and `Plasmoid.configuration` reached from
  a leaf component, which is now passed in as a property.
- Offscreen renders of the popup and of the panel at 22, 32 and 44 px, against
  the live daemon.
- `plasmoidviewer` with the popup forced open: no QML warnings, and the
  daemon's state afterwards confirms the widget writes nothing on its own -
  `manual_override_active` stayed false through a full open-and-render cycle.

## 6. Third pass: one design language

The sprites were a set of thirteen drawings that happened to share a palette.
They are now a family with written rules, at the top of
`generate_weather_icons.py`:

1. **Grid** — 16x16, drawn only at whole multiples of it (16, 32, 48).
2. **Light** — one source, upper left; darkest tone only as the grounding
   shadow; no black outlines.
3. **Bodies** — sun and moon are the *same* authored 8x8 disc, so a day
   sprite and its night twin weigh the same on screen. The crescent is that
   disc with a shifted copy of itself removed, carved from the lower right so
   the lit limb faces the light.
4. **Clouds** — one silhouette in two sizes; a cloud never floats, and
   whatever falls out of it starts one row below its shadow.
5. **Rhythm** — precipitation runs on a staggered two-row beat in fixed
   columns, so drizzle, rain and heavy rain read as the same weather getting
   worse.
6. **Night** — the palette is dimmed by one fixed factor, never redrawn.
7. **Palette** — the terminal cockpit's.

The discs, rays, clouds and bolt are authored masks now, not shapes computed
from a radius: the old sun had uneven rays because a loop rounded them into
place. Everything is hand-placed and reviewed at 16px on both a light and a
dark background.

### Two touches from the terminal cockpit

- **The sun rides the day.** The track under the sunrise/noon/sunset row
  carries the actual sky sprite at its native 16px, positioned by the real
  clock between sunrise and sunset, and it slides when the value changes.
- **The settings page previews the panel.** The top of the settings page draws
  the widget exactly as the panel will, from live daemon data, and the two
  checkboxes below change what is in the frame. No guessing what a setting
  does.

### Fixed while testing the real config path

`plasmoidviewer` opening the actual settings dialog surfaced warnings qmllint
cannot see: Plasma assigns a `cfg_<key>Default` twin for every key in
`main.xml`, and a page that does not declare them logs a failure per key. All
five are declared now. The `pinned` key had no control anywhere in the UI, so
it is gone rather than carried as dead configuration.
