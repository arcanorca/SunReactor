#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

SUNREACTOR_INSTALLER_LIBRARY=1 source "$ROOT_DIR/install.sh"

assert_contains() {
    local haystack="$1" needle="$2"
    [[ "$haystack" == *"$needle"* ]] || {
        printf 'expected output to contain: %s\n' "$needle" >&2
        exit 1
    }
}

rendered_unit() {
    local home="$1" config="$2" state="$3" cache="$4"
    HOME="$home" XDG_CONFIG_HOME="$config" XDG_STATE_HOME="$state" XDG_CACHE_HOME="$cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; render_service_unit /dev/stdout' _ "$ROOT_DIR/install.sh"
}

test_default_paths_and_unit() {
    local home unit
    home=$(mktemp -d)
    unit=$(HOME="$home" env -u XDG_CONFIG_HOME -u XDG_STATE_HOME -u XDG_CACHE_HOME \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; render_service_unit /dev/stdout' _ "$ROOT_DIR/install.sh")
    assert_contains "$unit" "Documentation=https://github.com/arcanorca/SunReactor"
    assert_contains "$unit" "Environment=XDG_CONFIG_HOME=$home/.config"
    assert_contains "$unit" "Environment=XDG_STATE_HOME=$home/.local/state"
    assert_contains "$unit" "Environment=XDG_CACHE_HOME=$home/.cache"
    assert_contains "$unit" "ExecStart=\"$home/.local/bin/sunreactord\""
    assert_contains "$unit" "ExecReload=\"$home/.local/bin/sunreactorctl\" reload-config"
    assert_contains "$unit" "WantedBy=default.target"
    assert_contains "$unit" "NoNewPrivileges=true"
    assert_contains "$unit" "MemoryDenyWriteExecute=true"
    [[ "$unit" != *'@CONFIG_HOME@'* ]]
    [[ "$unit" != *'/usr/local/bin/'* ]]
    rm -rf "$home"
}

test_custom_paths_and_uninstall_paths() {
    local home unit
    home=$(mktemp -d)
    unit=$(rendered_unit "$home" /tmp/custom-config /tmp/custom-state /tmp/custom-cache)
    assert_contains "$unit" "Documentation=https://github.com/arcanorca/SunReactor"
    assert_contains "$unit" "Environment=XDG_CONFIG_HOME=/tmp/custom-config"
    assert_contains "$unit" "Environment=XDG_STATE_HOME=/tmp/custom-state"
    assert_contains "$unit" "Environment=XDG_CACHE_HOME=/tmp/custom-cache"
    assert_contains "$unit" "ExecStart=\"$home/.local/bin/sunreactord\""
    assert_contains "$unit" "ExecReload=\"$home/.local/bin/sunreactorctl\" reload-config"

    HOME="$home" XDG_CONFIG_HOME=/tmp/custom-config XDG_STATE_HOME=/tmp/custom-state \
        XDG_CACHE_HOME=/tmp/custom-cache SUNREACTOR_INSTALLER_LIBRARY=1 \
        bash -c 'source "$1"; [[ "$SYSTEMD_DIR" == /tmp/custom-config/systemd/user ]]; [[ "$CFG_DIR" == /tmp/custom-config/sunreactor ]]; [[ "$STATE_DIR" == /tmp/custom-state/sunreactor ]]; [[ "$CACHE_DIR" == /tmp/custom-cache/sunreactor ]]' _ "$ROOT_DIR/install.sh"

    mkdir -p "$home/.local/bin" /tmp/custom-config/systemd/user /tmp/custom-config/sunreactor /tmp/custom-state/sunreactor /tmp/custom-cache/sunreactor
    touch "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl" \
        /tmp/custom-config/systemd/user/sunreactord.service \
        /tmp/custom-config/sunreactor/config.toml \
        /tmp/custom-state/sunreactor/runtime-state.json \
        /tmp/custom-cache/sunreactor/cache-entry
    SYSTEMCTL_MARKER="$home/systemctl-called" PATH="/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME=/tmp/custom-config XDG_STATE_HOME=/tmp/custom-state XDG_CACHE_HOME=/tmp/custom-cache \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; QUIET=1; uninstall_sunreactor' _ "$ROOT_DIR/install.sh" >/dev/null 2>&1
    [[ ! -e "$home/.local/bin/sunreactord" && ! -e "$home/.local/bin/sunreactorctl" ]]
    [[ ! -e /tmp/custom-config/systemd/user/sunreactord.service ]]
    # Standard uninstall MUST preserve user configuration and runtime state
    [[ -e /tmp/custom-config/sunreactor/config.toml ]]
    [[ -e /tmp/custom-state/sunreactor/runtime-state.json ]]

    # Purge uninstall MUST remove user configuration and runtime state
    SYSTEMCTL_MARKER="$home/systemctl-called" PATH="/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME=/tmp/custom-config XDG_STATE_HOME=/tmp/custom-state XDG_CACHE_HOME=/tmp/custom-cache \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; QUIET=1; PURGE=1; uninstall_sunreactor' _ "$ROOT_DIR/install.sh" >/dev/null 2>&1
    [[ ! -e /tmp/custom-config/sunreactor && ! -e /tmp/custom-state/sunreactor && ! -e /tmp/custom-cache/sunreactor ]]
    rm -rf "$home" /tmp/custom-config /tmp/custom-state /tmp/custom-cache
}

test_invalid_xdg_path_is_rejected() {
    local home
    home=$(mktemp -d)
    if HOME="$home" XDG_CONFIG_HOME=relative \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; validate_xdg_paths' _ "$ROOT_DIR/install.sh" 2>/dev/null; then
        printf 'relative XDG_CONFIG_HOME was accepted\n' >&2
        exit 1
    fi
    rm -rf "$home"
}

test_no_service_and_unavailable_manager() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-called"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' called >"$SYSTEMCTL_MARKER"
exit 1
SHIM
    chmod +x "$fakebin/systemctl"
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; parse_args --no-service; [[ "$NO_SERVICE" == 1 ]]' _ "$ROOT_DIR/install.sh"
    [[ ! -e "$marker" ]] || {
        printf 'no-service mode unexpectedly invoked systemctl\n' >&2
        exit 1
    }
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; ! systemd_user_manager_available' _ "$ROOT_DIR/install.sh"
    [[ -e "$marker" ]] || {
        printf 'systemd capability probe did not invoke systemctl\n' >&2
        exit 1
    }
    rm -rf "$home" "$fakebin"
}

