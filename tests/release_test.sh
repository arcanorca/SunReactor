#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SUNREACTOR_INSTALLER_LIBRARY=1 source "$ROOT_DIR/install.sh"

CARGO_VERSION=$(awk '
    /^\[package\]$/ { in_package = 1; next }
    /^\[/ { if (in_package) exit }
    in_package && $1 == "version" && $2 == "=" {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' "$ROOT_DIR/Cargo.toml")

assert_equals() {
    local expected="$1" actual="$2"
    [[ "$actual" == "$expected" ]] || {
        printf 'expected: %s\nactual:   %s\n' "$expected" "$actual" >&2
        exit 1
    }
}

fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

test_tag_version_uses_the_packaged_asset_name() {
    assert_equals \
        "sunreactor-0.12.1-linux-x86_64-gnu.tar.gz" \
        "$(release_archive_name v0.12.1 x86_64 gnu)"
    assert_equals \
        "sunreactor-0.12.1-linux-aarch64-musl.tar.gz" \
        "$(release_archive_name 0.12.1 aarch64 musl)"
    if release_archive_name 'v0.12.1/../../bad' x86_64 gnu >/dev/null 2>&1; then
        printf 'release asset name accepted an unsafe tag\n' >&2
        exit 1
    fi
}

test_packager_rejects_a_tag_that_does_not_match_cargo() {
    if "$ROOT_DIR/scripts/release.sh" 0.0.0-mismatch x86_64-unknown-linux-gnu \
        "$fixture/wrong-version" "$ROOT_DIR" >/dev/null 2>&1; then
        printf 'release helper accepted a version that differs from Cargo.toml\n' >&2
        exit 1
    fi
}

test_packager_includes_service_and_rejects_stale_binaries() {
    local tools target_dir output archive members
    tools="$fixture/tools"
    target_dir="$fixture/target/x86_64-unknown-linux-gnu/release"
    output="$fixture/release"
    mkdir -p "$tools" "$target_dir"

    cat >"$tools/readelf" <<'SH'
#!/usr/bin/env bash
case "${1:-}" in
    --version-info)
        [[ ${FAKE_STATIC:-0} == 1 ]] || printf 'Version: GLIBC_2.34\n'
        ;;
    -l)
        [[ ${FAKE_STATIC:-0} == 1 ]] || printf '      [Requesting program interpreter: /lib64/ld-linux-x86-64.so.2]\n'
        ;;
    -d)
        [[ ${FAKE_STATIC:-0} == 1 ]] || printf ' 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]\n'
        ;;
esac
SH
    chmod +x "$tools/readelf"

    for binary in sunreactord sunreactorctl; do
        cat >"$target_dir/$binary" <<SH
#!/usr/bin/env bash
if [[ \${1:-} == --version ]]; then
    printf '%s %s\\n' '$binary' "$CARGO_VERSION"
elif [[ \${1:-} == --help ]]; then
    printf '%s\\n' '$binary help'
else
    exit 2
fi
SH
        chmod +x "$target_dir/$binary"
    done

    archive=$(PATH="$tools:$PATH" CARGO_TARGET_DIR="$fixture/target" \
        "$ROOT_DIR/scripts/release.sh" "$CARGO_VERSION" x86_64-unknown-linux-gnu \
        "$output" "$ROOT_DIR")
    members=$(tar tzf "$archive" | sort)
    assert_equals \
        "$(printf '%s\n' LICENSE README.md sunreactord sunreactord.service sunreactorctl | sort)" \
        "$members"

    extract_archive "$archive"
    [[ -f "$ARTIFACT_DIR/sunreactord.service" ]] || {
        printf 'packaged service template was not extracted\n' >&2
        exit 1
    }
    verify_binary_versions "$ARTIFACT_DIR" "$CARGO_VERSION"

    mv "$ARTIFACT_DIR/sunreactord.service" "$fixture/sunreactord.service.saved"
    if HOME="$fixture/home" XDG_CONFIG_HOME="$fixture/config" \
        XDG_STATE_HOME="$fixture/state" XDG_CACHE_HOME="$fixture/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c \
        'source "$1"; ARTIFACT_DIR="$2"; render_service_unit "$3"' \
        _ "$ROOT_DIR/install.sh" "$ARTIFACT_DIR" "$fixture/missing.service" \
        >/dev/null 2>&1; then
        printf 'installer accepted a release archive without its service template\n' >&2
        exit 1
    fi
    mv "$fixture/sunreactord.service.saved" "$ARTIFACT_DIR/sunreactord.service"

    HOME="$fixture/home" XDG_CONFIG_HOME="$fixture/config" \
        XDG_STATE_HOME="$fixture/state" XDG_CACHE_HOME="$fixture/cache" \
        SUNREACTOR_INSTALLER_LIBRARY=1 bash -c \
        'source "$1"; ARTIFACT_DIR="$2"; render_service_unit "$3"' \
        _ "$ROOT_DIR/install.sh" "$ARTIFACT_DIR" "$fixture/rendered.service"
    grep -Fq "ExecStart=\"$fixture/home/.local/bin/sunreactord\"" "$fixture/rendered.service" || {
        printf 'installer did not render the downloaded service template using the install path\n' >&2
        exit 1
    }

    cat >"$ARTIFACT_DIR/sunreactord" <<'SH'
#!/usr/bin/env bash
[[ ${1:-} == --version ]] && printf 'sunreactord 0.1.0\n'
SH
    chmod +x "$ARTIFACT_DIR/sunreactord"
    if verify_binary_versions "$ARTIFACT_DIR" "$CARGO_VERSION"; then
        printf 'installer accepted a stale binary from the downloaded archive\n' >&2
        exit 1
    fi

    cat >"$target_dir/sunreactord" <<'SH'
#!/usr/bin/env bash
[[ ${1:-} == --version ]] && printf 'sunreactord 0.1.0\n'
SH
    chmod +x "$target_dir/sunreactord"
    if PATH="$tools:$PATH" CARGO_TARGET_DIR="$fixture/target" \
        "$ROOT_DIR/scripts/release.sh" "$CARGO_VERSION" x86_64-unknown-linux-gnu \
        "$fixture/stale-release" "$ROOT_DIR"; then
        printf 'release helper accepted a stale binary version\n' >&2
        exit 1
    fi

cat >"$target_dir/sunreactord" <<SH
#!/usr/bin/env bash
if [[ \${1:-} == --version ]]; then
    printf 'sunreactord %s\n' "$CARGO_VERSION"
elif [[ \${1:-} == --help ]]; then
    printf 'sunreactord help\n'
else
    exit 2
fi
SH
    chmod +x "$target_dir/sunreactord"
}

