#!/usr/bin/env bash

# ==========================================
# SUNREACTOR AUTOMATED INSTALLER
# ==========================================
# Adheres to strict mode, SOLID, and KISS.

set -euo pipefail

# ==========================================
# 1. CONFIGURATION (Readonly Constants)
# ==========================================
readonly REPO="arcanorca/SunReactor"
readonly BIN_DIR="$HOME/.local/bin"
readonly SYSTEMD_DIR="$HOME/.config/systemd/user"
readonly CFG_DIR="$HOME/.config/sunreactor"
readonly TMP_DIR="$(mktemp -d)"

# State variables
QUIET=0
UNINSTALL=0

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
            *) ;;
        esac
    done
}

check_dependencies() {
    local deps=("curl" "tar" "sudo")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            log_error "Missing required dependency: $dep"
            exit 1
        fi
    done

    # JSON parsing for auto-discovery
    if ! command -v python3 >/dev/null 2>&1 && ! command -v jq >/dev/null 2>&1; then
        log_error "Missing dependency: 'python3' or 'jq' is required for auto-discovery setup."
        exit 1
    fi
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
        x86_64)
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

    local tarball="sunreactor-${version}-linux-${target}.tar.gz"
    local url="https://github.com/$REPO/releases/download/${version}/${tarball}"
    local dest="$TMP_DIR/$tarball"

    log_info "Downloading SunReactor ${version} for ${target}..."
    if ! curl -# -L "$url" -o "$dest"; then
        log_error "Download failed. Check your connection or the release asset existence."
        exit 1
    fi
    echo "$dest"
}

# ==========================================
# 5. FILE OPERATIONS MODULE
# ==========================================
extract_archive() {
    local archive_path="$1"
    log_info "Extracting..."
    tar xzf "$archive_path" -C "$TMP_DIR"
}

install_binaries() {
    log_info "Installing binaries to $BIN_DIR..."
    mkdir -p "$BIN_DIR"
    install -m 755 "$TMP_DIR/sunreactord" "$TMP_DIR/sunreactorctl" "$BIN_DIR/"
}

# ==========================================
# 6. SYSTEMD MODULE
# ==========================================
setup_systemd() {
    log_info "Setting up systemd service..."
    
    # Defensive replacement to ensure systemd uses the correct local binary path
    sed -i "s|/usr/bin/sunreactord|$BIN_DIR/sunreactord|g" "$TMP_DIR/sunreactord.service"
    sed -i "s|/usr/local/bin/sunreactord|$BIN_DIR/sunreactord|g" "$TMP_DIR/sunreactord.service"
    
    mkdir -p "$SYSTEMD_DIR"
    cp "$TMP_DIR/sunreactord.service" "$SYSTEMD_DIR/"
    
    systemctl --user daemon-reload
    log_info "Enabling and starting daemon..."
    systemctl --user enable --now sunreactord.service
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
            systemctl --user reload sunreactord.service || true
            log_success "Successfully detected and configured your monitors!"
            sleep 1
        else
            log_error "Auto-discovery failed or no monitors found. You may need to configure manually."
            sleep 2
        fi
    fi

    echo ""
    log_success "Installation Complete!"
    
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

    if systemctl --user is-active --quiet sunreactord.service; then
        systemctl --user stop sunreactord.service || true
    fi
    erase_line

    if systemctl --user is-enabled --quiet sunreactord.service 2>/dev/null; then
        systemctl --user disable sunreactord.service || true
    fi
    erase_line

    if [[ -f "$SYSTEMD_DIR/sunreactord.service" ]]; then
        rm -f "$SYSTEMD_DIR/sunreactord.service"
        systemctl --user daemon-reload
    fi
    erase_line

    rm -f "$BIN_DIR/sunreactord" "$BIN_DIR/sunreactorctl"
    erase_line

    rm -rf "$CFG_DIR"
    rm -rf "$HOME/.local/state/sunreactor"
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

    check_dependencies
    print_banner

    local version
    version=$(fetch_latest_version)

    local archive
    archive=$(download_release "$version")

    extract_archive "$archive"
    install_binaries
    setup_systemd
    launch_dashboard
}

# ==========================================
# BOOTSTRAP
# ==========================================
main "$@"
