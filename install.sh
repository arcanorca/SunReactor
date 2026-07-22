#!/usr/bin/env bash
set -Eeuo pipefail

readonly REPO="${SUNREACTOR_REPO:-arcanorca/SunReactor}"
readonly BINDIR="${SUNREACTOR_BINDIR:-$HOME/.local/bin}"
readonly UNITDIR="${SUNREACTOR_UNITDIR:-$HOME/.config/systemd/user}"
readonly CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/sunreactor/config.toml"
readonly SYSTEMCTL="${SUNREACTOR_SYSTEMCTL:-systemctl}"
readonly SYSTEMD_ANALYZE="${SUNREACTOR_SYSTEMD_ANALYZE:-systemd-analyze}"
WORKDIR="$(mktemp -d)"
readonly WORKDIR
readonly STAGE="$WORKDIR/stage"
readonly BACKUP="$WORKDIR/backup"

QUIET=0
UNINSTALL=0
MUTATED=0
COMMITTED=0
CONFIG_CREATED=0
SERVICE_WAS_ACTIVE=0
SERVICE_WAS_ENABLED=0
RESULT_STATE="DEPENDENCY_FAILURE"

log() { [[ $QUIET -eq 1 ]] || printf '==> %s\n' "$*" >&2; }
warn() { printf '==> WARNING: %s\n' "$*" >&2; }
die() {
    RESULT_STATE="$1"
    shift
    printf '==> ERROR: %s\nSUNREACTOR_RESULT=%s\n' "$*" "$RESULT_STATE" >&2
    exit 1
}

