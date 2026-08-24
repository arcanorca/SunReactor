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

mkdir -p "$out_dir"
tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner \
    --format=ustar -czf "$out_dir/$archive" -C "$package" LICENSE README.md sunreactorctl sunreactord

max_glibc=""
for binary in sunreactord sunreactorctl; do
    version=$(readelf --version-info "$target_dir/$target/release/$binary" 2>/dev/null \
        | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1 || true)
    [[ -n "$version" && ( -z "$max_glibc" || $(printf '%s\n%s\n' "$max_glibc" "${version#GLIBC_}" | sort -V | tail -1) == "${version#GLIBC_}" ) ]] && max_glibc="${version#GLIBC_}"
done
interpreter=0
needed=0
for binary in sunreactord sunreactorctl; do
    readelf -l "$target_dir/$target/release/$binary" | grep -q 'Requesting program interpreter' && interpreter=1 || true
    readelf -d "$target_dir/$target/release/$binary" | grep -q 'NEEDED' && needed=1 || true
done
static=0
[[ "$interpreter" -eq 0 && "$needed" -eq 0 ]] && static=1
printf 'artifact=%s glibc=%s static=%s\n' "$archive" "$max_glibc" "$static" > "$out_dir/$archive.meta"
printf '%s %s\n' "$archive" "${max_glibc:-static}" > "$out_dir/ABI-METADATA"
printf '%s\n' "$out_dir/$archive"