test_systemd_setup_failure_restores_previous_installation() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
case "$*" in
    "--user show-environment"|"--user daemon-reload"|"--user start sunreactord.service"|"--user stop sunreactord.service") exit 0 ;;
    "--user is-active --quiet sunreactord.service") exit 1 ;;
    "--user is-enabled --quiet sunreactord.service") exit 0 ;;
    "--user enable sunreactord.service")
        if [[ ! -e "$SYSTEMCTL_MARKER.enable-failed" ]]; then
            touch "$SYSTEMCTL_MARKER.enable-failed"
            exit 1
        fi
        exit 0
        ;;
    "--user show -p UnitPath --value") printf '%s\n' "$HOME/config/systemd/user" ;;
    "--user show --property=FragmentPath --value sunreactord.service") printf '%s\n' "$HOME/config/systemd/user/sunreactord.service" ;;
    *) exit 0 ;;
esac
SHIM
    chmod +x "$fakebin/systemctl"
    mkdir -p "$home/.local/bin" "$home/config/systemd/user"
    printf 'old daemon\n' >"$home/.local/bin/sunreactord"
    printf 'old CLI\n' >"$home/.local/bin/sunreactorctl"
    printf 'old service\n' >"$home/config/systemd/user/sunreactord.service"
    chmod 755 "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl"
    set +e
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        printf 'service failure was reported as success\n' >&2
        exit 1
    fi
    grep -Fx 'old daemon' "$home/.local/bin/sunreactord" >/dev/null
    grep -Fx 'old CLI' "$home/.local/bin/sunreactorctl" >/dev/null
    grep -Fx 'old service' "$home/config/systemd/user/sunreactord.service" >/dev/null
    [[ -e "$marker" ]]
    grep -Fx -- '--user show-environment' "$marker" >/dev/null
    grep -Fx -- '--user enable sunreactord.service' "$marker" >/dev/null
    if ! grep -F 'previous installation was restored' "$home/installer.log" >/dev/null; then
        printf 'rollback error message was not emitted\n' >&2
        cat "$home/installer.log" >&2
        exit 1
    fi
    rm -rf "$home" "$fakebin"
}

