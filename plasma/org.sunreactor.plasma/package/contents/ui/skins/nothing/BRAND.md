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
- Type: three bundled OFL faces — `Doto` Bold for uppercase section headings and
  dot-matrix numerals, `Geist` Medium/Regular for crisp primary names, sentences
  and tabular figures, and `JetBrains Mono` (+10% tracking) for uppercase
  technical tags (`APPLIED`, `WEATHER`, `RANGE`, `FEELS`, `RH`, `WIND`, `AQI`,
  `RISE`, `NOON`, `SET`) and pills.
- Cards: radius 22 squircles, 1 device px hairline (`#3a3a3a` / 10% ink), matte
  charcoal fill (`#18181A` on OLED black), padding ~8–10 on every side. Every
  section on both the Brightness and Weather pages is a cohesive squircle card.
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
- Weather icons: official Nothing OS circular dot-matrix weather pictograms on the
  389×389 grid (`tools/vendor/nothing/` + exact-grid companions in
  `assets/weather/`), plus crisp parabolic solar horizon glyphs (`nothing_sunrise.png`,
  `nothing_sunset.png`).
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