test_musl_packager_rejects_dynamic_executables() {
    local tools target_dir output archive
    tools="$fixture/tools"
    target_dir="$fixture/target/x86_64-unknown-linux-musl/release"
    output="$fixture/musl-release"
    mkdir -p "$target_dir"
    cp "$fixture/target/x86_64-unknown-linux-gnu/release/sunreactord" "$target_dir/"
    cp "$fixture/target/x86_64-unknown-linux-gnu/release/sunreactorctl" "$target_dir/"

    if PATH="$tools:$PATH" CARGO_TARGET_DIR="$fixture/target" \
        "$ROOT_DIR/scripts/release.sh" "$CARGO_VERSION" x86_64-unknown-linux-musl \
        "$output" "$ROOT_DIR"; then
        printf 'release helper accepted dynamic musl executables\n' >&2
        exit 1
    fi

    archive=$(FAKE_STATIC=1 PATH="$tools:$PATH" CARGO_TARGET_DIR="$fixture/target" \
        "$ROOT_DIR/scripts/release.sh" "$CARGO_VERSION" x86_64-unknown-linux-musl \
        "$output" "$ROOT_DIR")
    [[ -f "$archive" ]] || {
        printf 'release helper did not package static musl executables\n' >&2
        exit 1
    }
}

test_sha256_manifest_checks_are_exact() {
    local sha256 abi_sha256
    printf 'archive contents\n' >"$fixture/archive.tar.gz"
    sha256=$(sha256_file "$fixture/archive.tar.gz")
    printf '%s  archive.tar.gz\n' "$sha256" >"$fixture/SHA256SUMS"
    verify_checksum "$fixture/archive.tar.gz" "$fixture/SHA256SUMS" archive.tar.gz
    if verify_checksum "$fixture/archive.tar.gz" "$fixture/SHA256SUMS" missing.tar.gz; then
        printf 'checksum manifest accepted a missing artifact\n' >&2
        exit 1
    fi

    printf 'ABI metadata\n' >"$fixture/ABI-METADATA"
    abi_sha256=$(sha256_file "$fixture/ABI-METADATA")
    printf '%s  ABI-METADATA\n' "$abi_sha256" >"$fixture/SHA256SUMS"
    verify_checksum "$fixture/ABI-METADATA" "$fixture/SHA256SUMS" ABI-METADATA
}

test_tag_version_uses_the_packaged_asset_name
test_packager_rejects_a_tag_that_does_not_match_cargo
test_packager_includes_service_and_rejects_stale_binaries
test_musl_packager_rejects_dynamic_executables
test_sha256_manifest_checks_are_exact
printf 'release helper tests passed\n'
