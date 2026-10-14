#!/usr/bin/env bash
# Proves the pixel renderer: tools/harness/PixelProof.qml (engine-only test
# surface, real SunReactorClient against the fake daemon) under Xvfb at each
# device pixel ratio, screen capture, palette conformance per reported region.
#
#   tools/pixel_proof.sh --out DIR [--themes "handheld commodore64 nothing"]
#                        [--scales "1.0 1.25 1.35 1.5 2.0"] [--qml DIR]
#
# Exit status 0 only if every region passes and, at fractional ratios, every
# negative control ("control_*": QML Text inside an art-resolution layer)
# fails - a checker that cannot see the failure proves nothing. At integer
# ratios Qt's own text happens to be exact for grid fonts, so the control may
# pass there.
# Same isolation as shot.sh: env -i, own Xvfb and D-Bus, private XDG dirs.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
QMLDIR="$ROOT/build/qml"
OUT="${SHOT_OUT:-}"
THEMES="handheld commodore64 nothing"
SCALES="1.0 1.25 1.35 1.5 2.0"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --themes) THEMES="$2"; shift 2 ;;
        --scales) SCALES="$2"; shift 2 ;;
        --qml) QMLDIR="$(cd "$2" && pwd)"; shift 2 ;;
        -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
[[ -n "$OUT" ]] || { echo "--out DIR is required" >&2; exit 2; }
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

RUNPARENT="${SHOT_RUNTIME_PARENT:-${TMPDIR:-/tmp}}"
WORK="$(mktemp -d "$RUNPARENT/srshot.XXXXXX")"
cleanup() {
    [[ -n "${FAKE_PID:-}" ]] && kill "$FAKE_PID" 2>/dev/null || true
    [[ -n "$WORK" && -d "$WORK" && "$(basename "$WORK")" == srshot.* ]] && rm -rf -- "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/run" "$WORK/config" "$WORK/data" "$WORK/cache"
chmod 700 "$WORK/run"
SOCK="$WORK/run/sunreactor/control.sock"

status=0
summary=()
for theme in $THEMES; do
    for scale in $SCALES; do
        name="proof_${theme}_${scale}"
        python3 "$HERE/fake_daemon.py" --socket "$SOCK" --theme "$theme" > "$WORK/fake.out" 2>&1 &
        FAKE_PID=$!
        for _ in $(seq 50); do [[ -S "$SOCK" ]] && break; sleep 0.1; done
        log="$OUT/$name.log"
        cat > "$WORK/inner.sh" <<EOF
qml6 "$HERE/harness/PixelProof.qml" > "$log" 2>&1 &
P=\$!
for i in \$(seq 100); do grep -q SUNREACTOR_READY "$log" 2>/dev/null && break; sleep 0.1; done
sleep 0.3
import -window root "$WORK/root.png"
kill \$P 2>/dev/null; wait \$P 2>/dev/null
EOF
        env -i HOME="$HOME" PATH="/usr/bin:/usr/sbin:/bin" LANG="en_US.UTF-8" TZ="Europe/Istanbul" \
            XDG_RUNTIME_DIR="$WORK/run" XDG_CONFIG_HOME="$WORK/config" XDG_DATA_HOME="$WORK/data" \
            XDG_CACHE_HOME="$WORK/cache" QML_IMPORT_PATH="$QMLDIR" QT_SCALE_FACTOR="$scale" \
            QT_QPA_PLATFORM=xcb SUNREACTOR_HARNESS=1 QT_FORCE_STDERR_LOGGING=1 \
            dbus-run-session -- xvfb-run -a -s "-screen 0 1400x1000x24" sh "$WORK/inner.sh" \
            > "$WORK/xvfb.out" 2>&1 || true
        kill "$FAKE_PID" 2>/dev/null || true; wait "$FAKE_PID" 2>/dev/null || true; FAKE_PID=""

        w=$(python3 -c "print(round(520*$scale))"); h=$(python3 -c "print(round(360*$scale))")
        python3 -c "from PIL import Image; Image.open('$WORK/root.png').crop((0,0,$w,$h)).save('$OUT/$name.png')"
        # The last report of each region wins (layout may settle late).
        python3 - "$log" "$OUT/$name.regions.json" <<'PY'
import json, sys
regions = {}
for line in open(sys.argv[1], errors="replace"):
    if "SUNREACTOR_PIXEL_REGION " in line:
        r = json.loads(line.split("SUNREACTOR_PIXEL_REGION ", 1)[1])
        regions[r["name"]] = r
json.dump(list(regions.values()), open(sys.argv[2], "w"), indent=1)
PY
        echo "== $name  $(grep -o 'SUNREACTOR_THEME .*' "$log" | cut -c18- || true)"
        if ! grep -q '"surface"' "$OUT/$name.regions.json" || ! grep -q '"control_' "$OUT/$name.regions.json"; then
            echo "  FAIL: the harness reported no regions (QML did not load?)"; tail -5 "$log"; status=1
        fi
        while read -r region; do
            rname=$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['name'])" "$region")
            echo "[$region]" > "$WORK/one.json"
            if python3 "$HERE/conformance.py" "$OUT/$name.png" --theme "$theme" --regions "$WORK/one.json" \
                    --diff "$OUT/${name}_${rname}.diff.png" --quiet > "$WORK/conf.out"; then
                result=PASS
            else
                result=FAIL
            fi
            line="$(head -1 "$WORK/conf.out")"
            fractional=$(python3 -c "print(int(abs($scale - round($scale)) > 1e-6))")
            if [[ "$rname" == control_* ]]; then
                if [[ "$result" == FAIL ]]; then echo "  ok (control fails as it must): $line"
                elif [[ "$fractional" == 0 ]]; then echo "  ok (control may pass at integer DPR): $line"
                else echo "  BROKEN CONTROL (should fail): $line"; status=1; fi
            else
                echo "  $line"
                [[ "$result" == PASS ]] || status=1
            fi
            [[ "$result" == FAIL && "$rname" != control_* ]] && sed -n '2,4p' "$WORK/conf.out"
            [[ "$result" == PASS ]] && rm -f "$OUT/${name}_${rname}.diff.png"
        done < <(python3 -c "import json,sys; [print(json.dumps(r)) for r in json.load(open(sys.argv[1]))]" "$OUT/$name.regions.json")
        grep -E "Warning|Error|error:" "$log" | grep -v "QML debugging" | head -5 || true
    done
done
echo
[[ $status -eq 0 ]] && echo "PIXEL PROOF: PASS" || echo "PIXEL PROOF: FAIL"
exit $status
