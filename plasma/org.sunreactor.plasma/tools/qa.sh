#!/usr/bin/env bash
# The whole local QA of the widget, in order, with one compact summary:
#
#   1. build     cmake --build build (configures build/ first when it is missing)
#   2. ctest     every test the build registers (client, dot matrix, engine, picker)
#   3. qmllint   package/contents/ui with -I build/qml; fails on Error diagnostics.
#                The i18n "[unqualified]" notes are counted, never failing.
#   4. audit     tools/overflow_audit.sh --quick --jobs 2 over every theme; fails on
#                any OVERFLOW, CLIPPED or COLLISION, or on a render that did not finish
#   5. pixel     tools/pixel_proof.sh (the pixel themes at five device pixel ratios)
#
#   tools/qa.sh [--out DIR] [--nest]
#
#   6. nest      (only with --nest) tools/nest/check.py: the nested real-popup matrix,
#                4 themes x 2 fonts x stored/none x 2 panel edges, one private kwin and
#                plasmashell per case. Needs the Plasma 6 binaries; not in the default run.
#
# Every stage runs even when an earlier one fails; the build-dependent stages are
# skipped only when the build itself fails. The audits and the pixel proof run under
# flock on $SR_AUDIT_LOCK (default /tmp/sunreactor-1000/audit.lock) with SHOT_SERVERNUM=63
# unless the environment sets them. Exit status 0 only if every stage passes.
# DIR (default ~/.cache/sunreactor/qa/STAMP) keeps each stage's log and the audit report.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
BUILD="$ROOT/build"
QMLLINT="${QMLLINT:-/usr/lib/qt6/bin/qmllint}"
LOCK="${SR_AUDIT_LOCK:-/tmp/sunreactor-1000/audit.lock}"
export SHOT_SERVERNUM="${SHOT_SERVERNUM:-63}"
OUT=""
NEST=0

usage() { sed -n '2,24p' "$0"; }
while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --nest) NEST=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option $1" >&2; usage >&2; exit 2 ;;
    esac
done

STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${OUT:-${XDG_CACHE_HOME:-$HOME/.cache}/sunreactor/qa/$STAMP}"
mkdir -p "$OUT" "$(dirname "$LOCK")"
OUT="$(cd "$OUT" && pwd)"

NAMES=()
STATUS=()
DETAIL=()
record() {
    NAMES+=("$1")
    STATUS+=("$2")
    DETAIL+=("$3")
    echo "== $1: $2  ${3}"
}

stage_build() {
    if [[ ! -f "$BUILD/CMakeCache.txt" ]] && ! cmake -S "$ROOT" -B "$BUILD" > "$OUT/build.log" 2>&1; then
        record build FAIL "configure failed, see $OUT/build.log"
        return 1
    fi
    if cmake --build "$BUILD" -j "$(nproc)" >> "$OUT/build.log" 2>&1; then
        record build PASS "cmake --build ok"
        return 0
    fi
    record build FAIL "see $OUT/build.log"
    return 1
}

stage_ctest() {
    ctest --test-dir "$BUILD" --output-on-failure > "$OUT/ctest.log" 2>&1
    local rc=$?
    local summary
    summary="$(grep -E 'tests passed|tests failed' "$OUT/ctest.log" | tail -1 | sed 's/^ *//')"
    if [[ $rc -eq 0 ]]; then
        record ctest PASS "${summary:-all passed}"
    else
        record ctest FAIL "${summary:-ctest exit $rc}, see $OUT/ctest.log"
    fi
}

