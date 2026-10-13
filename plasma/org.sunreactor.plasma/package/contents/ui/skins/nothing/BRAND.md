# Nothing theme: design brief (flagship, NOT pixel art)

The user: "there is no pixel art in Nothing, it is dot matrix". The Nothing theme must
leave the pixel family and look like Nothing OS / Nothing Phone widgets.

## Language
- Dot matrix: round dots on a regular grid (circular, antialiased, equal diameter,
  centres snapped to device pixels), never square pixels. Unlit dots stay faintly
  visible (about 12% of fg) like an LED matrix; lit dots are full fg or the red accent.
- Palette is strictly black / white / greys + ONE red accent (#d71921). Red is
  used sparingly: the live/record dot, the "now" marker, alerts, the one number that
  matters. No gradients, no shadows, no glow, no blur, no tinted colours.
  Pure black surface (OLED), white dot text, grey (#8a8a8a) for secondary.
- Typography: big numerals and headlines in a dot-matrix face drawn by our own
  generator (5x7 glyph tables already exist in plugin/pixelart.* PixelGlyphs; render
  each lit cell as a circle), labels in small UPPERCASE monospace with wide letter
  spacing (use the bundled JetBrains Mono, 9-10 px logical, tracking +8%) -- Nothing's
  Ndot/Lettera fonts are proprietary, do not bundle them.
- Layout: generous radius containers (card radius 20-28 logical px, 1 device-px
  hairline outline in #3a3a3a, or filled #0a0a0a on black), lots of air, strict
  grid, left-aligned numerals, circular controls. Transparent/see-through feel.
  Sliders are rows of dots (lit up to the value, one red dot as the thumb), rings
  are dotted circles, the day curve is a column-of-dots plot (each column of dots
  lit to the curve height, the current column red).
- Weather icons: dot-matrix pictograms on a 12x12 dot grid (sun = ring of dots,
  cloud = dotted silhouette, rain = falling dot columns, snow = scattered dots).
- Motion: none by default (idle CPU must stay zero); a live red dot may blink at
  0.5 Hz only if the client reports live and motion effects are enabled.
- Voice: lowercase/uppercase sparse text, no emoji, plain facts ("auto", "manual",
  "paused", "next 16:20 -> 23%").

## Technical rules
- All colours come from theme tokens (palette bg/fg/accent/text_muted/border_inactive);
  do not hardcode hex in QML except via tokens. The theme id is "nothing"; register
  skins under the SKINS key "id:nothing" in modules/Modules.js and the tab/footer/
  panel parts tables the same way other flagships do.
- Dots must be crisp at DPR 1.0/1.35/2.0: choose the dot pitch as an integer number
  of device pixels, dot diameter = pitch * 0.62 rounded to whole device px.
- Draw dot fields with ONE custom QQuickItem (scene-graph geometry, one batch of
  quads/circles or an SDF-free textured quad), never one QML item per dot.