print_banner() {
    [[ $QUIET -eq 1 || ! -t 1 ]] && return

    local mode="$1" reset='\033[0m'
    local art=(
        '  _____             ____                 _             '
        ' / ___| _   _ _ __ |  _ \ ___  __ _  ___| |_ ___  _ __ '
        " \\___ \\| | | | '_ \\| |_) / _ \\/ _\` |/ __| __/ _ \\| '__|"
        '  ___) | |_| | | | |  _ <  __/ (_| | (__| || (_) | |   '
        ' |____/ \__,_|_| |_|_| \_\___|\__,_|\___|\__\___/|_|   '
        '                                                       '
    )
    local colors color_index line

    if [[ $mode == uninstall ]]; then
        colors=('\033[38;5;246m' '\033[38;5;243m' '\033[38;5;240m' '\033[38;5;238m' '\033[38;5;236m' '\033[38;5;234m')
    else
        colors=('\033[38;5;220m' '\033[38;5;214m' '\033[38;5;208m' '\033[38;5;202m' '\033[38;5;196m' '\033[38;5;160m')
    fi

    printf '\n' >&2
    for line in "${!art[@]}"; do
        color_index=$((line % ${#colors[@]}))
        printf '%b%s%b\n' "${colors[$color_index]}" "${art[$line]}" "$reset" >&2
        sleep 0.1
    done

    if [[ $mode == uninstall ]]; then
        printf '%b\n\n' "   \033[1;3;38;5;242mSun sets forever. Your configuration stays safe.\033[0m" >&2
    else
        printf '%b\n\n' "   \033[1;3mAutomate Monitor Brightness, Synced with the Sun\033[0m" >&2
    fi
}

source_build_instructions() {
    printf '%s\n' \
        'Build from source explicitly (Rust is not installed automatically):' \
        '  cargo build --release --locked' \
        "  install -Dm755 target/release/sunreactord '$BINDIR/sunreactord'" \
        "  install -Dm755 target/release/sunreactorctl '$BINDIR/sunreactorctl'" >&2
}

rollback() {
    [[ $MUTATED -eq 1 && $COMMITTED -eq 0 ]] || return 0
    warn "Installation failed after replacement; restoring the previous installation."
    "$SYSTEMCTL" --user stop sunreactord.service >/dev/null 2>&1 || true
    for name in sunreactord sunreactorctl; do
        if [[ -f "$BACKUP/$name" ]]; then
            install -m 755 "$BACKUP/$name" "$BINDIR/$name" || true
        else
            rm -f "$BINDIR/$name"
        fi
    done
    if [[ -f "$BACKUP/sunreactord.service" ]]; then
        install -m 644 "$BACKUP/sunreactord.service" "$UNITDIR/sunreactord.service" || true
    else
        rm -f "$UNITDIR/sunreactord.service"
    fi
    if [[ $CONFIG_CREATED -eq 1 ]]; then
        rm -f "$CONFIG_FILE"
    fi
    "$SYSTEMCTL" --user daemon-reload >/dev/null 2>&1 || true
    if [[ $SERVICE_WAS_ENABLED -eq 1 ]]; then
        "$SYSTEMCTL" --user enable sunreactord.service >/dev/null 2>&1 || true
    else
        "$SYSTEMCTL" --user disable sunreactord.service >/dev/null 2>&1 || true
    fi
    if [[ $SERVICE_WAS_ACTIVE -eq 1 ]]; then
        "$SYSTEMCTL" --user start sunreactord.service >/dev/null 2>&1 || true
    fi
}

cleanup() {
    local rc=$?
    rollback
    rm -rf "$WORKDIR"
    return "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -q|--quiet) QUIET=1 ;;
            --uninstall) UNINSTALL=1 ;;
            *) die UNSUPPORTED_PLATFORM "Unknown option: $1" ;;
        esac
        shift
    done
}

require_commands() {
    local command
    for command in awk chmod cp curl grep head install mktemp mv rm sed sha256sum sleep tar uname; do
        command -v "$command" >/dev/null 2>&1 || die DEPENDENCY_FAILURE "Missing required command: $command"
    done
    command -v "$SYSTEMCTL" >/dev/null 2>&1 || die DEPENDENCY_FAILURE "systemctl is unavailable"
    command -v "$SYSTEMD_ANALYZE" >/dev/null 2>&1 || die DEPENDENCY_FAILURE "systemd-analyze is unavailable"
}

uninstall() {
    print_banner uninstall
    "$SYSTEMCTL" --user disable --now sunreactord.service >/dev/null 2>&1 || true
    rm -f "$UNITDIR/sunreactord.service" "$BINDIR/sunreactord" "$BINDIR/sunreactorctl"
    "$SYSTEMCTL" --user daemon-reload >/dev/null 2>&1 || true
    printf '%s\n' 'SunReactor binaries and user unit removed; configuration and state were preserved.'
    printf '%s\n' 'SUNREACTOR_RESULT=SUCCESS'
    COMMITTED=1
    exit 0
}

latest_version() {
    if [[ -n ${SUNREACTOR_VERSION:-} ]]; then
        printf '%s\n' "$SUNREACTOR_VERSION"
        return
    fi
    local metadata tag
    metadata=$(curl --fail --silent --show-error "https://api.github.com/repos/$REPO/releases/latest") \
        || die DEPENDENCY_FAILURE "Could not download release metadata."
    tag=$(printf '%s\n' "$metadata" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)
    [[ -n $tag ]] || die DEPENDENCY_FAILURE "Release metadata did not contain a tag_name."
    printf '%s\n' "$tag"
}

download_artifact() {
    local version="$1" archive_name="$2" destination="$3"
    if [[ -n ${SUNREACTOR_RELEASE_DIR:-} ]]; then
        [[ -f "$SUNREACTOR_RELEASE_DIR/$archive_name" ]] || return 1
        [[ -f "$SUNREACTOR_RELEASE_DIR/$archive_name.sha256" ]] || return 1
        cp "$SUNREACTOR_RELEASE_DIR/$archive_name" "$destination/"
        cp "$SUNREACTOR_RELEASE_DIR/$archive_name.sha256" "$destination/"
        return
    fi
    local base="https://github.com/$REPO/releases/download/$version"
    curl --fail --location --silent --show-error "$base/$archive_name" -o "$destination/$archive_name" || return 1
    curl --fail --location --silent --show-error "$base/$archive_name.sha256" -o "$destination/$archive_name.sha256" || return 1
}

verify_checksum() {
    local archive="$1" checksum_file="$2" expected actual
    expected=$(awk 'NF { print $1; exit }' "$checksum_file")
    [[ $expected =~ ^[[:xdigit:]]{64}$ ]] || return 1
    actual=$(sha256sum "$archive" | awk '{print $1}')
    [[ $actual == "$expected" ]]
}

classify_launch_failure() {
    local output="$1"
    case "$output" in
        *GLIBC_*not\ found*) printf '%s' 'incompatible GLIBC' ;;
        *Exec\ format\ error*|*cannot\ execute\ binary*) printf '%s' 'wrong architecture' ;;
        *error\ while\ loading\ shared\ libraries*) printf '%s' 'missing dynamic library' ;;
        *Permission\ denied*) printf '%s' 'invalid executable permissions' ;;
        *) printf '%s' 'general launch failure' ;;
    esac
}

