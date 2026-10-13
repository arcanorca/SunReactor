#!/usr/bin/env bash
# Renders tools/harness/GlyphGallery.qml to one PNG, headless.
#
#   tools/glyph_gallery.sh --out DIR [--name FILE.png] [--scale 1.35]
#
# The QML runs on Qt's offscreen platform: no X server, no window on the desktop.
# --scale sets QT_SCALE_FACTOR, so the window's devicePixelRatio (what the glyphs
# snap to) matches the factor. The gallery reads tools/fixtures/themes.json with
# XMLHttpRequest, which Qt only allows for local files when QML_XHR_ALLOW_FILE_READ
# is set; this script sets it for that one process. Env: GLYPH_QML overrides qml6.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT=""
NAME="glyph_gallery.png"
SCALE="1.35"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --name) NAME="$2"; shift 2 ;;
        --scale) SCALE="$2"; shift 2 ;;
        -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done

[[ -n "$OUT" ]] || { echo "--out DIR is required; keep images out of the repo" >&2; exit 2; }
QML="${GLYPH_QML:-$(command -v qml6 || true)}"
[[ -n "$QML" ]] || { echo "missing qml6 (set GLYPH_QML)" >&2; exit 2; }
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

# AF_UNIX paths are limited to 108 bytes, so the runtime dir must be short.
WORK="$(mktemp -d "${TMPDIR:-/tmp}/glyph.XXXXXX")"
cleanup() {
    # Only ever remove the directory this script created.
    [[ -n "$WORK" && -d "$WORK" && "$(basename "$WORK")" == glyph.* ]] && rm -rf -- "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/run" "$WORK/config" "$WORK/cache"
chmod 700 "$WORK/run"

env -i HOME="$HOME" PATH="/usr/bin:/usr/sbin:/bin" LANG="C.UTF-8" TZ="Europe/Istanbul" \
    XDG_RUNTIME_DIR="$WORK/run" XDG_CONFIG_HOME="$WORK/config" XDG_CACHE_HOME="$WORK/cache" \
    QT_QPA_PLATFORM=offscreen QT_SCALE_FACTOR="$SCALE" QT_FORCE_STDERR_LOGGING=1 QML_XHR_ALLOW_FILE_READ=1 \
    timeout 120 "$QML" "$HERE/harness/GlyphGallery.qml" -- "$OUT/$NAME" \
    > "$WORK/qml.log" 2>&1 || true

grep -E "GLYPH_GALLERY|ReferenceError|TypeError|SyntaxError|Error:" "$WORK/qml.log" | sed 's/^qml: //' || true
if [[ -f "$OUT/$NAME" ]]; then
    echo "$OUT/$NAME"
else
    echo "FAILED: no $NAME written; qml log tail:" >&2
    tail -n 20 "$WORK/qml.log" >&2
    exit 1
fi
