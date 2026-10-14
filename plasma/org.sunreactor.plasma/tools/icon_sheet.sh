#!/usr/bin/env bash
# Renders tools/harness/IconSheet.qml: a theme's registry weather icon, every
# condition by day and by night, at each size, on the theme's background. One PNG
# per device pixel ratio, headless (Qt offscreen, no window on the desktop).
#
#   tools/icon_sheet.sh --theme ID --out DIR [--sizes "22 32 48 96"]
#                       [--dprs "1.0 1.35 2.0"] [--style auto|modern|line|solid|pixel]
#                       [--qml DIR]
#
# Output: DIR/icon_sheet_<theme>_dpr<ratio>.png, one per ratio; the paths are printed.
# The icon is the one the widget shows for that theme (Modules.weatherIcon), so the
# sheet follows the registry. --qml is the build tree's qml dir that holds
# org/sunreactor/plasma (default: build/qml; build the plugin first).
# Env: ICON_QML overrides qml6.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
THEME=""
OUT=""
STYLE="auto"
SIZES="22 32 48 96"
DPRS="1.0 1.35 2.0"
QMLDIR="$ROOT/build/qml"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --theme) THEME="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        --sizes) SIZES="$2"; shift 2 ;;
        --dprs) DPRS="$2"; shift 2 ;;
        --style) STYLE="$2"; shift 2 ;;
        --qml) QMLDIR="$2"; shift 2 ;;
        -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done

[[ -n "$THEME" ]] || { echo "--theme ID is required" >&2; exit 2; }
[[ "$THEME" =~ ^[a-z0-9_]+$ ]] || { echo "theme id must be [a-z0-9_]+: $THEME" >&2; exit 2; }
[[ "$STYLE" =~ ^(auto|modern|line|solid|pixel)$ ]] || { echo "--style must be auto|modern|line|solid|pixel: $STYLE" >&2; exit 2; }
[[ -n "$OUT" ]] || { echo "--out DIR is required; keep images out of the repo" >&2; exit 2; }
QML="${ICON_QML:-$(command -v qml6 || true)}"
[[ -n "$QML" ]] || { echo "missing qml6 (set ICON_QML)" >&2; exit 2; }
[[ -f "$QMLDIR/org/sunreactor/plasma/qmldir" ]] || { echo "no plugin under $QMLDIR (build first)" >&2; exit 2; }
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

# AF_UNIX paths are limited to 108 bytes, so the runtime dir must be short.
WORK="$(mktemp -d "${TMPDIR:-/tmp}/iconsheet.XXXXXX")"
cleanup() {
    # Only ever remove the directory this script created.
    [[ -n "$WORK" && -d "$WORK" && "$(basename "$WORK")" == iconsheet.* ]] && rm -rf -- "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/run" "$WORK/config" "$WORK/cache"
chmod 700 "$WORK/run"

status=0
for dpr in $DPRS; do
    name="icon_sheet_${THEME}_dpr${dpr}.png"
    log="$WORK/qml_${dpr}.log"
    env -i HOME="$HOME" PATH="/usr/bin:/usr/sbin:/bin" LANG="C.UTF-8" TZ="Europe/Istanbul" \
        XDG_RUNTIME_DIR="$WORK/run" XDG_CONFIG_HOME="$WORK/config" XDG_CACHE_HOME="$WORK/cache" \
        QT_QPA_PLATFORM=offscreen QT_SCALE_FACTOR="$dpr" QT_FORCE_STDERR_LOGGING=1 \
        QML_XHR_ALLOW_FILE_READ=1 QML_IMPORT_PATH="$QMLDIR" \
        timeout 120 "$QML" "$HERE/harness/IconSheet.qml" -- "$OUT/$name" "$THEME" "$SIZES" "$STYLE" \
        > "$log" 2>&1 || true

    if grep -qE "WARNING|ReferenceError|TypeError|SyntaxError|Error:" "$log"; then
        grep -E "WARNING|ReferenceError|TypeError|SyntaxError|Error:" "$log" | sed 's/^qml: //' >&2
        status=1
    fi
    if [[ -f "$OUT/$name" ]]; then
        echo "$OUT/$name"
    else
        echo "FAILED: no $name written; qml log tail:" >&2
        tail -n 20 "$log" >&2
        status=1
    fi
done
exit "$status"
