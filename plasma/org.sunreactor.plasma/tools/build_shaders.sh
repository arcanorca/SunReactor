#!/usr/bin/env bash
# Rebuilds the shader packs of the widget: every package/contents/ui/skins/*/shaders/*.frag
# becomes a .qsb beside it, compiled by Qt's qsb for the RHI backends (GLSL 100 es,
# 120 and 150; HLSL 5.0; Metal 1.2). The packs are committed, so the widget needs
# no build tools at run time; run this after editing a .frag.
#
#   tools/build_shaders.sh           rewrite the packs
#   tools/build_shaders.sh --check   build into a temporary directory and fail if a
#                                    committed pack differs (or is missing)
#
# qsb is found as $QSB, then on PATH, then in the Qt 6 libexec and bin directories.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
SKINS="$ROOT/package/contents/ui/skins"

CHECK=""
case "${1:-}" in
    "") ;;
    --check) CHECK=1 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
esac

QSB="${QSB:-}"
if [[ -z "$QSB" ]]; then
    for candidate in qsb /usr/lib/qt6/bin/qsb /usr/lib64/qt6/bin/qsb /usr/lib/qt6/libexec/qsb /usr/lib64/qt6/libexec/qsb; do
        if command -v "$candidate" >/dev/null 2>&1; then
            QSB="$candidate"
            break
        fi
    done
fi
[[ -n "$QSB" ]] || { echo "qsb not found (set QSB=/path/to/qsb)" >&2; exit 2; }

# Keep sources and packs in one sorted order, and the packs' names beside the source.
mapfile -t SOURCES < <(find "$SKINS" -path '*/shaders/*.frag' | sort)
[[ ${#SOURCES[@]} -gt 0 ]] || { echo "no shader sources under $SKINS" >&2; exit 2; }

OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/build_shaders.XXXXXX")"
cleanup() {
    # Only ever remove the directory this script created.
    [[ -n "$OUT_DIR" && -d "$OUT_DIR" && "$(basename "$OUT_DIR")" == build_shaders.* ]] && rm -rf -- "$OUT_DIR"
}
trap cleanup EXIT

status=0
for src in "${SOURCES[@]}"; do
    name="$(basename "$src").qsb"
    built="$OUT_DIR/$name"
    "$QSB" --glsl "100 es,120,150" --hlsl 50 --msl 12 -o "$built" "$src"
    committed="$(dirname "$src")/$name"
    if [[ -n "$CHECK" ]]; then
        if [[ ! -f "$committed" ]]; then
            echo "missing: $committed (run tools/build_shaders.sh)" >&2
            status=1
        elif ! cmp -s "$built" "$committed"; then
            echo "stale: $committed (run tools/build_shaders.sh)" >&2
            status=1
        fi
    else
        cp "$built" "$committed"
        echo "wrote $committed"
    fi
done
exit "$status"
