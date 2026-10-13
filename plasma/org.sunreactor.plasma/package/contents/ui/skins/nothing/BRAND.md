# Nothing theme: design brief (flagship, NOT pixel art)

The Nothing theme leaves the pixel family and looks like Nothing OS / Nothing
Phone widgets: dot matrix, black, white, grey, one red signal.

## Language
- Dot matrix: round dots on a regular grid (antialiased, equal diameter, centres
  snapped to device pixels), never square pixels. Lit dots are white or the red
  accent; unlit grid cells stay faint (12 % of ink) only where a grid is the point
  (rows, sliders, rings, the plot baseline). Numerals draw no unlit cells at all.
- Palette: black surface (OLED), white ink, #8a8a8a grey for secondary text, one
  red accent (#d71921). Red is the live lamp, the "now" dot, the ring head, the
  slider head and the one call to action. No gradients, shadows, glow or blur.
- Type: ONE label size. Captions, tags, pills and status sentences are small
  UPPERCASE mono (JetBrains Mono, +10 % tracking) or the same size in lower case.
  Values are mono figures. Numerals are the dot-matrix 5x7 face (DotText, size 1,
  letter spacing 1 dot column); never the 3x5 face for numbers. Times are text,
  never dot text (AM and PM would collide). Nothing's Ndot/Lettera fonts are
  proprietary; they are not bundled.
- Dot numerals: 5x7 face, one dot column between glyphs, lit white (ink). Every
  lit dot is 0.75 of its pitch
  (`digitScale`), so the numerals read as bold dots, not a speckle. Hero (target
  in the ring) at pitch 4 device px or more; per-display, forecast and telemetry
  numerals at exactly 4 device px (`pitchDigit`), 7 dots = 28 device px tall.
- Cards: radius 24, 1 device px hairline (#3a3a3a), filled #0a0a0a, padding 10 on
  every side, 8 between rows. Every section of the brightness page is a card;
  the weather reading is a card, forecast and sun times sit on the black as bare
  columns with hairline rules.
- Pills: one height (28 px at scale 1). Active = white fill with black text;
  inactive = hairline outline with grey text; hover lifts the outline to white.
- Section labels: grey, the label style. Only live state and "now" are red.
- Status lamp (components/StatusLamp.qml, this theme only): white while automatic,
  red when set by hand, grey while paused, a hollow red ring when the daemon is offline.
- Layout: left-aligned numerals, circular controls, a strict grid, 19 grid units
  wide. Brightness page about 500 logical px for one display (ALL row and more
  displays add rows); Weather about 340.

## Signature elements
- Light cycle: the target is a dotted ring (lit arc clockwise from the top, red
  head), the 5x7 percentage centred in it, the readings (APPLIED, WEATHER, RANGE)
  beside it.
- The day is a SPARSE dot plot: one lit dot per column at the curve height, a
  faint dotted baseline, the "now" column a thin dotted line with one red dot.
  For a single display the weather cost is a second row of tiny dim dots at the
  clear-sky height, above the curve. Never a filled block.
- Sliders are rows of round dots: lit white up to the value, one red head dot.
- Weather icons: 12 x 12 dot pictograms (sun = ring of dots, cloud = dotted
  silhouette, rain = falling dot columns, snow = scattered dots), drawn bare.
- Motion: none (idle CPU stays zero).
- Voice: lower-case or UPPERCASE sparse text, no emoji, plain facts
  ("automatic", "manual", "paused", "next 16:20 · 23%").

## Technical rules
- All colours come from theme tokens (palette bg/fg/accent/text_muted/
  border_inactive); no hex in QML. The theme id is "nothing"; its skin is
  registered under "id:nothing" in modules/Modules.js.
- Dots are crisp at DPR 1.0, 1.35 and 2.0: the pitch is an integer number of
  device pixels, dot diameter = pitch x dotScale rounded to whole device pixels
  (0.62 for grids, 0.75 for numerals).
- Dot fields are one QQuickItem each (DotField, one geometry node). Numerals,
  rings, bars and plots write cells with setCell/fill when their inputs change,
  never per frame.
- Files: NothingStyle (tokens and metrics), Pill, DotSlider, DotPlot (the day
  plot), and one file per module (LightCycle, Displays, Automation, Weather,
  Forecast, SunTimes, Telemetry, ThemeGlyph, RefreshControl, Lamp, Tabs).
