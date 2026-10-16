# Flagship skin brief (shared by casiodigital, cyberpunk, synthwave84, hackergreen)

Problem: these four themes were in the pixel family and all shared the generic 5x7 pixel
font and neon chrome, so they did not look like what they are named after. Each now gets
its own skin folder `skins/<theme id>/` that looks authentically like its subject.

Rules
- Own folder skins/<id>/ with Telemetry, LightCycle, Displays, Automation, Weather, Forecast,
  SunTimes (module contract: modules/Modules.js header). You may import and wrap shared parts
  from ../instrument/ (Fader, TickScale, Tag, Digits, Glow, Curve.js, Legend ...) or ../terminal/
  (TermFrame, TermBar, TermText ...) by relative path, but do NOT edit those folders.
- Register ONLY under SKINS key "id:<theme id>" in modules/Modules.js (and the TABBAR / footer
  Parts tables only if you ship custom parts; otherwise the generic instrument/terminal ones are used).
  Use small atomic edits (other agents edit the same file concurrently): re-read the file right
  before editing, change only your own key line(s).
- Mode control: reuse modules/ModeControl.qml like skins/instrument/Automation.qml does.
- All colours from theme tokens (theme palette); fonts: bundled only (see fonts/FONTS-LICENSES.txt;
  Orbitron-Variable, IBM Plex, Geist, JetBrains Mono, PressStart2P, unscii, VT323 exist). Add
  an OFL font only with a licence note. No new C++ unless essential.
- No animation by default (idle CPU stays zero). Crisp at DPR 1.0/1.35/2.0 (snap to device pixels).
- Text must not be giant: the hero percentage is at most ~2.6x the body size, never more than a
  third of the popup width.
- Headless only: `cd plasma/org.sunreactor.plasma && cmake --build build -j12` (only if C++ changed)
  then `SHOT_SERVERNUM=<your number> TMPDIR=/tmp/claude-1000 tools/shot.sh --out <scratch dir>
  --tag x --themes <id> --scales "1.0 1.35 2.0" --views popup` (also `--config "tab=weather"`, and
  `--scenario manual|paused|triple` at 1.35). First update tools/fixtures/themes.json entry for
  your theme to match src/theme/builtin/<id>.toml (family/widget block changed; do not touch other
  entries). View PNGs with the Read tool, critique like a designer, iterate at most 3 rounds.
  Check: no QML warnings in .log files; /usr/lib/qt6/bin/qmllint -I build/qml on your files;
  ctest passes. Never open windows on the desktop. Commit only your files. Report <= 8 lines with paths.