test_daemon_readiness_failure_restores_previous_installation() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
case "$*" in
    "--user show-environment"|"--user daemon-reload"|"--user cat sunreactord.service") exit 0 ;;
    "--user is-active --quiet sunreactord.service") [[ -e "$SYSTEMCTL_MARKER.active" ]] ;;
    "--user is-enabled --quiet sunreactord.service") [[ -e "$SYSTEMCTL_MARKER.enabled" ]] ;;
    "--user show -p UnitPath --value") printf '%s\n' "$HOME/config/systemd/user" ;;
    "--user show --property=FragmentPath --value sunreactord.service") printf '%s\n' "$HOME/config/systemd/user/sunreactord.service" ;;
    "--user enable sunreactord.service") touch "$SYSTEMCTL_MARKER.enabled" ;;
    "--user disable sunreactord.service") rm -f "$SYSTEMCTL_MARKER.enabled" ;;
    "--user start sunreactord.service"|"--user restart sunreactord.service") touch "$SYSTEMCTL_MARKER.active" ;;
    "--user stop sunreactord.service") rm -f "$SYSTEMCTL_MARKER.active" ;;
    *) exit 0 ;;
esac
SHIM
    chmod +x "$fakebin/systemctl"
    mkdir -p "$home/.local/bin" "$home/config/systemd/user"
    printf 'old daemon\n' >"$home/.local/bin/sunreactord"
    printf 'old CLI\n' >"$home/.local/bin/sunreactorctl"
    printf 'old service\n' >"$home/config/systemd/user/sunreactord.service"
    chmod 755 "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl"

    set +e
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_TEST_DAEMON_ALIVE=false SUNREACTOR_IPC_READY_ATTEMPTS=1 \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    local status=$?
    set -e

    [[ $status -ne 0 ]]
    grep -Fx 'old daemon' "$home/.local/bin/sunreactord" >/dev/null
    grep -Fx 'old CLI' "$home/.local/bin/sunreactorctl" >/dev/null
    grep -Fx 'old service' "$home/config/systemd/user/sunreactord.service" >/dev/null
    grep -Fx -- '--user start sunreactord.service' "$marker" >/dev/null
    grep -Fx -- '--user stop sunreactord.service' "$marker" >/dev/null
    grep -Fx -- '--user disable sunreactord.service' "$marker" >/dev/null
    grep -F 'did not become ready on IPC' "$home/installer.log" >/dev/null
    grep -F 'previous installation was restored' "$home/installer.log" >/dev/null
    rm -rf "$home" "$fakebin"
}

test_incomplete_rollback_retains_recovery_files() {
    local home fakebin marker status recovery_dir recovery_root
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
case "$*" in
    "--user show-environment"|"--user daemon-reload"|"--user cat sunreactord.service") exit 0 ;;
    "--user is-active --quiet sunreactord.service") [[ -e "$SYSTEMCTL_MARKER.active" ]] ;;
    "--user is-enabled --quiet sunreactord.service") [[ -e "$SYSTEMCTL_MARKER.enabled" ]] ;;
    "--user show -p UnitPath --value") printf '%s\n' "$HOME/config/systemd/user" ;;
    "--user show --property=FragmentPath --value sunreactord.service") printf '%s\n' "$HOME/config/systemd/user/sunreactord.service" ;;
    "--user enable sunreactord.service") touch "$SYSTEMCTL_MARKER.enabled" ;;
    "--user disable sunreactord.service") rm -f "$SYSTEMCTL_MARKER.enabled" ;;
    "--user start sunreactord.service"|"--user restart sunreactord.service") touch "$SYSTEMCTL_MARKER.active" ;;
    "--user stop sunreactord.service") exit 1 ;;
    *) exit 0 ;;
