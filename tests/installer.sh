#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPO_ROOT
TESTS_RUN=0

fail() { printf 'installer test failed: %s\n' "$*" >&2; exit 1; }
assert_contains() { [[ $1 == *"$2"* ]] || fail "expected output to contain '$2'"; }

make_tools() {
    local root="$1"
    mkdir -p "$root/tools"
    cat > "$root/tools/systemctl" <<'EOF'
#!/usr/bin/env bash
set -eu
case "$*" in
  *is-active*) exit 1 ;;
  *"enable --now"*) [[ ${INSTALLER_FAIL_STEP:-} != service ]] ;;
  *) exit 0 ;;
esac
EOF
    cat > "$root/tools/systemd-analyze" <<'EOF'
#!/usr/bin/env bash
set -eu
unit="${@: -1}"
grep -q "ExecStart=$SUNREACTOR_BINDIR/sunreactord" "$unit"
grep -q "ExecReload=$SUNREACTOR_BINDIR/sunreactorctl reload-config" "$unit"
! grep -q '@BINDIR@' "$unit"
EOF
    chmod +x "$root/tools/systemctl" "$root/tools/systemd-analyze"
}

make_release() {
    local root="$1" mode="${2:-ok}" version="test-v1"
    local payload="$root/payload" release="$root/release"
    mkdir -p "$payload" "$release"
    if [[ $mode == glibc ]]; then
        cat > "$payload/sunreactorctl" <<'EOF'
#!/usr/bin/env bash
echo "sunreactorctl: /lib/x86_64-linux-gnu/libc.so.6: version \`GLIBC_2.39' not found" >&2
exit 1
EOF
    else
        cat > "$payload/sunreactorctl" <<'EOF'
#!/usr/bin/env bash
set -eu
case "${1:-}" in
  --version) echo 'sunreactorctl test' ;;
  config)
    if [[ ${2:-} == init ]]; then
      mkdir -p "$XDG_CONFIG_HOME/sunreactor"
      printf '[daemon]\ntick_seconds = 60\n' > "$XDG_CONFIG_HOME/sunreactor/config.toml"
    fi
    ;;
  ping)
    [[ ${INSTALLER_FAIL_STEP:-} != ipc ]] || exit 1
    if [[ ${INSTALLER_PING_FAILS:-0} -gt 0 ]]; then
      attempts=0
      if [[ -f "$INSTALLER_PING_STATE_FILE" ]]; then
        attempts=$(<"$INSTALLER_PING_STATE_FILE")
      fi
      if [[ $attempts -lt ${INSTALLER_PING_FAILS:-0} ]]; then
        printf '%s' "$((attempts + 1))" > "$INSTALLER_PING_STATE_FILE"
        exit 1
      fi
    fi
    ;;
  doctor)
    if [[ ${INSTALLER_DOCTOR:-ok} == relogin ]]; then
      echo '{"blocking_errors":1,"i2c_access":"I2C_GROUP_CONFIGURED_BUT_SESSION_STALE"}'
    elif [[ ${INSTALLER_DOCTOR:-ok} == blocked ]]; then
      echo '{"blocking_errors":1,"i2c_access":"DEVICE_EXISTS_BUT_PERMISSION_DENIED"}'
    else
      echo '{"blocking_errors":0,"i2c_access":"ACCESS_GRANTED_BY_UACCESS"}'
    fi
    ;;
  discover)
    if [[ -n ${INSTALLER_DISCOVER_LOG:-} ]]; then
      printf '%s\n' "$*" > "$INSTALLER_DISCOVER_LOG"
    fi
    [[ ${INSTALLER_DISCOVER:-ok} != fail ]] || exit 1
    if [[ ${INSTALLER_DISCOVER:-ok} != no_monitors ]]; then
      : > "$INSTALLER_DISCOVERY_APPLIED_FILE"
      echo 'added monitor(s): internal'
    fi
    ;;
  status)
    monitors=0
    if [[ -f ${INSTALLER_DISCOVERY_APPLIED_FILE:-/nonexistent} ]]; then
      monitors=${INSTALLER_MONITORS:-1}
    fi
    echo "configured_monitors: $monitors"
    ;;
esac
EOF
    fi
    cat > "$payload/sunreactord" <<'EOF'
#!/usr/bin/env bash
[[ ${1:-} == --help ]] && { echo 'sunreactord test help'; exit 0; }
exit 1
EOF
    cp "$REPO_ROOT/contrib/systemd/sunreactord.service" "$payload/sunreactord.service"
    chmod +x "$payload/sunreactord" "$payload/sunreactorctl"
    local archive="$release/sunreactor-${version}-linux-x86_64-gnu.tar.gz"
    tar -czf "$archive" -C "$payload" sunreactord sunreactorctl sunreactord.service
    sha256sum "$archive" > "$archive.sha256"
}

