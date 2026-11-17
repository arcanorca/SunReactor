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
    # The nested shell intentionally receives the installer path as $1.
    # shellcheck disable=SC2016
    HOME="$home" XDG_CONFIG_HOME="$config" XDG_STATE_HOME="$state" XDG_CACHE_HOME="$cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; render_service_unit /dev/stdout' _ "$ROOT_DIR/install.sh"
}

test_default_paths_and_unit() {
    local home unit
    home=$(mktemp -d)
    # shellcheck disable=SC2016
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

test_no_systemd_installs_autostart_entry() {
    local home fakebin autostart
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    printf '%s\n' '#!/bin/sh' 'exit 1' >"$fakebin/systemctl"
    chmod +x "$fakebin/systemctl"
    PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-plasma >/dev/null
        ' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
    autostart="$home/config/autostart/sunreactord.desktop"
    [[ -f "$autostart" ]]
    grep -Fx "Exec=$home/.local/bin/sunreactord" "$autostart" >/dev/null
    grep -Fx 'X-GNOME-Autostart-enabled=true' "$autostart" >/dev/null
    grep -Fx 'NoDisplay=true' "$autostart" >/dev/null
    grep -F 'Start the daemon now' "$home/installer.log" >/dev/null

    PATH="$fakebin:/usr/bin:/bin" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; QUIET=1; uninstall_sunreactor' _ "$ROOT_DIR/install.sh" >/dev/null 2>&1
    [[ ! -e "$autostart" ]]
    rm -rf "$home" "$fakebin"
}

test_doas_privilege_helper_selection() {
    local fakebin
    if [[ ${EUID:-1} -eq 0 ]]; then
        printf 'test_doas_privilege_helper_selection: SKIP (root has no privilege helper preference)\n'
        return 0
    fi
    fakebin=$(mktemp -d)
    printf '%s\n' '#!/bin/sh' 'exit 0' >"$fakebin/doas"
    chmod +x "$fakebin/doas"
    PATH="$fakebin:/usr/bin:/bin" HOME=/tmp SUNREACTOR_INSTALLER_LIBRARY=1 \
        bash -c 'command() { if [[ "$1" == -v && "$2" == sudo ]]; then return 1; fi; builtin command "$@"; }; source "$1"; [[ "$(privilege_helper)" == doas ]]; [[ "$(package_install_command apt)" == "doas apt install ddcutil brightnessctl i2c-tools" ]]' \
        _ "$ROOT_DIR/install.sh"
    rm -rf "$fakebin"
}

# Stale org/sunreactor/plasma copies in other user QML roots are moved to the
# backup tree; the installed root, other modules and non-HOME roots are not.
test_plasma_stale_copies_are_moved() {
    local home installed roots backup dry_run
    home=$(mktemp -d)
    installed="$home/.local/lib/qt6/qml"
    mkdir -p "$installed/org/sunreactor/plasma" \
        "$home/.local/lib/qml/org/sunreactor/plasma" \
        "$home/.local/lib/qml/org/sunreactor/tools" \
        "$home/.local/lib/qml/org/other/mod" \
        "$home/extra/qml/org/sunreactor/plasma"
    printf 'new\n' >"$installed/org/sunreactor/plasma/libsunreactorpluginplugin.so"
    printf 'stale\n' >"$home/.local/lib/qml/org/sunreactor/plasma/libsunreactorpluginplugin.so"
    printf 'stale\n' >"$home/extra/qml/org/sunreactor/plasma/qmldir"
    printf 'tools\n' >"$home/.local/lib/qml/org/sunreactor/tools/tools.so"
    printf 'other\n' >"$home/.local/lib/qml/org/other/mod/qmldir"

    # shellcheck disable=SC2016
    roots=$(HOME="$home" XDG_DATA_HOME="$home/data" QML_IMPORT_PATH="/usr/lib/qt6/qml:$home/extra/qml" \
        QML2_IMPORT_PATH="$installed" SUNREACTOR_INSTALLER_LIBRARY=1 \
        bash -c 'source "$1"; plasma_stale_scan_roots "$2"' _ "$ROOT_DIR/install.sh" "$installed")
    assert_contains "$roots" "$home/extra/qml"
    assert_contains "$roots" "$home/.local/lib/qml"
    [[ "$roots" != *"$installed"* && "$roots" != */usr/* ]] || {
        printf 'scan roots include the installed or a non-HOME directory: %s\n' "$roots" >&2
        exit 1
    }

    # shellcheck disable=SC2016
    HOME="$home" XDG_DATA_HOME="$home/data" QML_IMPORT_PATH="/usr/lib/qt6/qml:$home/extra/qml" \
        QML2_IMPORT_PATH="$installed" SUNREACTOR_INSTALLER_LIBRARY=1 \
        bash -c 'source "$1"; sweep_stale_plasma_copies "$2"' _ "$ROOT_DIR/install.sh" "$installed" \
        2>"$home/sweep.log"

    [[ ! -e "$home/.local/lib/qml/org/sunreactor/plasma" && ! -e "$home/extra/qml/org/sunreactor/plasma" ]]
    grep -F 'Moved stale SunReactor plugin copy' "$home/sweep.log" >/dev/null
    grep -F "$home/.local/lib/qml/org/sunreactor/plasma" "$home/sweep.log" >/dev/null
    backup=$(find "$home/data/sunreactor-backups" -path '*/stale-plugin-*' -name libsunreactorpluginplugin.so | head -n 1)
    [[ -n "$backup" ]] && grep -Fx 'stale' "$backup" >/dev/null
    [[ -f "$(find "$home/data/sunreactor-backups" -path '*/stale-plugin-*' -name qmldir | head -n 1)" ]]
    grep -Fx 'new' "$installed/org/sunreactor/plasma/libsunreactorpluginplugin.so" >/dev/null
    grep -Fx 'tools' "$home/.local/lib/qml/org/sunreactor/tools/tools.so" >/dev/null
    grep -Fx 'other' "$home/.local/lib/qml/org/other/mod/qmldir" >/dev/null

    # The dry-run plan lists the same scan roots and changes nothing.
    dry_run=$(HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" QML2_IMPORT_PATH="$installed" \
        bash "$ROOT_DIR/install.sh" --dry-run --with-plasma)
    assert_contains "$dry_run" "stale-plugin-"
    assert_contains "$dry_run" "scan: $home/.local/lib/qml"
    rm -rf "$home"
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
            main --no-plasma >/dev/null
        ' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
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
            main --no-plasma >/dev/null
        ' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
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
            main --no-plasma >/dev/null
        ' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
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
            main --no-service --no-plasma >/dev/null
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
    SYSTEMCTL_MARKER="$marker" PATH="$fakebin:/usr/bin:/bin" HOME="$home" XDG_CONFIG_HOME="$home/custom-config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; source "$SCRIPT_DIR/tests/installer_test_support.sh"; fetch_latest_version(){ printf test; }; download_release(){ printf "%s/x" "$TMP_DIR"; }; extract_archive(){ prepare_installer_test_artifacts; }; main --no-plasma >/dev/null' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
    local status=$?
    set -e
    [[ $status -ne 0 ]]
    [[ ! -e "$home/custom-config/systemd/user/sunreactord.service" ]]
    [[ ! -e "$home/.local/bin/sunreactord" && ! -e "$home/.local/bin/sunreactorctl" ]]
    grep -Fx -- '--user show --property=FragmentPath --value sunreactord.service' "$marker" >/dev/null
    if grep -E -- '--user (enable|start)' "$marker" >/dev/null; then
        printf 'service setup ran after manager path mismatch\n' >&2
        exit 1
    fi
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
        XDG_CONFIG_HOME='' XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-plasma >/dev/null 2>"$HOME/installer.log"
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
            main --no-plasma >/dev/null
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
            main --no-plasma >/dev/null
        ' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
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
    SYSTEMCTL_MARKER="$marker" TEST_SYSTEMD_DIR="$home/config/systemd/user" PATH="$fakebin:/usr/bin:/bin" HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; source "$SCRIPT_DIR/tests/installer_test_support.sh"; fetch_latest_version(){ printf test; }; download_release(){ printf "%s/x" "$TMP_DIR"; }; extract_archive(){ prepare_installer_test_artifacts; }; main --no-plasma >/dev/null' _ "$ROOT_DIR/install.sh" 2>"$home/installer.log"
    local status=$?
    set -e
    [[ $status -ne 0 ]]
    [[ ! -e "$home/config/systemd/user/sunreactord.service" ]]
    [[ ! -e "$home/.local/bin/sunreactord" && ! -e "$home/.local/bin/sunreactorctl" ]]
    grep -Fx -- '--user show --property=FragmentPath --value sunreactord.service' "$marker" >/dev/null
    if grep -E -- '--user (enable|start)' "$marker" >/dev/null; then
        printf 'service setup ran after discoverability failure\n' >&2
        exit 1
    fi
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
test_no_systemd_installs_autostart_entry
test_doas_privilege_helper_selection
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
            main --no-plasma >/dev/null
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

# Plasma widget step, without KDE: shims on PATH stand in for cmake,
# plasmashell and kpackagetool6. cmake logs its calls to $CMAKE_STUB_LOG and
# fails when CMAKE_STUB_FAIL is set; its install writes fake plugin files.
plasma_shims() {
    local fakebin="$1"
    cat >"$fakebin/cmake" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"${CMAKE_STUB_LOG:-/dev/null}"
[[ -z "${CMAKE_STUB_FAIL:-}" ]] || { printf 'cmake stub: configure failed\n' >&2; exit 1; }
case "${1:-}" in
    --build) exit 0 ;;
    --install)
        qml_relative="${CMAKE_STUB_QML_RELATIVE:-lib/qt6/qml}"
        root="${DESTDIR:?}$HOME/.local"
        mkdir -p "$root/lib" "$root/$qml_relative/org/sunreactor/plasma" \
            "$root/share/plasma/plasmoids/org.sunreactor.plasma/contents/ui"
        printf 'new plugin\n' >"$root/lib/libsunreactorplugin.so"
        printf 'new qml plugin\n' >"$root/$qml_relative/org/sunreactor/plasma/libsunreactorpluginplugin.so"
        printf 'module org.sunreactor.plasma\n' >"$root/$qml_relative/org/sunreactor/plasma/qmldir"
        printf 'staged package, never copied\n' \
            >"$root/share/plasma/plasmoids/org.sunreactor.plasma/contents/ui/ignored.qml"
        exit 0 ;;
    *)
        build_dir=""
        while [[ $# -gt 0 ]]; do
            if [[ "$1" == -B ]]; then
                build_dir="$2"
                mkdir -p "$build_dir"
            fi
            shift
        done
        exit 0 ;;
esac
SHIM
    printf '#!/usr/bin/env bash\nexit 0\n' >"$fakebin/plasmashell"
    printf '#!/usr/bin/env bash\nexit 0\n' >"$fakebin/kpackagetool6"
    chmod +x "$fakebin/cmake" "$fakebin/plasmashell" "$fakebin/kpackagetool6"
}

# Hides one tool from `command -v` inside a test shell: merged-/usr systems
# would otherwise still find a real plasmashell on /usr/bin.
# shellcheck disable=SC2016
HIDE_TOOL_SNIPPET='command() { if [[ "$1" == -v && "$2" == "${HIDE_TOOL:-}" ]]; then return 1; fi; builtin command "$@"; }'

test_plasma_flags_and_decision() {
    local home fakebin
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    plasma_shims "$fakebin"
    HOME="$home" SUNREACTOR_INSTALLER_LIBRARY=1 PATH="$fakebin:/usr/bin:/bin" bash -c '
        source "$1"
        parse_args --with-plasma --dry-run
        [[ "$PLASMA_MODE" == on && "$DRY_RUN" == 1 ]]
        parse_args --no-plasma
        [[ "$PLASMA_MODE" == off ]]
        ! plasma_widget_decision
        [[ "$PLASMA_DECISION" == "skip: disabled by --no-plasma" ]]
        PLASMA_MODE=auto
        plasma_widget_decision
        [[ "$PLASMA_DECISION" == install ]]
    ' _ "$ROOT_DIR/install.sh"
    rm -rf "$home" "$fakebin"
}

test_plasma_dry_run_changes_nothing() {
    local home fakebin out
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    plasma_shims "$fakebin"
    out=$(HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" PATH="$fakebin:/usr/bin:/bin" \
        bash "$ROOT_DIR/install.sh" --dry-run --with-plasma)
    assert_contains "$out" "decision: install"
    assert_contains "$out" "$home/data/plasma/plasmoids/org.sunreactor.plasma"
    assert_contains "$out" "qmlcache"
    assert_contains "$out" "systemctl --user restart plasma-plasmashell.service"
    [[ ! -e "$home/.local" && ! -e "$home/data" && ! -e "$home/config" ]] || {
        printf 'dry run created files under HOME\n' >&2
        exit 1
    }
    rm -rf "$home" "$fakebin"
}

test_plasma_auto_skips_without_plasmashell() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/cmake-calls"
    plasma_shims "$fakebin"
    HIDE_TOOL=plasmashell CMAKE_STUB_LOG="$marker" HOME="$home" \
        XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" \
        XDG_DATA_HOME="$home/data" PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 \
        bash -c "$HIDE_TOOL_SNIPPET"'
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    grep -F 'Plasma widget skipped: plasmashell or kpackagetool6 not found' "$home/installer.log" >/dev/null
    [[ ! -e "$marker" ]] || {
        printf 'widget step ran cmake without plasmashell\n' >&2
        exit 1
    }
    [[ ! -e "$home/data/plasma" ]]
    rm -rf "$home" "$fakebin"
}

test_plasma_no_plasma_never_builds() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/cmake-calls"
    plasma_shims "$fakebin"
    CMAKE_STUB_LOG="$marker" HOME="$home" XDG_CONFIG_HOME="$home/config" \
        XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" \
        PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service --no-plasma >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    [[ ! -e "$marker" ]] || {
        printf -- '--no-plasma invoked cmake\n' >&2
        exit 1
    }
    [[ ! -e "$home/data/plasma" ]]
    rm -rf "$home" "$fakebin"
}

test_plasma_environment_is_idempotent() {
    local home sentinel env_file
    home=$(mktemp -d)
    sentinel='keep this file untouched'
    HOME="$home" XDG_CONFIG_HOME="$home/config" SUNREACTOR_INSTALLER_LIBRARY=1 \
        bash -c 'source "$1"; mkdir -p "$CONFIG_HOME/environment.d"; printf "%s\n" "$2" >"$CONFIG_HOME/environment.d/10-qml-import-path.conf"; write_plasma_environment "$HOME/.local/lib64/qt6/qml"' \
        _ "$ROOT_DIR/install.sh" "$sentinel"
    env_file="$home/config/plasma-workspace/env/sunreactor.sh"
    [[ -x "$env_file" ]]
    grep -Fx "$sentinel" "$home/config/environment.d/10-qml-import-path.conf" >/dev/null
    HOME="$home" XDG_CONFIG_HOME="$home/config" bash -c '
        QML_IMPORT_PATH="/existing:$HOME/.local/lib64/qt6/qml:/tail"
        QML2_IMPORT_PATH="/other"
        . "$1"
        . "$1"
        [[ "$QML_IMPORT_PATH" == "/existing:$HOME/.local/lib64/qt6/qml:/tail" ]]
        [[ "$QML2_IMPORT_PATH" == "$HOME/.local/lib64/qt6/qml:/other" ]]
    ' _ "$env_file"
    rm -rf "$home"
}

test_qtpaths6_lib64_layout() {
    local home fakebin marker
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    marker="$home/cmake-calls"
    plasma_shims "$fakebin"
    printf '%s\n' '#!/bin/sh' 'printf "%s\n" /usr/lib64/qt6/qml' >"$fakebin/qtpaths6"
    chmod +x "$fakebin/qtpaths6"
    CMAKE_STUB_LOG="$marker" CMAKE_STUB_QML_RELATIVE=lib64/qt6/qml HOME="$home" XDG_CONFIG_HOME="$home/config" \
        XDG_STATE_HOME="$home/state" XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" \
        PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    grep -F -- '-DKDE_INSTALL_QMLDIR=lib64/qt6/qml' "$marker" >/dev/null
    [[ -f "$home/.local/lib64/qt6/qml/org/sunreactor/plasma/qmldir" ]]
    grep -Fx "$home/.local/lib64/qt6/qml" "$home/config/sunreactor/plasma-qml-dir" >/dev/null
    grep -F "$home/.local/lib64/qt6/qml" "$home/config/plasma-workspace/env/sunreactor.sh" >/dev/null
    rm -rf "$home" "$fakebin"
}

test_plasma_widget_success_is_atomic_and_keeps_backup() {
    local home fakebin data status
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    data="$home/data"
    plasma_shims "$fakebin"
    # A previous widget: its package and plugin are replaced and must be kept.
    mkdir -p "$data/plasma/plasmoids/org.sunreactor.plasma" "$home/.local/lib"
    printf 'old package\n' >"$data/plasma/plasmoids/org.sunreactor.plasma/metadata.json"
    printf 'old plugin\n' >"$home/.local/lib/libsunreactorplugin.so"
    set +e
    HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$data" CMAKE_STUB_LOG="$home/cmake-calls" \
        PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    status=$?
    set -e
    [[ $status -eq 0 ]] || {
        cat "$home/installer.log" >&2
        exit 1
    }
    grep -F 'Plasma widget installed' "$home/installer.log" >/dev/null
    grep -F 'systemctl --user restart plasma-plasmashell.service' "$home/installer.log" >/dev/null
    cmp -s "$ROOT_DIR/plasma/org.sunreactor.plasma/package/metadata.json" \
        "$data/plasma/plasmoids/org.sunreactor.plasma/metadata.json"
    grep -Fx 'new plugin' "$home/.local/lib/libsunreactorplugin.so" >/dev/null
    grep -Fx 'new qml plugin' "$home/.local/lib/qt6/qml/org/sunreactor/plasma/libsunreactorpluginplugin.so" >/dev/null
    grep -Fx 'module org.sunreactor.plasma' "$home/.local/lib/qt6/qml/org/sunreactor/plasma/qmldir" >/dev/null
    [[ ! -e "$data/plasma/plasmoids/org.sunreactor.plasma/contents/ui/ignored.qml" ]]
    grep -Fx 'old package' "$(find "$data/sunreactor-backups" -name metadata.json | head -n 1)" >/dev/null
    grep -Fx 'old plugin' "$(find "$data/sunreactor-backups" -name libsunreactorplugin.so | head -n 1)" >/dev/null
    [[ -z "$(find "$home" -name '*.sunreactor-new' -o -name '*.new-*' 2>/dev/null)" ]] || {
        printf 'widget install left temporary files behind\n' >&2
        exit 1
    }
    rm -rf "$home" "$fakebin"
}

test_plasma_failure_never_fails_install() {
    local home fakebin status
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    plasma_shims "$fakebin"
    set +e
    CMAKE_STUB_FAIL=1 HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" \
        PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service --with-plasma >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    status=$?
    set -e
    [[ $status -eq 0 ]] || {
        cat "$home/installer.log" >&2
        printf 'a failing widget step failed the whole install\n' >&2
        exit 1
    }
    grep -F 'Plasma widget not installed' "$home/installer.log" >/dev/null
    [[ -x "$home/.local/bin/sunreactord" && -x "$home/.local/bin/sunreactorctl" ]]
    [[ ! -e "$home/data/plasma/plasmoids/org.sunreactor.plasma" ]]
    rm -rf "$home" "$fakebin"
}

test_plasma_qmlcache_cleared_and_running_warning() {
    local home fakebin status
    home=$(mktemp -d)
    fakebin=$(mktemp -d)
    plasma_shims "$fakebin"

    mkdir -p "$home/cache/plasmashell/qmlcache" "$home/cache/other"
    printf 'stale\n' >"$home/cache/plasmashell/qmlcache/stale.qmlc"
    printf 'keep\n' >"$home/cache/plasmashell/keep.txt"
    printf 'keep\n' >"$home/cache/other/keep.txt"

    printf '#!/usr/bin/env bash\nexit 0\n' >"$fakebin/pgrep"
    chmod +x "$fakebin/pgrep"

    set +e
    HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" CMAKE_STUB_LOG="$home/cmake-calls" \
        PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service --with-plasma >/dev/null 2>"$HOME/installer.log"
        ' _ "$ROOT_DIR/install.sh"
    status=$?
    set -e
    [[ $status -eq 0 ]] || {
        cat "$home/installer.log" >&2
        exit 1
    }

    [[ ! -e "$home/cache/plasmashell/qmlcache" ]] || {
        printf 'plasmashell/qmlcache was not cleared\n' >&2
        exit 1
    }
    [[ -f "$home/cache/plasmashell/keep.txt" && -f "$home/cache/other/keep.txt" ]] || {
        printf 'unrelated cache files were removed\n' >&2
        exit 1
    }
    grep -F 'Cleared Plasma QML cache' "$home/installer.log" >/dev/null
    grep -F 'plasmashell is currently running' "$home/installer.log" >/dev/null
    grep -F 'systemctl --user restart plasma-plasmashell.service' "$home/installer.log" >/dev/null
    grep -F 'plasmashell --replace' "$home/installer.log" >/dev/null

    # When plasmashell is not running, the running-process warning is omitted.
    printf '#!/usr/bin/env bash\nexit 1\n' >"$fakebin/pgrep"
    chmod +x "$fakebin/pgrep"
    HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" \
        XDG_CACHE_HOME="$home/cache" XDG_DATA_HOME="$home/data" CMAKE_STUB_LOG="$home/cmake-calls" \
        PATH="$fakebin:/usr/bin:/bin" SUNREACTOR_INSTALLER_LIBRARY=1 bash -c '
            source "$1"
            source "$SCRIPT_DIR/tests/installer_test_support.sh"
            fetch_latest_version() { printf "%s\n" test; }
            download_release() { printf "%s/archive.tar.gz\n" "$TMP_DIR"; }
            extract_archive() { prepare_installer_test_artifacts; }
            launch_dashboard() { :; }
            main --no-service --with-plasma >/dev/null 2>"$HOME/installer-stopped.log"
        ' _ "$ROOT_DIR/install.sh"

    if grep -F 'plasmashell is currently running' "$home/installer-stopped.log" >/dev/null; then
        printf 'running-plasmashell warning printed when plasmashell was stopped\n' >&2
        exit 1
    fi
    grep -F 'Restart plasmashell to load the widget' "$home/installer-stopped.log" >/dev/null

    rm -rf "$home" "$fakebin"
}

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
    if SUNREACTOR_INSTALLER_LIBRARY=1 bash -c 'source "$1"; extract_archive "$2"' _ "$ROOT_DIR/install.sh" "$archive"; then
        printf 'unsafe archive was accepted\n' >&2
        exit 1
    fi
    rm -rf "$home"
}

test_plasma_flags_and_decision
test_plasma_dry_run_changes_nothing
test_plasma_auto_skips_without_plasmashell
test_plasma_no_plasma_never_builds
test_plasma_environment_is_idempotent
test_qtpaths6_lib64_layout
test_plasma_widget_success_is_atomic_and_keeps_backup
test_plasma_failure_never_fails_install
test_plasma_qmlcache_cleared_and_running_warning
test_plasma_stale_copies_are_moved

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