esac
SHIM
    chmod +x "$fakebin/systemctl"

    set +e
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_TEST_DAEMON_ALIVE=false SUNREACTOR_IPC_READY_ATTEMPTS=1 \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    status=$?
    set -e

    [[ $status -ne 0 ]]
    grep -F 'Installation rollback was incomplete' "$home/installer.log" >/dev/null
    recovery_dir=$(grep -oE '/tmp/tmp\.[[:alnum:]]+/backup' \
        "$home/installer.log" | tail -n 1)
    [[ -f "$recovery_dir/sunreactord.absent" ]]
    [[ -f "$recovery_dir/sunreactorctl.absent" ]]
    recovery_root=${recovery_dir%/backup}
    rm -rf "$recovery_root" "$home" "$fakebin"
}

test_no_service_install_does_not_call_systemctl() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-called"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' called >"$SYSTEMCTL_MARKER"
exit 1
SHIM
    chmod +x "$fakebin/systemctl"
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service >/dev/null
        ' _ "$ROOT_DIR/install.sh"
    [[ -x "$home/.local/bin/sunreactord" && -x "$home/.local/bin/sunreactorctl" ]]
    [[ ! -e "$marker" ]] || {
        printf 'no-service installation invoked systemctl\n' >&2
        exit 1
    }
    rm -rf "$home" "$fakebin"
}

test_manager_path_mismatch_skips_service_integration() {
    local home fakebin marker
    home=$(mktemp -d); fakebin=$(mktemp -d); marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
case "$*" in
    "--user show-environment"|"--user daemon-reload") exit 0 ;;
    "--user is-active --quiet sunreactord.service"|"--user is-enabled --quiet sunreactord.service") exit 1 ;;
    "--user show -p UnitPath --value") printf '%s\n' /home/test/.config/systemd/user; exit 0 ;;
    "--user show --property=FragmentPath --value sunreactord.service") printf '%s\n' "$HOME/.config/systemd/user.control/sunreactord.service"; exit 0 ;;
    "--user cat sunreactord.service") exit 0 ;;
    *) exit 0 ;;
esac
SHIM
    chmod +x "$fakebin/systemctl"
    set +e
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" XDG_CONFIG_HOME="$home/custom-config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; source "$SCRIPT_DIR/tests/installer_test_support.sh"; fetch_latest_version(){ printf test; }; download_release(){ printf "%s/x" "$TMP_DIR"; }; extract_archive(){ prepare_installer_test_artifacts; }; main >/dev/null 2>"$HOME/installer.log"' _ "$ROOT_DIR/install.sh"
    local status=$?
    set -e
    [[ $status -ne 0 ]]
    [[ ! -e "$home/custom-config/systemd/user/sunreactord.service" ]]
    [[ ! -e "$home/.local/bin/sunreactord" && ! -e "$home/.local/bin/sunreactorctl" ]]
    grep -Fx -- '--user show --property=FragmentPath --value sunreactord.service' "$marker" >/dev/null
    ! grep -E -- '--user (enable|start)' "$marker" >/dev/null
    grep -F 'previous installation was restored' "$home/installer.log" >/dev/null
    rm -rf "$home" "$fakebin"
}

test_default_manager_path_allows_service_integration() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
if [[ "$*" == "--user show -p UnitPath --value" ]]; then
    printf '%s\n' "$HOME/.config/systemd/user"
fi
if [[ "$*" == "--user daemon-reload" ]]; then
    exit 0
fi
if [[ "$*" == "--user is-active --quiet sunreactord.service" || "$*" == "--user is-enabled --quiet sunreactord.service" ]]; then
    exit 1