smoke_binary() {
    local binary="$1" argument="$2" output
    if output=$("$binary" "$argument" 2>&1); then
        return 0
    fi
    printf '%s: %s\n' "$(classify_launch_failure "$output")" "$output" >&2
    return 1
}

wait_for_ipc() {
    local attempts="${SUNREACTOR_IPC_READY_ATTEMPTS:-100}"
    [[ $attempts =~ ^[1-9][0-9]*$ ]] || attempts=100

    while (( attempts > 0 )); do
        if "$BINDIR/sunreactorctl" ping >/dev/null 2>&1; then
            return 0
        fi
        attempts=$((attempts - 1))
        if (( attempts > 0 )); then
            sleep 0.1
        fi
    done

    return 1
}

render_unit() {
    local template="$1" output="$2" escaped
    [[ $BINDIR == /* ]] || die CONFIG_FAILURE "BINDIR must be absolute: $BINDIR"
    [[ $BINDIR != *[[:space:]]* ]] || die CONFIG_FAILURE "BINDIR paths containing whitespace are not supported: $BINDIR"
    escaped=${BINDIR//\\/\\\\}
    escaped=${escaped//&/\\&}
    escaped=${escaped//|/\\|}
    sed "s|@BINDIR@|$escaped|g" "$template" > "$output"
    if grep -q '@BINDIR@' "$output"; then
        die CONFIG_FAILURE "Service template contains an unresolved BINDIR placeholder."
    fi
}

backup_existing() {
    mkdir -p "$BACKUP"
    [[ -f "$BINDIR/sunreactord" ]] && cp -p "$BINDIR/sunreactord" "$BACKUP/sunreactord"
    [[ -f "$BINDIR/sunreactorctl" ]] && cp -p "$BINDIR/sunreactorctl" "$BACKUP/sunreactorctl"
    if [[ -f "$UNITDIR/sunreactord.service" ]]; then
        cp -p "$UNITDIR/sunreactord.service" "$BACKUP/sunreactord.service"
        if [[ ! -f "$UNITDIR/sunreactord.service.pre-sunreactor" ]]; then
            cp -p "$UNITDIR/sunreactord.service" "$UNITDIR/sunreactord.service.pre-sunreactor"
            log "Preserved the previous user unit as sunreactord.service.pre-sunreactor."
        fi
    fi
    if "$SYSTEMCTL" --user is-active --quiet sunreactord.service >/dev/null 2>&1; then
        SERVICE_WAS_ACTIVE=1
    fi
    if "$SYSTEMCTL" --user is-enabled --quiet sunreactord.service >/dev/null 2>&1; then
        SERVICE_WAS_ENABLED=1
    fi
}

replace_file() {
    local source="$1" destination="$2" mode="$3" temporary
    temporary="${destination}.sunreactor-new-$$"
    install -m "$mode" "$source" "$temporary"
    mv -f "$temporary" "$destination"
}

install_transaction() {
    local rendered_unit="$1"
    mkdir -p "$BINDIR" "$UNITDIR"
    backup_existing
    MUTATED=1
    replace_file "$STAGE/sunreactord" "$BINDIR/sunreactord" 755
    replace_file "$STAGE/sunreactorctl" "$BINDIR/sunreactorctl" 755
    replace_file "$rendered_unit" "$UNITDIR/sunreactord.service" 644

    smoke_binary "$BINDIR/sunreactorctl" --version || die BINARY_INCOMPATIBLE "Installed CLI failed its smoke test."
    smoke_binary "$BINDIR/sunreactord" --help || die BINARY_INCOMPATIBLE "Installed daemon failed its smoke test."
    "$SYSTEMD_ANALYZE" --user verify "$UNITDIR/sunreactord.service" \
        || die SERVICE_FAILURE "systemd rejected the rendered user unit."
    "$SYSTEMCTL" --user daemon-reload || die SERVICE_FAILURE "systemd user manager reload failed."

    if [[ ! -f "$CONFIG_FILE" ]]; then
        "$BINDIR/sunreactorctl" config init >/dev/null || die CONFIG_FAILURE "Default config creation failed."
        CONFIG_CREATED=1
    fi
    "$BINDIR/sunreactorctl" config validate >/dev/null || die CONFIG_FAILURE "Configuration validation failed."
    "$SYSTEMCTL" --user enable --now sunreactord.service \
        || die SERVICE_FAILURE "The daemon could not be enabled and started."
    if ! wait_for_ipc; then
        warn "Daemon did not expose IPC within 10 seconds; service diagnostics follow."
        "$SYSTEMCTL" --user status sunreactord.service --no-pager >&2 || true
        die IPC_FAILURE "Daemon IPC did not respond."
    fi

    local doctor_json
    doctor_json=$("$BINDIR/sunreactorctl" doctor --json) || die DEPENDENCY_FAILURE "Doctor command failed."
    grep -Eq '"i2c_access"[[:space:]]*:[[:space:]]*"I2C_GROUP_CONFIGURED_BUT_SESSION_STALE"' <<< "$doctor_json" \
        && die RELOGIN_REQUIRED "I2C group membership is configured but the current session is stale; log out and back in or reboot."
    grep -Eq '"blocking_errors"[[:space:]]*:[[:space:]]*0' <<< "$doctor_json" \
        || die DEPENDENCY_FAILURE "Doctor reported blocking environment errors. Run sunreactorctl doctor."
}

main() {
    parse_args "$@"
    require_commands
    if [[ $UNINSTALL -eq 1 ]]; then
        uninstall
    fi
    [[ $(uname -s) == Linux ]] || die UNSUPPORTED_PLATFORM "Only GNU/Linux is supported by this installer."
    [[ $(uname -m) == x86_64 ]] || {
        source_build_instructions
        die UNSUPPORTED_PLATFORM "No portable prebuilt artifact is published for $(uname -m)."
    }

    print_banner install
    mkdir -m 700 "$STAGE"
    local version archive_name archive
    version=$(latest_version)
    archive_name="sunreactor-${version}-linux-x86_64-gnu.tar.gz"
    download_artifact "$version" "$archive_name" "$WORKDIR" || {
        source_build_instructions
        die SOURCE_BUILD_REQUIRED "A compatible release asset and checksum were not found. Existing files were not changed."
    }
    archive="$WORKDIR/$archive_name"
    verify_checksum "$archive" "$archive.sha256" || die BINARY_INCOMPATIBLE "Release checksum verification failed; existing files were not changed."
    tar -xzf "$archive" -C "$STAGE" || die BINARY_INCOMPATIBLE "Release archive is corrupted or invalid."
    [[ -f "$STAGE/sunreactord" && -f "$STAGE/sunreactorctl" && -f "$STAGE/sunreactord.service" ]] \
        || die BINARY_INCOMPATIBLE "Release archive is missing required files."
    chmod 755 "$STAGE/sunreactord" "$STAGE/sunreactorctl"
    smoke_binary "$STAGE/sunreactorctl" --version || {
        source_build_instructions
        die BINARY_INCOMPATIBLE "Downloaded CLI is incompatible; existing files were not changed."
    }
    smoke_binary "$STAGE/sunreactord" --help || {
        source_build_instructions
        die BINARY_INCOMPATIBLE "Downloaded daemon is incompatible; existing files were not changed."
    }
    render_unit "$STAGE/sunreactord.service" "$WORKDIR/sunreactord.service"
    install_transaction "$WORKDIR/sunreactord.service"

    COMMITTED=1
    local status=SUCCESS
    if "$BINDIR/sunreactorctl" status 2>/dev/null | grep -q 'configured_monitors: 0'; then
        status=SUCCESS_NO_MONITORS_CONFIGURED
        warn "Software installation succeeded, but no monitors are configured. Run: sunreactorctl discover --apply"
    fi
    printf 'SunReactor installation verified.\nSUNREACTOR_RESULT=%s\n' "$status"
    printf 'Next: run sunreactorctl to open the TUI.\n'
}

main "$@"
