#!/usr/bin/env bash
# Renders tools/harness/DotGallery.qml to one PNG, headless.
#
#   tools/dot_gallery.sh --out DIR [--name FILE.png] [--scale 1.35] [--build DIR]
#
# The QML imports the built org.sunreactor.plasma module (DotText, DotBar, ...), so
# it needs the plugin build: --build is the CMake build dir (default: ../build next
# to tools/). Dots are custom scene-graph geometry, which Qt Quick's software
# backend (what the offscreen platform gets) does not draw, so this runs under
# xvfb-run on a private Xvfb display with the xcb platform and Mesa's RHI OpenGL:
# no window appears on the desktop. --scale sets QT_SCALE_FACTOR, so the window's
# devicePixelRatio (what the dots snap to) matches the factor. The gallery reads tools/fixtures/themes.json with
# XMLHttpRequest, which Qt only allows for local files when QML_XHR_ALLOW_FILE_READ
# is set; this script sets it for that one process. Env: DOT_QML overrides qml6.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT=""
NAME="dot_gallery.png"
SCALE="1.35"
BUILD="$HERE/../build"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --name) NAME="$2"; shift 2 ;;
        --scale) SCALE="$2"; shift 2 ;;
        --build) BUILD="$2"; shift 2 ;;
        -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done

[[ -n "$OUT" ]] || { echo "--out DIR is required; keep images out of the repo" >&2; exit 2; }
[[ -f "$BUILD/qml/org/sunreactor/plasma/qmldir" ]] || { echo "no plugin build at $BUILD (run cmake --build first)" >&2; exit 2; }
QML="${DOT_QML:-$(command -v qml6 || true)}"
[[ -n "$QML" ]] || { echo "missing qml6 (set DOT_QML)" >&2; exit 2; }
command -v xvfb-run >/dev/null || { echo "missing xvfb-run (needed for the RHI backend)" >&2; exit 2; }
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
BUILD="$(cd "$BUILD" && pwd)"

# AF_UNIX paths are limited to 108 bytes, so the runtime dir must be short.
WORK="$(mktemp -d "${TMPDIR:-/tmp}/dots.XXXXXX")"
cleanup() {
    # Only ever remove the directory this script created.
    [[ -n "$WORK" && -d "$WORK" && "$(basename "$WORK")" == dots.* ]] && rm -rf -- "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/run" "$WORK/config" "$WORK/cache"
chmod 700 "$WORK/run"

xvfb-run -a -s "-screen 0 1920x1080x24" sh -c '
    exec env -i HOME="$1" PATH="/usr/bin:/usr/sbin:/bin" LANG="C.UTF-8" TZ="Europe/Istanbul" \
        XDG_RUNTIME_DIR="$2" XDG_CONFIG_HOME="$3" XDG_CACHE_HOME="$4" DISPLAY="$DISPLAY" \
        QT_QPA_PLATFORM=xcb QT_SCALE_FACTOR="$5" QT_FORCE_STDERR_LOGGING=1 QML_XHR_ALLOW_FILE_READ=1 \
        QML_IMPORT_PATH="$6" timeout 120 "$7" "$8" -- "$9"
' sh "$HOME" "$WORK/run" "$WORK/config" "$WORK/cache" "$SCALE" "$BUILD/qml" "$QML" \
    "$HERE/harness/DotGallery.qml" "$OUT/$NAME" > "$WORK/qml.log" 2>&1 || true

grep -E "DOT_GALLERY|ReferenceError|TypeError|SyntaxError|Error:|warning:" "$WORK/qml.log" | sed 's/^qml: //' || true
if [[ -f "$OUT/$NAME" ]]; then
    echo "$OUT/$NAME"
else
    echo "FAILED: no $NAME written; qml log tail:" >&2
    tail -n 20 "$WORK/qml.log" >&2
    exit 1
fi