fi
if [[ "$*" == "--user show --property=FragmentPath --value sunreactord.service" ]]; then
    printf '%s\n' "$HOME/.config/systemd/user/sunreactord.service"
fi
exit 0
SHIM
    cat >"$fakebin/systemd-analyze" <<'SHIM'
#!/usr/bin/env bash
if [[ "$*" == "--user show -p UnitPath --value" ]]; then
    printf '%s\n' "$HOME/.config/systemd/user"
    exit 0
fi
exit 99
SHIM
    chmod +x "$fakebin/systemctl" "$fakebin/systemd-analyze"
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME= XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    [[ -f "$home/.config/systemd/user/sunreactord.service" ]]
    grep -Fx -- '--user cat sunreactord.service' "$marker" >/dev/null
    grep -Fx -- '--user enable sunreactord.service' "$marker" >/dev/null
    grep -Fx -- '--user start sunreactord.service' "$marker" >/dev/null
    if grep -F 'automatic service integration unavailable' "$home/installer.log" >/dev/null 2>&1; then
        printf 'default manager path was treated as unavailable\n' >&2
        exit 1
    fi
    rm -rf "$home" "$fakebin"
}

test_custom_manager_path_allows_service_integration() {
    local home fakebin marker custom_config
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    custom_config="$home/custom-config"
    marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
if [[ "$*" == "--user show -p UnitPath --value" ]]; then
    printf '%s\n' "$TEST_SYSTEMD_DIR"
fi
if [[ "$*" == "--user show --property=FragmentPath --value sunreactord.service" ]]; then
    printf '%s\n' "$TEST_SYSTEMD_DIR/sunreactord.service"
fi
exit 0
SHIM
    chmod +x "$fakebin/systemctl"
    SYSTEMCTL_MARKER="$marker" TEST_SYSTEMD_DIR="$custom_config/systemd/user" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$custom_config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            main >/dev/null
        ' _ "$ROOT_DIR/install.sh"
    [[ -f "$custom_config/systemd/user/sunreactord.service" ]]
    grep -Fx -- '--user cat sunreactord.service' "$marker" >/dev/null
    grep -Fx -- '--user restart sunreactord.service' "$marker" >/dev/null
    rm -rf "$home" "$fakebin"
}

test_manager_path_membership() {
    local home
    home=$(mktemp -d)
    HOME="$home" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
        source "$1"
        paths="/home/test/.config/systemd/user
/etc/systemd/user"
        unit_path_is_discoverable /home/test/.config/systemd/user "$paths"
        ! unit_path_is_discoverable /tmp/custom-config/systemd/user "$paths"
    ' _ "$ROOT_DIR/install.sh"
    rm -rf "$home"
}

test_discoverable_postcondition_is_required() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
case "$*" in
    "--user show-environment"|"--user daemon-reload") exit 0 ;;
    "--user is-active --quiet sunreactord.service"|"--user is-enabled --quiet sunreactord.service") exit 1 ;;
    "--user show -p UnitPath --value") printf '%s\n' "$TEST_SYSTEMD_DIR"; exit 0 ;;
    "--user show --property=FragmentPath --value sunreactord.service") printf '%s\n' "$TEST_SYSTEMD_DIR/sunreactord.service"; exit 0 ;;
    "--user cat sunreactord.service") exit 1 ;;
    *) exit 0 ;;
esac
SHIM
    cat >"$fakebin/systemd-analyze" <<'SHIM'
#!/usr/bin/env bash
[[ "$*" == "--user show -p UnitPath --value" ]] && printf '%s\n' "$TEST_SYSTEMD_DIR"
SHIM
    chmod +x "$fakebin/systemctl" "$fakebin/systemd-analyze"
    set +e
    SYSTEMCTL_MARKER="$marker" TEST_SYSTEMD_DIR="$home/config/systemd/user" PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            main >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    status=$?
    set -e
    [[ $status -ne 0 ]]
    [[ ! -e "$home/config/systemd/user/sunreactord.service" ]]
    [[ ! -e "$home/.local/bin/sunreactord" && ! -e "$home/.local/bin/sunreactorctl" ]]
    grep -Fx -- '--user cat sunreactord.service' "$marker" >/dev/null
    grep -F 'previous installation was restored' "$home/installer.log" >/dev/null
    rm -rf "$home" "$fakebin"
}

