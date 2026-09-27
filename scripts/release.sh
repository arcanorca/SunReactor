#!/usr/bin/env bash
set -euo pipefail

usage() {
    printf 'usage: %s <version> <target> <output-dir> <source-dir>\n' "$0" >&2
    exit 2
}

[[ $# -eq 4 ]] || usage
version="$1"
target="$2"
out_dir="$3"
source_dir="$4"
target_dir="${CARGO_TARGET_DIR:-$source_dir/target}"

case "$version" in
    ''|*[![:alnum:].+-]*) printf 'invalid release version: %s\n' "$version" >&2; exit 2 ;;
esac

cargo_version=$(awk '
    /^\[package\]$/ { in_package = 1; next }
    /^\[/ { if (in_package) exit }
    in_package && $1 == "version" && $2 == "=" {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' "$source_dir/Cargo.toml")
if [[ -z "$cargo_version" || "$version" != "$cargo_version" ]]; then
    printf 'release version %s does not match Cargo.toml version %s\n' \
        "$version" "${cargo_version:-missing}" >&2
    exit 1
fi

case "$target" in
    x86_64-unknown-linux-gnu) artifact_arch=x86_64; artifact_abi=gnu ;;
    aarch64-unknown-linux-gnu) artifact_arch=aarch64; artifact_abi=gnu ;;
    x86_64-unknown-linux-musl) artifact_arch=x86_64; artifact_abi=musl ;;
    aarch64-unknown-linux-musl) artifact_arch=aarch64; artifact_abi=musl ;;
    *) printf 'unsupported target: %s\n' "$target" >&2; exit 2 ;;
esac

archive="sunreactor-${version}-linux-${artifact_arch}-${artifact_abi}.tar.gz"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
package="$work/sunreactor"
mkdir -p "$package"
install -m 755 "$target_dir/$target/release/sunreactord" "$package/sunreactord"
install -m 755 "$target_dir/$target/release/sunreactorctl" "$package/sunreactorctl"
install -m 644 "$source_dir/LICENSE" "$package/LICENSE"
install -m 644 "$source_dir/README.md" "$package/README.md"
install -m 644 "$source_dir/contrib/systemd/sunreactord.service" "$package/sunreactord.service"

command -v readelf >/dev/null 2>&1 || { printf 'readelf is required to inspect release binaries\n' >&2; exit 1; }
glibc_versions="$work/glibc-versions"
: >"$glibc_versions"
interpreter=0
needed=0
for binary in sunreactord sunreactorctl; do
    binary_path="$package/$binary"
    if ! program_headers=$(readelf -l "$binary_path" 2>&1); then
        printf 'could not inspect program headers for %s\n' "$binary" >&2
        exit 1
    fi
    if ! dynamic_section=$(readelf -d "$binary_path" 2>&1); then
        printf 'could not inspect dynamic section for %s\n' "$binary" >&2
        exit 1
    fi
    if ! version_info=$(readelf --version-info "$binary_path" 2>&1); then
        printf 'could not inspect symbol versions for %s\n' "$binary" >&2
        exit 1
    fi
    [[ "$program_headers" == *'Requesting program interpreter'* ]] && interpreter=1
    [[ "$dynamic_section" == *'(NEEDED)'* ]] && needed=1
    printf '%s\n' "$version_info" | grep -oE 'GLIBC_[0-9.]+' >>"$glibc_versions" || true
done

max_glibc=$(sort -Vu "$glibc_versions" | tail -n 1)
static=0
case "$target" in
    *-musl)
        if [[ "$interpreter" -ne 0 || "$needed" -ne 0 ]]; then
            printf 'musl release binaries must be static (interpreter=%s needed=%s)\n' \
                "$interpreter" "$needed" >&2
            exit 1
        fi
        max_glibc=static
        static=1
        ;;
    *-gnu)
        if [[ "$interpreter" -ne 1 || "$needed" -ne 1 || -z "$max_glibc" ]]; then
            printf 'GNU release binaries must be dynamically linked and expose GLIBC version requirements\n' >&2
            exit 1
        fi
        max_glibc=${max_glibc#GLIBC_}
        ;;
esac

for binary in sunreactord sunreactorctl; do
    binary_path="$package/$binary"
    if ! reported_version=$("$binary_path" --version 2>&1); then
        printf '%s --version failed during packaging\n' "$binary" >&2
        exit 1
    fi
    if [[ "$reported_version" != "$binary $version" ]]; then
        printf '%s reports %s; expected %s %s\n' \
            "$binary" "$reported_version" "$binary" "$version" >&2
        exit 1
    fi
    if ! "$binary_path" --help >/dev/null 2>&1; then
        printf '%s --help failed during packaging\n' "$binary" >&2
        exit 1
    fi
done

mkdir -p "$out_dir"
tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner \
    --format=ustar -czf "$out_dir/$archive" -C "$package" \
    LICENSE README.md sunreactorctl sunreactord sunreactord.service
printf 'artifact=%s glibc=%s static=%s\n' "$archive" "$max_glibc" "$static" > "$out_dir/$archive.meta"
printf '%s %s\n' "$archive" "$max_glibc" > "$out_dir/ABI-METADATA"
printf '%s\n' "$out_dir/$archive"