run_case() {
    local name="$1" expected_state="$2" expected_rc="$3" setup="$4"
    shift 4
    local root output rc
    root=$(mktemp -d)
    make_tools "$root"
    "$setup" "$root"
    set +e
    output=$(env \
        HOME="$root/home" \
        XDG_CONFIG_HOME="$root/home/.config" \
        SUNREACTOR_VERSION=test-v1 \
        SUNREACTOR_RELEASE_DIR="$root/release" \
        SUNREACTOR_BINDIR="$root/home/.local/bin" \
        SUNREACTOR_UNITDIR="$root/home/.config/systemd/user" \
        SUNREACTOR_SYSTEMCTL="$root/tools/systemctl" \
        SUNREACTOR_SYSTEMD_ANALYZE="$root/tools/systemd-analyze" \
        SUNREACTOR_IPC_READY_ATTEMPTS=3 \
        INSTALLER_PING_STATE_FILE="$root/ping-attempts" \
        INSTALLER_DISCOVERY_APPLIED_FILE="$root/discovery-applied" \
        INSTALLER_DISCOVER_LOG="$root/discover-args" \
        "$@" \
        bash "$REPO_ROOT/install.sh" --quiet 2>&1)
    rc=$?
    set -e
    [[ $rc -eq $expected_rc ]] || fail "$name returned $rc, expected $expected_rc: $output"
    assert_contains "$output" "SUNREACTOR_RESULT=$expected_state"
    if [[ $expected_rc -eq 0 ]]; then
        grep -qx 'discover --apply' "$root/discover-args" \
            || fail "$name did not invoke automatic monitor discovery"
    fi
    rm -rf "$root"
    TESTS_RUN=$((TESTS_RUN + 1))
}

setup_ok() { mkdir -p "$1/home"; make_release "$1"; }
setup_glibc() { mkdir -p "$1/home"; make_release "$1" glibc; }
setup_missing() { mkdir -p "$1/home" "$1/release"; }
setup_checksum() {
    setup_ok "$1"
    printf '%064d  bad\n' 0 > "$1/release/sunreactor-test-v1-linux-x86_64-gnu.tar.gz.sha256"
}
setup_existing() {
    setup_ok "$1"
    mkdir -p "$1/home/.local/bin" "$1/home/.config/systemd/user"
    printf '#!/bin/sh\necho old-daemon\n' > "$1/home/.local/bin/sunreactord"
    printf '#!/bin/sh\necho old-cli\n' > "$1/home/.local/bin/sunreactorctl"
    printf '# old unit\n' > "$1/home/.config/systemd/user/sunreactord.service"
    chmod +x "$1/home/.local/bin/sunreactord" "$1/home/.local/bin/sunreactorctl"
}

run_case automatic_discovery SUCCESS 0 setup_ok
run_case no_monitors SUCCESS_NO_MONITORS_CONFIGURED 0 setup_ok INSTALLER_DISCOVER=no_monitors
run_case compatible_with_monitors SUCCESS 0 setup_ok INSTALLER_MONITORS=2
run_case glibc BINARY_INCOMPATIBLE 1 setup_glibc
run_case missing_asset SOURCE_BUILD_REQUIRED 1 setup_missing
run_case checksum BINARY_INCOMPATIBLE 1 setup_checksum
run_case doctor_block DEPENDENCY_FAILURE 1 setup_ok INSTALLER_DOCTOR=blocked
run_case relogin RELOGIN_REQUIRED 1 setup_ok INSTALLER_DOCTOR=relogin
run_case ipc_readiness_wait SUCCESS 0 setup_ok INSTALLER_PING_FAILS=2
run_case ipc_failure IPC_FAILURE 1 setup_ok INSTALLER_FAIL_STEP=ipc
run_case discovery_failure DISCOVERY_FAILURE 1 setup_ok INSTALLER_DISCOVER=fail

rollback_root=$(mktemp -d)
make_tools "$rollback_root"
setup_existing "$rollback_root"
set +e
rollback_output=$(env \
    HOME="$rollback_root/home" XDG_CONFIG_HOME="$rollback_root/home/.config" \
    SUNREACTOR_VERSION=test-v1 SUNREACTOR_RELEASE_DIR="$rollback_root/release" \
    SUNREACTOR_BINDIR="$rollback_root/home/.local/bin" \
    SUNREACTOR_UNITDIR="$rollback_root/home/.config/systemd/user" \
    SUNREACTOR_SYSTEMCTL="$rollback_root/tools/systemctl" \
    SUNREACTOR_SYSTEMD_ANALYZE="$rollback_root/tools/systemd-analyze" \
    INSTALLER_FAIL_STEP=service bash "$REPO_ROOT/install.sh" --quiet 2>&1)
