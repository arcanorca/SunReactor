#!/usr/bin/env bash

# ==========================================
# SUNREACTOR AUTOMATED INSTALLER
# ==========================================
# Adheres to strict mode, SOLID, and KISS.

set -euo pipefail

if [[ -z "${HOME:-}" ]]; then
    printf 'ERROR: HOME must be set for a user-local installation.\n' >&2
    exit 1
fi

# ==========================================
# 1. CONFIGURATION (Readonly Constants)
# ==========================================
readonly REPO="arcanorca/SunReactor"
readonly BIN_DIR="$HOME/.local/bin"
readonly CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
readonly STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}"
readonly CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}"
readonly SYSTEMD_DIR="$CONFIG_HOME/systemd/user"
readonly CFG_DIR="$CONFIG_HOME/sunreactor"
readonly STATE_DIR="$STATE_HOME/sunreactor"
readonly CACHE_DIR="$CACHE_HOME/sunreactor"
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly TMP_DIR="$(mktemp -d)"

# State variables
QUIET=0
UNINSTALL=0
NO_SERVICE=0

# ==========================================
# 2. SYSTEM MODULE
# ==========================================
cleanup() {
    rm -rf "$TMP_DIR"
}
trap 'cleanup; exit 1' INT TERM
trap cleanup EXIT

parse_args() {
    for arg in "$@"; do
        case $arg in
            -q|--quiet) QUIET=1 ;;
            --uninstall) UNINSTALL=1 ;;
            --no-service) NO_SERVICE=1 ;;
            *) ;;
        esac
    done
}

check_dependencies() {
    local deps=("curl" "tar" "install" "mkdir" "mktemp" "grep" "sed" "awk" "sort")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            log_error "Missing required dependency: $dep"
            exit 1
        fi
    done
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        return 1
    fi
}

detect_libc() {
    if command -v getconf >/dev/null 2>&1 && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
        printf 'gnu\n'
        return 0
    fi
    local ldd_output loader
    if ldd_output=$(ldd --version 2>&1) && [[ "$ldd_output" == *musl* ]]; then
        printf 'musl\n'
        return 0
    fi
    for loader in /lib/ld-musl-*.so.1 /lib64/ld-musl-*.so.1; do
        [[ -e "$loader" ]] && { printf 'musl\n'; return 0; }
    done
    return 1
}

verify_checksum() {
    local archive_path="$1" sums_path="$2" artifact_name="$3"
    local expected actual
    expected=$(awk -v name="$artifact_name" '$2 == name || $2 == "*" name { print $1; found=1 } END { exit !found }' "$sums_path") || {
        log_error "Checksum manifest does not contain the downloaded artifact: $artifact_name"
        return 1
    }
    actual=$(sha256_file "$archive_path") || {
        log_error "No SHA-256 utility (sha256sum or shasum) is available."
        return 1
    }
    if [[ "$actual" != "$expected" ]]; then
        log_error "SHA-256 mismatch for $artifact_name (expected $expected, got $actual)."
        return 1
    fi
    log_success "SHA-256 verified for $artifact_name."
}

