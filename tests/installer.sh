#!/usr/bin/env bash
set -euo pipefail

# Keep the historical test command as an alias to the current installer suite.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$SCRIPT_DIR/installer_test.sh" "$@"