rollback_rc=$?
set -e
[[ $rollback_rc -eq 1 ]] || fail "rollback case unexpectedly succeeded"
assert_contains "$rollback_output" 'SUNREACTOR_RESULT=SERVICE_FAILURE'
grep -q old-daemon "$rollback_root/home/.local/bin/sunreactord" || fail 'old daemon was not restored'
grep -q old-cli "$rollback_root/home/.local/bin/sunreactorctl" || fail 'old CLI was not restored'
grep -q 'old unit' "$rollback_root/home/.config/systemd/user/sunreactord.service" || fail 'old unit was not restored'
rm -rf "$rollback_root"
TESTS_RUN=$((TESTS_RUN + 1))

uninstall_root=$(mktemp -d)
make_tools "$uninstall_root"
mkdir -p \
    "$uninstall_root/home/.local/bin" \
    "$uninstall_root/home/.config/systemd/user" \
    "$uninstall_root/home/.config/sunreactor" \
    "$uninstall_root/home/.local/state/sunreactor" \
    "$uninstall_root/home/.cache/sunreactor"
printf 'binary\n' > "$uninstall_root/home/.local/bin/sunreactorctl"
printf 'unit\n' > "$uninstall_root/home/.config/systemd/user/sunreactord.service"
printf 'config\n' > "$uninstall_root/home/.config/sunreactor/config.toml"
printf 'state\n' > "$uninstall_root/home/.local/state/sunreactor/runtime-state.json"
printf 'cache\n' > "$uninstall_root/home/.cache/sunreactor/weather.json"
uninstall_output=$(env \
    HOME="$uninstall_root/home" \
    XDG_CONFIG_HOME="$uninstall_root/home/.config" \
    XDG_STATE_HOME="$uninstall_root/home/.local/state" \
    XDG_CACHE_HOME="$uninstall_root/home/.cache" \
    SUNREACTOR_BINDIR="$uninstall_root/home/.local/bin" \
    SUNREACTOR_UNITDIR="$uninstall_root/home/.config/systemd/user" \
    SUNREACTOR_SYSTEMCTL="$uninstall_root/tools/systemctl" \
    SUNREACTOR_SYSTEMD_ANALYZE="$uninstall_root/tools/systemd-analyze" \
    bash "$REPO_ROOT/install.sh" --quiet --uninstall 2>&1)
assert_contains "$uninstall_output" 'SUNREACTOR_RESULT=SUCCESS'
[[ ! -e "$uninstall_root/home/.config/sunreactor" ]] || fail 'uninstall left configuration behind'
[[ ! -e "$uninstall_root/home/.local/state/sunreactor" ]] || fail 'uninstall left state behind'
[[ ! -e "$uninstall_root/home/.cache/sunreactor" ]] || fail 'uninstall left cache behind'
[[ ! -e "$uninstall_root/home/.local/bin/sunreactorctl" ]] || fail 'uninstall left CLI behind'
rm -rf "$uninstall_root"
TESTS_RUN=$((TESTS_RUN + 1))

uninstall_without_systemd_root=$(mktemp -d)
mkdir -p "$uninstall_without_systemd_root/home/.config/sunreactor"
printf 'config\n' > "$uninstall_without_systemd_root/home/.config/sunreactor/config.toml"
uninstall_without_systemd_output=$(env \
    HOME="$uninstall_without_systemd_root/home" \
    XDG_CONFIG_HOME="$uninstall_without_systemd_root/home/.config" \
    SUNREACTOR_SYSTEMCTL="$uninstall_without_systemd_root/tools/missing-systemctl" \
    SUNREACTOR_SYSTEMD_ANALYZE="$uninstall_without_systemd_root/tools/missing-systemd-analyze" \
    bash "$REPO_ROOT/install.sh" --quiet --uninstall 2>&1)
assert_contains "$uninstall_without_systemd_output" 'SUNREACTOR_RESULT=SUCCESS'
[[ ! -e "$uninstall_without_systemd_root/home/.config/sunreactor" ]] \
    || fail 'uninstall required unavailable systemd tools'
rm -rf "$uninstall_without_systemd_root"
TESTS_RUN=$((TESTS_RUN + 1))

printf 'installer tests: %d passed\n' "$TESTS_RUN"
