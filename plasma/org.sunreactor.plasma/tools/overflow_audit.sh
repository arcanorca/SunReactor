#!/usr/bin/env bash
# Layout audit of the popup across themes, pages, scales, font DPI, popup widths
# and fake-daemon scenarios. Each combination is one headless render through
# tools/shot.sh --audit: the harness walks the popup's item tree after the
# daemon data has arrived and prints its findings (tools/harness/Audit.qml),
# which tools/overflow_summarize.py turns into per-theme counts and reports.
#
#   tools/overflow_audit.sh [--quick | --full] [--out DIR] [--themes "a b"]
#                           [--pages "brightness weather"] [--dprs "1.35"]
#                           [--dpis "96 144"] [--scenarios "default longnames"]
#                           [--units "19"] [--fontpt "10 12"] [--jobs 4] [--dry-run]
#
#   --quick     DPR 1.35; DPI 96 and 144; scenarios default and longnames;
#               both pages; popup 19 grid units; fonts 10 and 12 pt.
#               2 x 2 x 2 x 2 renders per theme.
#   --full      DPR 1.0 1.35 2.0; DPI 96 120 144; scenarios default longnames
#               triple manual; both pages; fonts 10 12 14 pt. 324 renders per theme.
#   --fontpt    the viewer's UI font size in pt (the matrix dimension "pt"): the
#               general font is Inter at N pt, the fixed font Hack at N pt, the
#               smallest readable Inter Display 9 pt, as in the user's kdeglobals
#               (shot.sh --font / --fixed-font / --small-font, KDE platform theme).
#               "none" = the harness default (no font settings, Qt's own theme).
#   --themes    subset of the entries of tools/fixtures/themes.json (default: all)
#   --units     popup width in grid units (17 and 19 test a narrower popup)
#   --jobs      parallel viewers, one Xvfb each on displays 80, 81, ... (max 4)
#   --out       output directory (default: ~/.cache/sunreactor/overflow-audit/STAMP)
#
# Output in DIR:
#   overflow_report.md / overflow_report.json   per-theme counts, top offenders
#   shots/<name>.png                            the popup of each combination
#   logs/<name>.log                             the viewer's log, audit lines included
#   manifest.tsv                                the combinations that ran
#   raw/                                        shot.sh's work files (requests, regions)
# A name is theme__page__dprX__dpiY__scenario__guN.
#
# Findings (see tools/harness/Audit.qml for the rules):
#   OVERFLOW   an item's box leaves its frame, clipping item or scroll viewport,
#              or the popup horizontally
#   CLIPPED    text cut without an ellipsis;  ELIDED: cut with one
#   COLLISION  two text items in one row overlap by more than 2 px both ways
# Items marked `property bool auditIgnore: true` are skipped with their subtree
# (for decorative art that paints outside its card; no skin sets it yet).
#
# Font DPI: QT_FONT_DPI is passed as given. In this Qt it also multiplies the
# window's device pixel ratio (1.35 at 144 renders at 2.025), so a DPI row is
# also a pixel-grid row. Each META line records the ratio that was in effect.
#
# Environment: SHOT_POPUP_W / SHOT_POPUP_H (logical px, default 900 x 1000),
# SHOT_SCREEN (default 3840x2160), SHOT_SERVERNUM (first display, default 80).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
FIXTURES="$HERE/fixtures/themes.json"

MODE=""
OUT=""
THEMES=""
PAGES="brightness weather"
DPRS="1.35"
DPIS="96 144"
SCENARIOS="default longnames"
UNITS="19"
FONTPTS="10 12"
JOBS=4
DRY=""

usage() { sed -n '2,33p' "$0"; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick) MODE=quick; shift ;;
        --full) MODE=full; shift ;;
        --out) OUT="$2"; shift 2 ;;
        --themes) THEMES="$2"; shift 2 ;;
        --pages) PAGES="$2"; shift 2 ;;
        --dprs) DPRS="$2"; shift 2 ;;
        --dpis) DPIS="$2"; shift 2 ;;
        --scenarios) SCENARIOS="$2"; shift 2 ;;
        --units) UNITS="$2"; shift 2 ;;
        --fontpt) FONTPTS="$2"; shift 2 ;;
        --jobs) JOBS="$2"; shift 2 ;;
        --dry-run) DRY=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option $1" >&2; usage >&2; exit 2 ;;
    esac
done

if [[ "$MODE" == full ]]; then
    DPRS="1.0 1.35 2.0"
    DPIS="96 120 144"
    SCENARIOS="default longnames triple manual"
    FONTPTS="10 12 14"
fi