test_fragment_path_rejects_shadowed_unit() {
    local home fakebin marker
    home=$(mktemp -d); fakebin=$(mktemp -d); marker="$home/systemctl-calls"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SYSTEMCTL_MARKER"
case "$*" in
    "--user show-environment"|"--user daemon-reload") exit 0 ;;
    "--user is-active --quiet sunreactord.service"|"--user is-enabled --quiet sunreactord.service") exit 1 ;;
    "--user show -p UnitPath --value") printf '%s\n' "$TEST_SYSTEMD_DIR"; exit 0 ;;
    "--user show --property=FragmentPath --value sunreactord.service") printf '%s\n' "$HOME/shadow/systemd/user/sunreactord.service"; exit 0 ;;
    "--user cat sunreactord.service") exit 0 ;;
    *) exit 0 ;;
esac
SHIM
    chmod +x "$fakebin/systemctl"
    set +e
    SYSTEMCTL_MARKER="$marker" TEST_SYSTEMD_DIR="$home/config/systemd/user" PATH="$fakebin:/usr/bin:/bin" HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; source "$SCRIPT_DIR/tests/installer_test_support.sh"; fetch_latest_version(){ printf test; }; download_release(){ printf "%s/x" "$TMP_DIR"; }; extract_archive(){ prepare_installer_test_artifacts; }; main >/dev/null 2>"$HOME/installer.log"' _ "$ROOT_DIR/install.sh"
    local status=$?
    set -e
    [[ $status -ne 0 ]]
    [[ ! -e "$home/config/systemd/user/sunreactord.service" ]]
    [[ ! -e "$home/.local/bin/sunreactord" && ! -e "$home/.local/bin/sunreactorctl" ]]
    grep -Fx -- '--user show --property=FragmentPath --value sunreactord.service' "$marker" >/dev/null
    ! grep -E -- '--user (enable|start)' "$marker" >/dev/null
    grep -F 'unexpected unit' "$home/installer.log" >/dev/null
    rm -rf "$home" "$fakebin"
}

test_service_path_escaping() {
    local home unit
    home=$(mktemp -d)
    unit=$(rendered_unit "$home" "$home/config with space/100%" "$home/state" "$home/cache")
    assert_contains "$unit" "Environment=XDG_CONFIG_HOME=$home/config\\x20with\\x20space/100%%"
    assert_contains "$unit" "Documentation=https://github.com/arcanorca/SunReactor"
    assert_contains "$unit" "ExecStart=\"$home/.local/bin/sunreactord\""
    assert_contains "$unit" "ExecReload=\"$home/.local/bin/sunreactorctl\" reload-config"
    mkdir -p "$home/.local/bin"
    touch "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl"
    chmod +x "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl"
    printf '%s\n' "$unit" >"$home/sunreactord.service"
    if command -v systemd-analyze >/dev/null 2>&1; then
        systemd-analyze verify "$home/sunreactord.service"
    fi
    rm -rf "$home"
}

test_real_user_manager_qualification() {
    if ! command -v systemctl >/dev/null 2>&1 || ! command -v systemd-analyze >/dev/null 2>&1; then
        printf 'real systemd-user qualification: SKIP (required tools unavailable)\n'
        return 0
    fi
    if ! systemctl --user show-environment >/dev/null 2>&1; then
        printf 'real systemd-user qualification: SKIP (user manager unavailable)\n'
        return 0
    fi
    printf 'systemd-analyze environment/version (not running-manager introspection): %s\n' "$(systemd-analyze --version | sed -n '1p')"
    local manager_paths intended_path
    manager_paths=$(systemctl --user show -p UnitPath --value)
    printf 'real systemd-user UnitPath:\n%s\n' "$manager_paths"
    intended_path="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
    if unit_path_is_discoverable "$intended_path" "$manager_paths"; then
        printf 'real manager path qualification: PASS (%s)\n' "$intended_path"
    else
        printf 'real manager path qualification: SKIP (shell path is not manager path: %s)\n' "$intended_path"
    fi
}

