#!/usr/bin/env bash
# Headless screenshots of the widget: fake daemon + plasmoidviewer under Xvfb.
#
#   tools/shot.sh --out DIR [--tag NAME] [--themes "handheld braun dracula"]
#                 [--scales "1.0 1.35 2.0"] [--views "popup panel-h panel-v"]
#                 [--scenario default|manual|paused|single|triple|longnames|offline|unknown|noweather]
#                 [--old] [--scheme dark|light] [--theme-warning TEXT]
#                 [--config "panelMode=ring showTemperature=false"]
#                 [--font-dpi N] [--units N] [--audit]
#                 [--font "Inter:12"] [--fixed-font "Hack:12"] [--small-font "Inter Display:9"]
#                 [--package DIR] [--qml DIR]
#
# Output: DIR/<tag>_<theme>_<view>_<scale>.png plus a .log with the viewer's QML
# warnings next to each PNG. The popup view crops to the popup at its own size:
# the harness containment logs it (SUNREACTOR_HARNESS_POPUP) and shot.sh prints
# it. The panel views are cropped to the viewer window, in device pixels.
# --config sets plasmoid config keys (main.xml names; values are JSON when they
# parse, else strings) through SUNREACTOR_HARNESS_CONFIG, which the widget
# only reads under SUNREACTOR_HARNESS (plasmoidviewer recreates its layout
# file on every start, so it cannot be seeded). Env: SHOT_PANEL_THICK / SHOT_PANEL_LEN / SHOT_POPUP_W /
# SHOT_POPUP_H (logical px; the window the popup is shown in, at least as tall
# as the popup can grow), SHOT_SCREEN (Xvfb screen, WxH device px; default
# 2400x2400: the popup's maximum height is 75% of the screen, so a smaller one
# makes a page scroll), SHOT_SETTLE (s), SHOT_SERVERNUM (Xvfb display
# number; default: pick a free one), SHOT_KEEP=1 keeps the work dir.
# --font-dpi N sets QT_FONT_DPI as given; in this Qt that also scales the device
# pixel ratio by N/96 (see tools/overflow_audit.sh). --units N sets the popup's
# width in grid units (popupGridUnits,
# popup view only). --audit runs the layout audit (tools/harness/Audit.qml): the
# screenshot is taken once SUNREACTOR_AUDIT_END is in the viewer's log, and the
# audit's marker lines stay in DIR/<tag>_<theme>_<view>_<scale>.log.
#
# --font / --fixed-font / --small-font set the viewer's fonts (the general,
# fixed and smallestReadable entries of kdeglobals) as "Family:pt" or
# "Family,pt" (env SHOT_FONT, SHOT_FIXED_FONT, SHOT_SMALL_FONT). They reach
# Kirigami.Theme (defaultFont, smallFont) through the KDE platform theme, which
# the harness selects whenever a font is set (SHOT_QPA_THEME overrides). Unset =
# the harness default, which has no kdeglobals fonts. Kirigami.Units.gridUnit
# stays 18 px in this harness at any font size, so the popup stays 19 grid units
# (342 px); the terminal skins' grid is fitted to that width, not to the font.
#
# Isolation (nothing reaches the user's session):
#   - env -i: no WAYLAND_DISPLAY / DISPLAY of the desktop, no user QML paths;
#   - Xvfb display of our own, a private D-Bus session (dbus-run-session);
#   - XDG_CONFIG_HOME / XDG_DATA_HOME / XDG_CACHE_HOME in a throwaway dir, so the
#     installed widget in ~/.local/share/plasma is invisible and plasmoidviewer
#     writes no config into ~/.config;
#   - XDG_RUNTIME_DIR is a private short dir; the widget's default socket path
#     therefore resolves to the fake daemon, never to the real control socket
#     (fake_daemon.py also refuses to bind the real path);
#   - QML_IMPORT_PATH is only --qml (the build tree), never ~/.local/lib/qt6/qml.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
PACKAGE="$ROOT/package"
QMLDIR="$ROOT/build/qml"
OUT="${SHOT_OUT:-}"
TAG="shot"
THEMES="handheld braun dracula"
SCALES="1.0 1.35 2.0"
VIEWS="popup panel-h panel-v"
SCENARIO="default"
OLD=""
CONFIG=""
FONT_DPI=""
UNITS=""
AUDIT=""
WARNING=""
SCHEME="dark"
SETTLE="${SHOT_SETTLE:-2.5}"
FONT="${SHOT_FONT:-}"
FIXED_FONT="${SHOT_FIXED_FONT:-}"
SMALL_FONT="${SHOT_SMALL_FONT:-}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --tag) TAG="$2"; shift 2 ;;
        --themes) THEMES="$2"; shift 2 ;;
        --scales) SCALES="$2"; shift 2 ;;
        --views) VIEWS="$2"; shift 2 ;;
        --scenario) SCENARIO="$2"; shift 2 ;;
        --old) OLD="--old"; shift ;;
        --config) CONFIG="$2"; shift 2 ;;
        --font-dpi) FONT_DPI="$2"; shift 2 ;;
        --units) UNITS="$2"; shift 2 ;;
        --audit) AUDIT=1; shift ;;
        --theme-warning) WARNING="$2"; shift 2 ;;
        --scheme) SCHEME="$2"; shift 2 ;;
        --font) FONT="$2"; shift 2 ;;
        --fixed-font) FIXED_FONT="$2"; shift 2 ;;
        --small-font) SMALL_FONT="$2"; shift 2 ;;
        --package) PACKAGE="$(cd "$2" && pwd)"; shift 2 ;;
        --qml) QMLDIR="$(cd "$2" && pwd)"; shift 2 ;;
        -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done

