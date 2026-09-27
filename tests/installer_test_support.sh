#!/usr/bin/env bash

# The integration tests source this helper after install.sh so fixtures use
# the installer's temporary directory and service template path.
prepare_installer_test_artifacts() {
    local artifact_dir="$TMP_DIR"

    cat >"$artifact_dir/sunreactord" <<'SH'
#!/usr/bin/env bash
case "${1:-}" in
    --version) printf 'sunreactord test\n' ;;
    --help) printf 'sunreactord test help\n' ;;
    *) exit 0 ;;
esac
SH

    cat >"$artifact_dir/sunreactorctl" <<'SH'
#!/usr/bin/env bash
case "${1:-}" in
    --version) printf 'sunreactorctl test\n' ;;
    --help) printf 'sunreactorctl test help\n' ;;
    status) printf 'daemon_alive: %s\n' "${SUNREACTOR_TEST_DAEMON_ALIVE:-true}" ;;
    *) exit 0 ;;
esac
SH

    chmod 755 "$artifact_dir/sunreactord" "$artifact_dir/sunreactorctl"
    cp "$SCRIPT_DIR/contrib/systemd/sunreactord.service" \
        "$artifact_dir/sunreactord.service"
    ARTIFACT_DIR="$artifact_dir"
}
