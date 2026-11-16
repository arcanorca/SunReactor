#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BASE_PATH="$PATH"

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

run_version_cases() {
    local label="$1" awk_binary="$2" fakebin required actual expected
    fakebin=$(mktemp -d)
    ln -s -- "$awk_binary" "$fakebin/awk"

    while IFS=' ' read -r required actual expected; do
        if [[ "$expected" == 0 ]]; then
            PATH="$fakebin:$BASE_PATH" SUNREACTOR_INSTALLER_LIBRARY=1 \
                bash -c 'source "$1"; version_at_least "$2" "$3"' \
                _ "$ROOT_DIR/install.sh" "$required" "$actual" || {
                rm -rf "$fakebin"
                fail "$label rejected $required >= $actual"
            }
        elif PATH="$fakebin:$BASE_PATH" SUNREACTOR_INSTALLER_LIBRARY=1 \
            bash -c 'source "$1"; version_at_least "$2" "$3"' \
            _ "$ROOT_DIR/install.sh" "$required" "$actual"; then
            rm -rf "$fakebin"
            fail "$label accepted $required >= $actual"
        fi
    done <<'CASES'
2.35 2.44 0
2.9 2.35 0
2.35 2.35 0
2.44 2.35 1
CASES

    rm -rf "$fakebin"
    printf 'version comparison (%s): PASS\n' "$label"
}

test_available_awk_implementations() {
    local ran=0 awk_binary busybox_binary
    if awk_binary=$(command -v gawk 2>/dev/null); then
        run_version_cases gawk "$awk_binary"
        ran=1
    else
        printf 'version comparison (gawk): SKIP (not installed)\n'
    fi
    if awk_binary=$(command -v mawk 2>/dev/null); then
        run_version_cases mawk "$awk_binary"
        ran=1
    else
        printf 'version comparison (mawk): SKIP (not installed)\n'
    fi
    if busybox_binary=$(command -v busybox 2>/dev/null); then
        run_version_cases 'busybox awk' "$busybox_binary"
        ran=1
    else
        printf 'version comparison (busybox awk): SKIP (not installed)\n'
    fi
    (( ran > 0 )) || fail 'no supported awk implementation is installed'
}

test_no_side_effects() {
    local option="$1" expected_status="$2" home fakebin marker stdout stderr status flag
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/network-called"
    stdout=$(mktemp)
    stderr=$(mktemp)

    for flag in curl wget; do
        cat >"$fakebin/$flag" <<'SH'
#!/bin/sh
printf '%s\n' "${0##*/}" >>"$INSTALL_TEST_NETWORK_MARKER"
exit 99
SH
        chmod +x "$fakebin/$flag"
    done

    set +e
    HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" INSTALL_TEST_NETWORK_MARKER="$marker" \
        PATH="$fakebin:$BASE_PATH" "$ROOT_DIR/install.sh" "$option" >"$stdout" 2>"$stderr"
    status=$?
    set -e

    [[ "$status" == "$expected_status" ]] || {
        cat "$stdout" "$stderr" >&2
        rm -rf "$home" "$fakebin" "$stdout" "$stderr"
        fail "$option exited $status, expected $expected_status"
    }
    [[ ! -e "$marker" ]] || {
        rm -rf "$home" "$fakebin" "$stdout" "$stderr"
        fail "$option invoked a network helper"
    }
    [[ -z "$(find "$home" -mindepth 1 -print -quit)" ]] || {
        find "$home" -mindepth 1 -print >&2
        rm -rf "$home" "$fakebin" "$stdout" "$stderr"
        fail "$option changed files under HOME"
    }

    if [[ "$option" == --help || "$option" == -h ]]; then
        grep -Fq 'Usage: install.sh [OPTIONS]' "$stdout" || fail "$option did not print usage"
        for flag in --uninstall --purge --no-service --with-plasma --no-plasma --dry-run; do
            grep -Fq -- "$flag" "$stdout" || fail "$option usage omitted $flag"
        done
    else
        grep -Fq 'Unknown option: --unknown-install-option' "$stderr" || fail 'unknown option error was not reported'
        grep -Fq 'Usage: install.sh [OPTIONS]' "$stderr" || fail 'unknown option usage was not printed'
    fi

    rm -rf "$home" "$fakebin" "$stdout" "$stderr"
    printf '%s side-effect test: PASS\n' "$option"
}

test_available_awk_implementations
test_no_side_effects --help 0
test_no_side_effects -h 0
test_no_side_effects --unknown-install-option 2
printf 'installer regression tests passed\n'