validate_xdg_paths() {
    local name value
    for name in CONFIG_HOME STATE_HOME CACHE_HOME; do
        value=${!name}
        if [[ "$value" != /* ]]; then
            log_error "$name must be an absolute path: $value"
            exit 1
        fi
        if [[ "$value" == *$'\n'* || "$value" == *$'\r'* ]]; then
            log_error "$name must not contain newlines"
            exit 1
        fi
        if [[ "$value" == *'"'* ]]; then
            log_error "$name must not contain double quotes"
            exit 1
        fi
    done
}

# ==========================================
# 3. LOGGER MODULE
# ==========================================
log_info() {
    [[ $QUIET -eq 1 ]] && return
    echo -e "\033[1;34m==>\033[0m \033[1m$1\033[0m" >&2
}

log_success() {
    [[ $QUIET -eq 1 ]] && return
    echo -e "\033[1;32m==>\033[0m \033[1;32m$1\033[0m" >&2
}

log_error() {
    echo -e "\033[1;31m==> ERROR:\033[0m \033[1m$1\033[0m" >&2
}

# ==========================================
# 4. NETWORK MODULE
# ==========================================
fetch_latest_version() {
    local tag
    tag=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [[ -z "$tag" ]]; then
        log_error "Failed to fetch the latest release version from GitHub."
        exit 1
    fi
    echo "$tag"
}

download_release() {
    local version="$1"
    local arch
    local target
    
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            target="x86_64"
            ;;
        aarch64|arm64)
            target="aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac

    local libc
    if ! libc=$(detect_libc); then
        log_error "Could not determine whether this Linux system uses glibc or musl."
        exit 1
    fi
    local tarball="sunreactor-${version}-linux-${target}-${libc}.tar.gz"
    local url="https://github.com/$REPO/releases/download/${version}/${tarball}"
    local dest="$TMP_DIR/$tarball"
    local sums="$TMP_DIR/SHA256SUMS"
    local abi_metadata="$TMP_DIR/ABI-METADATA"

    log_info "Downloading SunReactor ${version} for ${target}-${libc}..."
    if ! curl -# -fL "$url" -o "$dest"; then
        log_error "Download failed. Check your connection or the release asset existence."
        exit 1
    fi
    if ! curl -fsSL "https://github.com/$REPO/releases/download/${version}/SHA256SUMS" -o "$sums"; then
        log_error "Checksum manifest download failed; nothing was installed."
        exit 1
    fi
    if ! curl -fsSL "https://github.com/$REPO/releases/download/${version}/ABI-METADATA" -o "$abi_metadata"; then
        log_error "ABI metadata download failed; nothing was installed."
        exit 1
    fi
    verify_checksum "$dest" "$sums" "$tarball" || exit 1
    verify_checksum "$abi_metadata" "$sums" "ABI-METADATA" || exit 1
    local required_glibc host_glibc
    if [[ "$libc" == gnu ]]; then
        required_glibc=$(awk -v name="$tarball" '$1 == name { print $2; found=1 } END { exit !found }' "$abi_metadata") || {
            log_error "ABI metadata does not contain the downloaded artifact: $tarball"
            exit 1
        }
        host_glibc=$(getconf GNU_LIBC_VERSION | awk '{print $2}') || {
            log_error "Could not determine the host glibc version."
            exit 1
        }
        if ! printf '%s\n%s\n' "$required_glibc" "$host_glibc" | sort -V -C; then
            log_error "The GNU artifact requires glibc >= $required_glibc; this host has glibc $host_glibc."
            exit 1
        fi
    fi
    echo "$dest"
}

# ==========================================
# 5. FILE OPERATIONS MODULE
# ==========================================
extract_archive() {
    local archive_path="$1"
    log_info "Extracting..."
    local member members
    if ! members=$(tar tzf "$archive_path"); then
        log_error "Could not inspect archive members."
        return 1
    fi
    while IFS= read -r member; do
        case "$member" in
            sunreactord|sunreactorctl|LICENSE|README.md) ;;
            *) log_error "Unexpected or unsafe archive member: $member"; return 1 ;;
        esac
    done <<< "$members"
    rm -rf "$TMP_DIR/extracted"
    mkdir -p "$TMP_DIR/extracted"
    tar xzf "$archive_path" -C "$TMP_DIR/extracted"
    for member in sunreactord sunreactorctl; do
        [[ -f "$TMP_DIR/extracted/$member" ]] || { log_error "Archive is missing $member."; return 1; }
    done
    ARTIFACT_DIR="$TMP_DIR/extracted"
}

install_binaries() {
    log_info "Installing binaries to $BIN_DIR..."
    mkdir -p "$BIN_DIR"
    local artifact_dir="${ARTIFACT_DIR:-$TMP_DIR}"
    install -m 755 "$artifact_dir/sunreactord" "$artifact_dir/sunreactorctl" "$BIN_DIR/"
}

# ==========================================
# 6. SYSTEMD MODULE
# ==========================================
install_systemd_unit() {
    if ! render_service_unit "$TMP_DIR/sunreactord.service"; then
        log_error "Failed to render the systemd user service."
        return 1
    fi
    if ! mkdir -p "$SYSTEMD_DIR"; then
        log_error "Failed to create the systemd user unit directory: $SYSTEMD_DIR"
        return 1
    fi
    if ! install -m 644 "$TMP_DIR/sunreactord.service" "$SYSTEMD_DIR/sunreactord.service"; then
        log_error "Failed to install the systemd user unit: $SYSTEMD_DIR/sunreactord.service"
        return 1
    fi
    log_success "Service unit installed: $SYSTEMD_DIR/sunreactord.service"
}

setup_systemd() {
    log_info "Setting up systemd service..."

    if ! systemctl --user daemon-reload; then
        log_error "Systemd user manager could not reload the installed unit."
        return 1
    fi
    log_success "Systemd user manager reloaded."
    local installed_unit="$SYSTEMD_DIR/sunreactord.service"
    local fragment_path
    if ! fragment_path=$(systemctl --user show --property=FragmentPath --value sunreactord.service); then
        log_error "Systemd user manager could not resolve sunreactord.service."
        return 1
    fi
    if [[ "$fragment_path" != "$installed_unit" ]]; then
        log_error "Systemd user manager resolved sunreactord.service to an unexpected unit: $fragment_path"
        return 1
    fi
    if ! systemctl --user cat sunreactord.service >/dev/null; then
        log_error "Systemd user manager could not discover sunreactord.service by name."
        return 1
    fi
    log_success "Service unit is discoverable by the systemd user manager at the installed path."
    if ! systemctl --user enable sunreactord.service; then
        log_error "Systemd user manager could not enable sunreactord.service."
        return 1
    fi
    log_success "Service enabled."
    if ! systemctl --user start sunreactord.service; then
        log_error "Systemd user manager could not start sunreactord.service."
        return 1
    fi
    log_success "Service started."
}

systemd_user_manager_available() {
    command -v systemctl >/dev/null 2>&1 || return 1
    systemctl --user show-environment >/dev/null 2>&1
}

systemd_user_unit_paths() {
    # UnitPath is reported by the running manager. Do not derive this from the
    # installer's XDG environment, or from systemd-analyze's environment.
    command -v systemctl >/dev/null 2>&1 || return 1
    systemctl --user show -p UnitPath --value
}

unit_path_is_discoverable() {
    local intended_path="$1" manager_paths="$2" path
    # UnitPath is an array rendered with shell quoting. Keep this helper only
    # for diagnostics; FragmentPath is the authoritative postcondition.
    [[ "$manager_paths" == *"$intended_path"* ]] && return 0
    return 1
}

systemd_escape_value() {
    local value="$1"
    value=${value//\\/\\\\}
    value=${value//"/\\\\"}
    value=${value//%/%%}
    value=${value// /\\x20}
    value=${value//$'\t'/\\x09}
    printf '%s' "$value"
}

render_service_unit() {
    local destination="$1"
    local config state cache bin template
    config=$(systemd_escape_value "$CONFIG_HOME")
    state=$(systemd_escape_value "$STATE_HOME")
    cache=$(systemd_escape_value "$CACHE_HOME")
    bin=$(systemd_escape_value "$BIN_DIR")
    template=$(<"$SCRIPT_DIR/contrib/systemd/sunreactord.service")
    template=${template//@CONFIG_HOME@/$config}
    template=${template//@STATE_HOME@/$state}
    template=${template//@CACHE_HOME@/$cache}
    template=${template//@BIN_DIR@/$bin}
    printf '%s\n' "$template" >"$destination"
}

# ==========================================
# 7. PRESENTATION MODULE
# ==========================================
print_banner() {
    [[ $QUIET -eq 1 ]] && return

    local colors
    local reset="\033[0m"
    local art=(
        "  _____             ____                 _             "
        " / ___| _   _ _ __ |  _ \ ___  __ _  ___| |_ ___  _ __ "
        " \___ \| | | | '_ \| |_) / _ \/ _\` |/ __| __/ _ \| '__|"
        "  ___) | |_| | | | |  _ <  __/ (_| | (__| || (_) | |   "
        " |____/ \__,_|_| |_|_| \_\___|\__,_|\___|\__\___/|_|   "
        "                                                       "
    )

    echo ""
    if [[ $UNINSTALL -eq 1 ]]; then
        colors=(
            "\033[38;5;246m" "\033[38;5;243m" "\033[38;5;240m"
            "\033[38;5;238m" "\033[38;5;236m" "\033[38;5;234m"
        )
        for i in "${!art[@]}"; do
            local color_idx=$(( i % ${#colors[@]} ))
            echo -e "${colors[$color_idx]}${art[$i]}$reset"
            sleep 0.1
        done
        echo -e "   \033[1;3;38;5;242mSun sets forever. rm -rf taking over.\033[0m\n"
    else
        colors=(
            "\033[38;5;220m" "\033[38;5;214m" "\033[38;5;208m"
            "\033[38;5;202m" "\033[38;5;196m" "\033[38;5;160m"
        )
        for i in "${!art[@]}"; do
            local color_idx=$(( i % ${#colors[@]} ))
            echo -e "${colors[$color_idx]}${art[$i]}$reset"
            sleep 0.1
        done
        echo -e "   \033[1;3mAutomate Monitor Brightness, Synced with the Sun\033[0m\n"
    fi
    sleep 0.5
}

launch_dashboard() {
    # Check if we need to run the setup wizard (interactive only)
    local needs_setup=0
    if [[ ! -f "$CFG_DIR/config.toml" ]]; then
        needs_setup=1
    else
        local status_output
        status_output=$("$BIN_DIR/sunreactorctl" status 2>/dev/null || echo "")
        if echo "$status_output" | grep -q "configured_monitors: 0"; then
            needs_setup=1
        fi
    fi

    if [[ $needs_setup -eq 1 && $QUIET -eq 0 && -t 1 ]]; then
        log_info "No monitors configured. Auto-discovering displays..."
        
        # Safely extract the config_snippet from the JSON output
        local snippet=""
        if command -v python3 >/dev/null 2>&1; then
            snippet=$("$BIN_DIR/sunreactorctl" discover --json 2>/dev/null | python3 -c 'import json, sys; d=json.load(sys.stdin); print(d.get("config_snippet") or "")' 2>/dev/null || true)
        elif command -v jq >/dev/null 2>&1; then
            snippet=$("$BIN_DIR/sunreactorctl" discover --json 2>/dev/null | jq -r '.config_snippet' || true)
        fi

        if [[ -n "$snippet" && "$snippet" != "null" ]]; then
            # Ensure the config directory exists and initialize default settings
            if [[ ! -f "$CFG_DIR/config.toml" ]]; then
                mkdir -p "$CFG_DIR"
                "$BIN_DIR/sunreactorctl" config init >/dev/null 2>&1 || true
            fi

            # Adjust default bounds based on user preference
            snippet=$(echo "$snippet" | sed 's/min_pct = 0/min_pct = 15/g' | sed 's/max_pct = 100/max_pct = 60/g')
            echo -e "\n$snippet" >> "$CFG_DIR/config.toml"
            if systemd_user_manager_available; then
                systemctl --user reload sunreactord.service || true
            fi
            log_success "Successfully detected and configured your monitors!"
            sleep 1
        else
            log_error "Auto-discovery failed or no monitors found. You may need to configure manually."
            sleep 2
        fi
    fi

    echo ""
    log_success "Files installed successfully."
    
    if [ -t 1 ] && [[ $QUIET -eq 0 ]]; then
        echo -e "Launching dashboard in 3 seconds..."
        sleep 3
        exec "$BIN_DIR/sunreactorctl" tui
    else
        echo -e "You can now open the dashboard by running: \033[1;36msunreactorctl\033[0m"
        echo -e "\033[1;33mNote:\033[0m Make sure \033[1m$BIN_DIR\033[0m is in your \$PATH.\n"
    fi
}

# ==========================================
# 8. ORCHESTRATION MODULE
# ==========================================
uninstall_sunreactor() {
    local erase=0
    [[ $QUIET -eq 0 ]] && erase=1

    erase_line() {
        if [[ $erase -eq 1 ]]; then
            sleep 0.4
            tput cuu 1 2>/dev/null || echo -ne "\033[1A"
            tput el 2>/dev/null || echo -ne "\033[2K"
        fi
    }

    [[ $erase -eq 1 ]] && sleep 1
    
    # 9 lines total to erase:
    # Erase empty line and motto
    erase_line
    erase_line

    if systemd_user_manager_available && systemctl --user is-active --quiet sunreactord.service; then
        systemctl --user stop sunreactord.service || true
    fi
    erase_line

    if systemd_user_manager_available && systemctl --user is-enabled --quiet sunreactord.service 2>/dev/null; then
        systemctl --user disable sunreactord.service || true
    fi
    erase_line

    if [[ -f "$SYSTEMD_DIR/sunreactord.service" ]]; then
        rm -f "$SYSTEMD_DIR/sunreactord.service"
        if systemd_user_manager_available; then
            systemctl --user daemon-reload
        fi
    fi
    erase_line

    rm -f "$BIN_DIR/sunreactord" "$BIN_DIR/sunreactorctl"
    erase_line

    rm -rf "$CFG_DIR"
    rm -rf "$STATE_DIR" "$CACHE_DIR"
    erase_line

    # Erase remaining art lines and top padding
    erase_line
    erase_line
    erase_line

    log_success "SunReactor has been successfully uninstalled."
    echo -e "\033[1;32mAll configuration and state data have been wiped clean.\033[0m"
    exit 0
}

main() {
    parse_args "$@"

    if [[ $UNINSTALL -eq 1 ]]; then
        print_banner
        uninstall_sunreactor
    fi

    validate_xdg_paths
    check_dependencies
    print_banner

    local version
    version=$(fetch_latest_version)

    local archive
    archive=$(download_release "$version")

    extract_archive "$archive"
    install_binaries
    if [[ $NO_SERVICE -eq 1 ]]; then
        log_info "Files installed. Service setup disabled (--no-service); run $BIN_DIR/sunreactord manually."
    elif ! install_systemd_unit; then
        log_error "Installation completed partially: files are installed; service unit installation did not complete."
        exit 1
    elif systemd_user_manager_available; then
        if ! manager_unit_paths=$(systemd_user_unit_paths); then
            log_info "Running systemd user manager UnitPath could not be inspected; FragmentPath will remain authoritative."
        fi
        if setup_systemd; then
            log_success "Systemd user service installed and started."
        else
            log_error "Installation completed partially: files are installed; service activation did not complete."
            exit 1
        fi
    else
        log_info "No usable systemd user manager found; files installed without a service."
        log_info "Run $BIN_DIR/sunreactord manually, or enable a compatible service manager."
    fi
    launch_dashboard
}

# ==========================================
# BOOTSTRAP
# ==========================================
if [[ "${SUNREACTOR_INSTALLER_LIBRARY:-0}" != 1 ]]; then
    main "$@"
fi