stage_lint() {
    mapfile -d '' files < <(find "$ROOT/package/contents/ui" -name '*.qml' -print0 | sort -z)
    "$QMLLINT" -I "$BUILD/qml" "${files[@]}" > "$OUT/qmllint.log" 2>&1
    local rc=$?
    # qmllint's exit status is not trusted (it is 255 whenever it reports anything in
    # some builds): the diagnostics decide, and a crash with no output still fails.
    local counts
    counts="$(python3 - "$OUT/qmllint.log" <<'PY'
import sys
errors = warnings = notes = 0
for line in open(sys.argv[1], errors="replace"):
    if "[unqualified]" in line:
        notes += 1
    elif line.startswith("Error:"):
        errors += 1
    elif line.startswith("Warning:"):
        warnings += 1
print(errors, warnings, notes)
PY
)"
    read -r errors warnings notes <<< "$counts"
    if [[ $errors -eq 0 && ( $rc -eq 0 || $rc -eq 255 || $((warnings + notes)) -gt 0 ) ]]; then
        record qmllint PASS "$errors errors, $warnings warnings, $notes [unqualified] notes"
    else
        record qmllint FAIL "$errors errors, $warnings warnings, qmllint exit $rc, see $OUT/qmllint.log"
    fi
}

stage_audit() {
    flock "$LOCK" "$HERE/overflow_audit.sh" --quick --jobs 2 --out "$OUT/audit" > "$OUT/audit.log" 2>&1
    local report="$OUT/audit/overflow_report.json"
    if [[ ! -f "$report" ]]; then
        record audit FAIL "no report, see $OUT/audit.log"
        return
    fi
    local verdict
    mapfile -t verdict < <(python3 - "$report" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
themes = r.get("themes", [])
bad = [f"{t['theme']} {t['OVERFLOW']}/{t['CLIPPED']}/{t['COLLISION']}" for t in themes
       if t.get("OVERFLOW") or t.get("CLIPPED") or t.get("COLLISION")]
inc = len(r.get("incomplete", []))
totals = {k: sum(t.get(k, 0) for t in themes) for k in ("OVERFLOW", "CLIPPED", "COLLISION")}
detail = (f"{len(themes)} themes, {r.get('combinations', 0)} renders: {totals['OVERFLOW']} OVERFLOW, "
          f"{totals['CLIPPED']} CLIPPED, {totals['COLLISION']} COLLISION")
if bad:
    detail += "; defects in " + ", ".join(bad)
if inc:
    detail += f"; {inc} renders did not finish"
print("FAIL" if bad or inc else "PASS")
print(detail)
PY
)
    record audit "${verdict[0]:-FAIL}" "${verdict[1]:-no verdict}; report $OUT/audit/overflow_report.md"
}

stage_nest() {
    python3 "$ROOT/tools/nest/check.py" --out "$OUT/nest" > "$OUT/nest.log" 2>&1
    local rc=$?
    local summary
    summary="$(grep -E 'cases pass' "$OUT/nest.log" | tail -1)"
    if [[ $rc -eq 0 ]]; then
        record nest PASS "${summary:-all cases pass}"
    else
        record nest FAIL "${summary:-no summary, nest exit $rc}, see $OUT/nest.log"
    fi
}

stage_pixel() {
    # The pixel-renderer themes only: nothing is a vector theme and its surfaces are
    # antialiased by design, so the palette proof does not apply to it.
    local themes="handheld commodore64"
    flock "$LOCK" "$HERE/pixel_proof.sh" --out "$OUT/pixel" --themes "$themes" > "$OUT/pixel.log" 2>&1
    local rc=$?
    if [[ $rc -eq 0 ]]; then
        record pixel PASS "$themes at 5 device pixel ratios"
    else
        record pixel FAIL "pixel proof exit $rc, see $OUT/pixel.log"
    fi
}

if stage_build; then
    BUILT=1
else
    BUILT=0
fi
if [[ $BUILT -eq 1 ]]; then
    stage_ctest
else
    record ctest SKIP "build failed"
fi
stage_lint
if [[ $BUILT -eq 1 ]]; then
    stage_audit
    stage_pixel
    if [[ $NEST -eq 1 ]]; then
        stage_nest
    fi
else
    record audit SKIP "build failed"
    record pixel SKIP "build failed"
fi

overall=PASS
echo
echo "SunReactor widget QA  (logs: $OUT)"
for i in "${!NAMES[@]}"; do
    printf '  %-8s %-4s  %s\n' "${NAMES[$i]}" "${STATUS[$i]}" "${DETAIL[$i]}"
    [[ "${STATUS[$i]}" == PASS ]] || overall=FAIL
done
echo "overall: $overall"
[[ "$overall" == PASS ]]