test_default_paths_and_unit
test_custom_paths_and_uninstall_paths
test_invalid_xdg_path_is_rejected
test_no_service_and_unavailable_manager
test_systemd_setup_failure_restores_previous_installation
test_daemon_readiness_failure_restores_previous_installation
test_incomplete_rollback_retains_recovery_files
test_no_service_install_does_not_call_systemctl
test_manager_path_mismatch_skips_service_integration
test_default_manager_path_allows_service_integration
test_custom_manager_path_allows_service_integration
test_manager_path_membership
test_discoverable_postcondition_is_required
test_fragment_path_rejects_shadowed_unit
test_service_path_escaping

test_destdir_staging_installation() {
    local home destdir fakebin marker
    home=$(mktemp -d)
    destdir=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/systemctl-called"
    cat >"$fakebin/systemctl" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' called >"$SYSTEMCTL_MARKER"
exit 1
SHIM
    chmod +x "$fakebin/systemctl"
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" DESTDIR="$destdir" \
        XDG_CONFIG_HOME="$home/.config" XDG_STATE_HOME="$home/.local/state" XDG_CACHE_HOME="$home/.cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            main >/dev/null
        ' _ "$ROOT_DIR/install.sh"
    [[ -x "$destdir/$home/.local/bin/sunreactord" && -x "$destdir/$home/.local/bin/sunreactorctl" ]]
    [[ -f "$destdir/$home/.config/systemd/user/sunreactord.service" ]]
    [[ ! -e "$marker" ]] || {
        printf 'DESTDIR staging unexpectedly invoked systemctl\n' >&2
        exit 1
    }
    rm -rf "$home" "$destdir" "$fakebin"
}
test_destdir_staging_installation

test_checksum_helpers() {
    local file expected
    file=$(mktemp)
    printf 'sunreactor checksum fixture\n' >"$file"
    expected=$(sha256sum "$file" | cut -d' ' -f1)
    [[ "$(sha256_file "$file")" == "$expected" ]]
    rm -f "$file"
}

test_archive_member_rejection() {
    if ! command -v python3 >/dev/null 2>&1; then
        printf 'test_archive_member_rejection: SKIP (python3 unavailable)\n'
        return 0
    fi
    local home archive
    home=$(mktemp -d)
    archive="$home/unsafe.tar.gz"
    mkdir -p "$home/input"
    printf bad >"$home/input/sunreactord"
    printf bad >"$home/input/sunreactorctl"
    python3 - "$archive" <<'PY'
import sys, tarfile
with tarfile.open(sys.argv[1], 'w:gz') as tar:
    for name in ('sunreactord', 'sunreactorctl'):
        tar.add(sys.argv[1].replace('/unsafe.tar.gz', '/input/' + name), arcname=name)
    info = tarfile.TarInfo('../escape')
    info.size = 3
    tar.addfile(info, __import__('io').BytesIO(b'bad'))
PY
    ! (SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; extract_archive "$2"' _ "$ROOT_DIR/install.sh" "$archive")
    rm -rf "$home"
}

test_checksum_helpers
test_archive_member_rejection

if command -v systemd-analyze >/dev/null 2>&1; then
    home=$(mktemp -d)
    rendered_unit "$home" "$home/.config" "$home/.local/state" "$home/.cache" >"$home/sunreactord.service"
    mkdir -p "$home/.local/bin"
    touch "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl"
    chmod +x "$home/.local/bin/sunreactord" "$home/.local/bin/sunreactorctl"
    systemd-analyze verify "$home/sunreactord.service"
    rm -rf "$home"
fi

printf 'installer tests passed\n'