(( JOBS >= 1 && JOBS <= 4 )) || { echo "--jobs must be 1 to 4 (displays 80-83)" >&2; exit 2; }
[[ -f "$FIXTURES" ]] || { echo "no $FIXTURES" >&2; exit 2; }
for tool in python3 magick import xvfb-run plasmoidviewer dbus-run-session; do
    command -v "$tool" >/dev/null || { echo "missing $tool" >&2; exit 2; }
done

if [[ -z "$THEMES" ]]; then
    THEMES="$(python3 -c "import json,sys; print(' '.join(k for k in json.load(open(sys.argv[1])) if not k.startswith('_')))" "$FIXTURES")"
fi
THEMES="${THEMES//,/ }"

STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${OUT:-${XDG_CACHE_HOME:-$HOME/.cache}/sunreactor/overflow-audit/$STAMP}"
MANIFEST="$OUT/manifest.tsv"

# The matrix, one combination per line; the manifest is what the summary reads.
combos=()
for theme in $THEMES; do
    for page in $PAGES; do
        for dpr in $DPRS; do
            for dpi in $DPIS; do
                for scen in $SCENARIOS; do
                    for units in $UNITS; do
                        for fpt in $FONTPTS; do
                            combos+=("$theme	$page	$dpr	$dpi	$scen	$units	$fpt")
                        done
                    done
                done
            done
        done
    done
done
total=${#combos[@]}
echo "matrix: $total combinations ($(wc -w <<<"$THEMES") themes, pages: $PAGES, dpr: $DPRS, dpi: $DPIS, scenarios: $SCENARIOS, units: $UNITS, font pt: $FONTPTS)"
[[ -n "$DRY" ]] && exit 0

mkdir -p "$OUT/shots" "$OUT/logs" "$OUT/raw"
OUT="$(cd "$OUT" && pwd)"
MANIFEST="$OUT/manifest.tsv"
printf 'name\ttheme\tpage\tdpr\tdpi\tscen\tunits\tfontpt\n' > "$MANIFEST"

lane_run() {
    local lane="$1" index=0 record theme page dpr dpi scen units fpt name tag
    local font_env=()
    for record in "${combos[@]}"; do
        if (( index % JOBS == lane )); then
            IFS=$'\t' read -r theme page dpr dpi scen units fpt <<<"$record"
            name="${theme}__${page}__dpr${dpr}__dpi${dpi}__${scen}__gu${units}__pt${fpt}"
            tag="$name"
            font_env=()
            if [[ "$fpt" != none ]]; then
                font_env=("SHOT_QPA_THEME=kde" "SHOT_FONT=Inter:$fpt" "SHOT_FIXED_FONT=Hack:$fpt" "SHOT_SMALL_FONT=Inter Display:9")
            fi
            env ${font_env[@]+"${font_env[@]}"} \
            SHOT_SERVERNUM=$(( ${SHOT_SERVERNUM:-80} + lane )) \
            SHOT_POPUP_W="${SHOT_POPUP_W:-900}" SHOT_POPUP_H="${SHOT_POPUP_H:-1000}" \
            SHOT_SCREEN="${SHOT_SCREEN:-3840x2160}" \
                "$HERE/shot.sh" --out "$OUT/raw" --tag "$tag" --themes "$theme" --views popup \
                    --scales "$dpr" --scenario "$scen" --audit --font-dpi "$dpi" --units "$units" \
                    --config "tab=$page" > "$OUT/raw/$tag.stdout" 2>&1 || true
            local base="$OUT/raw/${tag}_${theme}_popup_${dpr}"
            [[ -f "$base.png" ]] && mv "$base.png" "$OUT/shots/$name.png"
            [[ -f "$base.log" ]] && mv "$base.log" "$OUT/logs/$name.log"
            [[ -f "$base.requests.jsonl" ]] && rm -f "$base.requests.jsonl"
            [[ -f "$base.regions.json" ]] && rm -f "$base.regions.json"
            [[ -f "$OUT/raw/$tag.stdout" ]] && rm -f "$OUT/raw/$tag.stdout"
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$theme" "$page" "$dpr" "$dpi" "$scen" "$units" "$fpt" >> "$OUT/manifest.lane$lane"
        fi
        index=$((index + 1))
    done
}

started=$(date +%s)
for (( lane = 0; lane < JOBS; lane++ )); do
    rm -f "$OUT/manifest.lane$lane"
    lane_run "$lane" &
done
wait
for (( lane = 0; lane < JOBS; lane++ )); do
    [[ -f "$OUT/manifest.lane$lane" ]] && cat "$OUT/manifest.lane$lane" >> "$MANIFEST" && rm -f "$OUT/manifest.lane$lane"
done
echo "rendered $total combinations in $(( $(date +%s) - started )) s into $OUT"

python3 "$HERE/overflow_summarize.py" --run "$OUT" --package "$ROOT/package"
