#!/usr/bin/env bash
set -euo pipefail

readonly BASELINE="${GLIBC_BASELINE:-2.35}"
[[ $# -gt 0 ]] || { printf 'usage: %s <binary>...\n' "$0" >&2; exit 2; }

version_newer() {
    local candidate="$1" baseline="$2"
    [[ "$(printf '%s\n%s\n' "$baseline" "$candidate" | sort -V | tail -n1)" == "$candidate" && "$candidate" != "$baseline" ]]
}

for binary in "$@"; do
    [[ -x $binary ]] || { printf 'not an executable: %s\n' "$binary" >&2; exit 2; }
    maximum=$(objdump -T "$binary" \
        | grep -o 'GLIBC_[0-9.]*' \
        | sed 's/^GLIBC_//' \
        | sort -Vu \
        | tail -n1)
    [[ -n $maximum ]] || { printf 'no GLIBC requirements found: %s\n' "$binary" >&2; exit 1; }
    printf '%s maximum required GLIBC: %s (baseline %s)\n' "$binary" "$maximum" "$BASELINE"
    if version_newer "$maximum" "$BASELINE"; then
        printf '%s requires GLIBC %s, newer than the declared %s baseline\n' "$binary" "$maximum" "$BASELINE" >&2
        exit 1
    fi
done