[[ -n "$OUT" ]] || { echo "--out DIR (or SHOT_OUT) is required; keep screenshots out of the repo" >&2; exit 2; }
# Harness config keys this run adds to --config (read by the containment and the widget).
HARNESS_CONFIG="$CONFIG"
[[ -n "$UNITS" ]] && HARNESS_CONFIG="$HARNESS_CONFIG popupGridUnits=$UNITS"
[[ -n "$AUDIT" ]] && HARNESS_CONFIG="$HARNESS_CONFIG audit=1"
# QT_FONT_DPI is passed as given. In this Qt it also multiplies the window's
# device pixel ratio: scale x FONT_DPI/96 (1.35 at 144 is 2.025). The crops below
# use that effective ratio; the popup line and the audit's META record it.
extra_env=()
[[ -n "$FONT_DPI" ]] && extra_env+=("QT_FONT_DPI=$FONT_DPI")
[[ -f "$QMLDIR/org/sunreactor/plasma/qmldir" ]] || { echo "no plugin under $QMLDIR (build first)" >&2; exit 2; }
for tool in xvfb-run plasmoidviewer import magick dbus-run-session python3; do
    command -v "$tool" >/dev/null || { echo "missing $tool" >&2; exit 2; }
done
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

# AF_UNIX paths are limited to 108 bytes, so the runtime dir must be short.
RUNPARENT="${SHOT_RUNTIME_PARENT:-${TMPDIR:-/tmp}}"
WORK="$(mktemp -d "$RUNPARENT/srshot.XXXXXX")"
cleanup() {
    [[ -n "${FAKE_PID:-}" ]] && kill "$FAKE_PID" 2>/dev/null || true
    # Only ever remove the directory this script created.
    [[ -n "${SHOT_KEEP:-}" ]] && { echo "kept $WORK" >&2; return; }
    [[ -n "$WORK" && -d "$WORK" && "$(basename "$WORK")" == srshot.* ]] && rm -rf -- "$WORK"
}
trap cleanup EXIT

mkdir -p "$WORK/run" "$WORK/config" "$WORK/data" "$WORK/cache"
chmod 700 "$WORK/run"
SOCK="$WORK/run/sunreactor/control.sock"

# A fixed colour scheme makes runs comparable between machines and sessions.
SCHEME_FILE="/usr/share/color-schemes/Breeze${SCHEME^}.colors"
[[ "$SCHEME" == "light" ]] && SCHEME_FILE="/usr/share/color-schemes/BreezeLight.colors"
# Fonts in Qt's QFont string form ("Family,pt,-1,5,400,0,0,0,0,0,0,0,0,0,0,1",
# the shape of a real kdeglobals entry); the same form for all three slots.
qt_font() {
    python3 - "$1" <<'PY'
import re, sys
spec = sys.argv[1].strip()
m = re.match(r"^(.*?)[:,]\s*(\d+(?:\.\d+)?)$", spec)
family, size = (m.group(1).strip(), m.group(2)) if m else (spec, "10")
print(f"{family},{size},-1,5,400,0,0,0,0,0,0,0,0,0,0,1")
PY
}
FONT_LINES=""
[[ -n "$FONT" ]] && FONT_LINES+="font=$(qt_font "$FONT")"$'\n'
[[ -n "$FIXED_FONT" ]] && FONT_LINES+="fixed=$(qt_font "$FIXED_FONT")"$'\n'
[[ -n "$SMALL_FONT" ]] && FONT_LINES+="smallestReadable=$(qt_font "$SMALL_FONT")"$'\n'
if [[ -f "$SCHEME_FILE" ]]; then
    { cat "$SCHEME_FILE"; printf '\n[General]\nColorScheme=Breeze%s\n' "${SCHEME^}"; printf '%s' "$FONT_LINES"; } > "$WORK/config/kdeglobals"
