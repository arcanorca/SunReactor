#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SUNREACTOR_INSTALLER_LIBRARY=1 source "$ROOT_DIR/install.sh"

fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin"
printf a >"$fixture/bin/sunreactord"
printf b >"$fixture/bin/sunreactorctl"
printf '0.1.0\n' >"$fixture/archive"
sha256=$(sha256_file "$fixture/archive")
printf '%s  artifact.tar.gz\n' "$sha256" >"$fixture/SHA256SUMS"
verify_checksum "$fixture/archive" "$fixture/SHA256SUMS" artifact.tar.gz
if verify_checksum "$fixture/archive" "$fixture/SHA256SUMS" missing.tar.gz; then exit 1; fi
printf 'abi-metadata\n' >"$fixture/ABI-METADATA"
abi_sha256=$(sha256_file "$fixture/ABI-METADATA")
printf '%s  ABI-METADATA\n' "$abi_sha256" >"$fixture/SHA256SUMS"
verify_checksum "$fixture/ABI-METADATA" "$fixture/SHA256SUMS" ABI-METADATA
printf 'release helper tests passed\n'
