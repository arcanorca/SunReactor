#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

build_dir=${QMLLINT_BUILD_DIR:-"$root/build/plasma"}
qml_dir=${QMLLINT_QML_DIR:-/usr/lib/qt6/qml}
qmllint=${QMLLINT:-/usr/lib/qt6/bin/qmllint}
source_dir=plasma/org.sunreactor.plasma/package/contents
baseline_file="$root/scripts/qmllint.baseline"

command -v jq >/dev/null || { echo "qmllint: jq is required" >&2; exit 2; }
[[ -x $qmllint ]] || { echo "qmllint: executable not found: $qmllint" >&2; exit 2; }
for file in "$build_dir/qml/org/sunreactor/plasma/qmldir" \
    "$build_dir/qml/org/sunreactor/plasma/sunreactorplugin.qmltypes" \
    "$build_dir/qml/org/sunreactor/plasma/libsunreactorpluginplugin.so" \
    "$qml_dir/QtQuick/qmldir" "$qml_dir/org/kde/plasma/plasmoid/qmldir" \
    "$qml_dir/org/kde/kirigami/qmldir" "$qml_dir/org/kde/kcmutils/qmldir"; do
    [[ -f $file ]] || { echo "qmllint: required module file missing: $file" >&2; exit 2; }
done
[[ -d $source_dir && -f $baseline_file ]] || { echo "qmllint: sources or baseline missing" >&2; exit 2; }
baseline=$(<"$baseline_file")
[[ $baseline =~ ^[0-9]+$ ]] || { echo "qmllint: invalid baseline" >&2; exit 2; }

manifest=$(mktemp "${TMPDIR:-/tmp}/sunreactor-qmlfiles.XXXXXX")
report=$(mktemp "${TMPDIR:-/tmp}/sunreactor-qmllint.XXXXXX")
trap 'rm -f -- "$manifest" "$report"' EXIT
find "$source_dir" -type f \( -name '*.qml' -o -name '*.js' \) -print0 | LC_ALL=C sort -z > "$manifest"
mapfile -d '' files < "$manifest"
(( ${#files[@]} > 0 )) || { echo "qmllint: no source files found" >&2; exit 2; }

if ! "$qmllint" --ignore-settings --json "$report" \
    -I "$build_dir/qml" -I "$qml_dir" --unused-imports warning "${files[@]}"; then
    echo "qmllint: analysis failed" >&2
    [[ ! -s $report ]] || jq -r '.files[]? as $f | $f.warnings[]? | "\($f.filename):\(.line):\(.column): [\(.id)] \(.message)"' "$report" >&2 || true
    exit 2
fi

if ! jq -e --argjson expected "${#files[@]}" \
    '(.files | type == "array" and length == $expected) and all(.files[]; (.filename | type == "string") and (.warnings | type == "array"))' \
    "$report" >/dev/null; then
    echo "qmllint: incomplete or invalid JSON report" >&2
    exit 2
fi

count=$(jq '[.files[].warnings[]] | length' "$report")
hard_errors=$(jq '[.files[].warnings[] | select(.type == "error" or .id == "import" or .id == "unresolved-type" or .id == "missing-type" or (.id | startswith("syntax")))] | length' "$report")
echo "qmllint: $count diagnostics across ${#files[@]} files (baseline: $baseline)"
jq -r '[.files[].warnings[]] | group_by(.id)[] | "  \(.[0].id): \(length)"' "$report"
if (( hard_errors > 0 || count > baseline )); then
    jq -r '.files[] as $f | $f.warnings[] | "\($f.filename):\(.line):\(.column): [\(.id)] \(.message)"' "$report" >&2
    echo "qmllint: unresolved imports/types, syntax errors, or warning baseline exceeded" >&2
    exit 1
fi