elif [[ -n "$FONT_LINES" ]]; then
    { printf '[General]\n'; printf '%s' "$FONT_LINES"; } > "$WORK/config/kdeglobals"
fi
# Fonts reach QGuiApplication (and Kirigami.Theme.defaultFont) only through the
# KDE platform theme, which reads kdeglobals; the harness default leaves Qt's own
# theme. SHOT_QPA_THEME overrides; any font setting selects "kde".
QPA="${SHOT_QPA_THEME:-}"
[[ -z "$QPA" && -n "$FONT_LINES" ]] && QPA=kde
[[ -n "$QPA" ]] && extra_env+=("QT_QPA_PLATFORMTHEME=$QPA")

# plasmoidviewer's own shell overlays an SDK toolbar and stretches the view to
# the whole screen. A private copy of the shell with tools/harness/Desktop.qml
# swapped in removes both; the harness containment shows the full
# representation filling the window (the popup), the real panel containment
# shows the compact one at panel thickness.
SHELL_SRC="/usr/share/plasma/shells/org.kde.plasma.plasmoidviewershell"
if [[ -d "$SHELL_SRC" ]]; then
    mkdir -p "$WORK/data/plasma/shells"
    cp -r "$SHELL_SRC" "$WORK/data/plasma/shells/"
    cp "$HERE/harness/Desktop.qml" "$WORK/data/plasma/shells/org.kde.plasma.plasmoidviewershell/contents/views/Desktop.qml"
fi
mkdir -p "$WORK/data/plasma/plasmoids"
cp -r "$HERE/harness/containment" "$WORK/data/plasma/plasmoids/org.sunreactor.harness"
# The layout audit is loaded by the containment only under --audit; it lives next to main.qml.
cp "$HERE/harness/Audit.qml" "$WORK/data/plasma/plasmoids/org.sunreactor.harness/contents/ui/Audit.qml"

POPUP_W="${SHOT_POPUP_W:-460}"
POPUP_H="${SHOT_POPUP_H:-1000}"
PANEL_LEN="${SHOT_PANEL_LEN:-300}"
PANEL_THICK="${SHOT_PANEL_THICK:-44}"
view_args() {
    case "$1" in
        popup)   echo "-c org.sunreactor.harness -f planar -s ${POPUP_W}x${POPUP_H}" ;;
        panel-h) echo "-c org.kde.panel -f horizontal -l bottomedge -s ${PANEL_LEN}x${PANEL_THICK}" ;;
        panel-v) echo "-c org.kde.panel -f vertical -l leftedge -s ${PANEL_THICK}x${PANEL_LEN}" ;;
        *) echo "unknown view $1" >&2; exit 2 ;;
    esac
}
view_size() {
    case "$1" in
        popup) echo "$POPUP_W $POPUP_H" ;;
        panel-h) echo "$PANEL_LEN $PANEL_THICK" ;;
        panel-v) echo "$PANEL_THICK $PANEL_LEN" ;;
    esac
}

XVFB_NUM="-a"
[[ -n "${SHOT_SERVERNUM:-}" ]] && XVFB_NUM="-n ${SHOT_SERVERNUM}"


shoot() {
    local theme="$1" view="$2" scale="$3"
    local name="${TAG}_${theme}_${view}_${scale}"
    local log="$WORK/requests.log"
    : > "$log"
    python3 "$HERE/fake_daemon.py" --socket "$SOCK" --theme "$theme" --scenario "$SCENARIO" $OLD \
        --theme-warning "$WARNING" --log "$log" > "$WORK/fake.out" 2>&1 &
    FAKE_PID=$!
    for _ in $(seq 50); do [[ -S "$SOCK" ]] && break; sleep 0.1; done
    [[ -S "$SOCK" ]] || { echo "fake daemon did not start: $(cat "$WORK/fake.out")" >&2; exit 1; }

    read -r lw lh < <(view_size "$view")
    local w h eff
    eff=$(python3 -c "print(round($scale*${FONT_DPI:-96}/96, 6))")
    w=$(python3 -c "print(round($lw*$eff))")
    h=$(python3 -c "print(round($lh*$eff))")
    cat > "$WORK/inner.sh" <<EOF
plasmoidviewer -a "$PACKAGE" $(view_args "$view") -x 0 -y 0 > "$OUT/$name.log" 2>&1 &
VP=\$!
# Wait for the widget to have read status at least twice, then let it settle.
for i in \$(seq 150); do
    [ "\$(grep -c '"status"' "$log" 2>/dev/null || echo 0)" -ge 2 ] && break
    sleep 0.1
done
if [ -n "$AUDIT" ]; then
    # The audit has measured the settled popup; wait for its end marker.
    for i in \$(seq 300); do grep -q -e SUNREACTOR_AUDIT_END -e SUNREACTOR_AUDIT_ERROR "$OUT/$name.log" 2>/dev/null && break; sleep 0.1; done
    sleep 0.2
else
    sleep $SETTLE
fi
import -window root -crop ${w}x${h}+0+0 +repage "$WORK/$name.png"
kill \$VP 2>/dev/null
wait \$VP 2>/dev/null
EOF
    env -i HOME="$HOME" USER="${USER:-}" PATH="/usr/bin:/usr/sbin:/bin" LANG="en_US.UTF-8" \
        TZ="Europe/Istanbul" \
        XDG_RUNTIME_DIR="$WORK/run" XDG_CONFIG_HOME="$WORK/config" XDG_DATA_HOME="$WORK/data" \
        XDG_CACHE_HOME="$WORK/cache" QML_IMPORT_PATH="$QMLDIR" QT_SCALE_FACTOR="$scale" \
        SUNREACTOR_HARNESS=1 SUNREACTOR_HARNESS_CONFIG="$HARNESS_CONFIG" QT_FORCE_STDERR_LOGGING=1 \
        ${extra_env[@]+"${extra_env[@]}"} \
        dbus-run-session -- xvfb-run $XVFB_NUM -s "-screen 0 ${SHOT_SCREEN:-2400x2400}x24" sh "$WORK/inner.sh" \
        > "$WORK/xvfb.out" 2>&1 || true
    kill "$FAKE_PID" 2>/dev/null || true
    wait "$FAKE_PID" 2>/dev/null || true
    FAKE_PID=""
    cp "$log" "$OUT/$name.requests.jsonl"
    # Pixel surfaces report where they landed (device px); conformance.py
    # --regions checks exactly those areas of the capture.
    python3 - "$OUT/$name.log" "$OUT/$name.regions.json" <<'PY'
import json, sys
regions = {}
for line in open(sys.argv[1], errors="replace"):
    if "SUNREACTOR_PIXEL_REGION " in line:
        r = json.loads(line.split("SUNREACTOR_PIXEL_REGION ", 1)[1])
        regions[r["name"]] = r
json.dump(list(regions.values()), open(sys.argv[2], "w"), indent=1)
PY
    # The popup's extent (window px) comes from the containment's log line; a
    # view without one (the panels) keeps the whole window.
    local cw ch lw lh
    read -r cw ch lw lh < <(python3 - "$OUT/$name.log" "$eff" "$w" "$h" <<'PY'
import json, sys
log, scale, win_w, win_h = sys.argv[1], float(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
popup = None
for line in open(log, errors="replace"):
    if "SUNREACTOR_HARNESS_POPUP " in line:
        popup = json.loads(line.split("SUNREACTOR_HARNESS_POPUP ", 1)[1])
if popup is None:
    print(win_w, win_h, "-", "-")
else:
    print(round((popup["x"] + popup["width"]) * scale), round((popup["y"] + popup["height"]) * scale),
          f"{popup['width']:g}", f"{popup['height']:g}")
PY
    )
    if [[ -f "$WORK/$name.png" ]]; then
        if [[ "$lw" == "-" ]]; then
            mv "$WORK/$name.png" "$OUT/$name.png"
        else
            (( cw <= w && ch <= h )) || echo "warning: $name popup ${lw}x${lh} is taller than the window (SHOT_POPUP_H)" >&2
            magick "$WORK/$name.png" -crop "${cw}x${ch}+0+0" +repage "$OUT/$name.png"
        fi
    fi
    if [[ -f "$OUT/$name.png" ]]; then
        local popup_note=""
        [[ "$lw" == "-" ]] || popup_note="popup ${lw}x${lh}, "
        echo "$OUT/$name.png (${popup_note}$(grep -c . "$log") requests)"
    else
        echo "FAILED $name" >&2
    fi
}

for theme in $THEMES; do
    for view in $VIEWS; do
        for scale in $SCALES; do
            shoot "$theme" "$view" "$scale"
        done
    done
done
