#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
PKG_DIR="$ROOT/plasma/org.sunreactor.plasma/package"
DIST_DIR="$ROOT/dist"

version=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["KPlugin"]["Version"])' "$PKG_DIR/metadata.json")
out_file="$DIST_DIR/org.sunreactor.plasma-v${version}.plasmoid"
desc_file="$DIST_DIR/KDE_STORE_DESCRIPTION.md"

mkdir -p "$DIST_DIR"

if command -v kpackagetool6 >/dev/null 2>&1; then
    stage="$(mktemp -d)"
    trap 'rm -rf "$stage"' EXIT
    kpackagetool6 --type Plasma/Applet --packageroot "$stage" --install "$PKG_DIR" >/dev/null
fi

python3 - "$PKG_DIR" "$out_file" <<'PY'
import os
import sys
import zipfile
from pathlib import Path

pkg = Path(sys.argv[1])
out = Path(sys.argv[2])

fixed_dt = (2026, 1, 1, 0, 0, 0)
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for root, dirs, files in os.walk(pkg):
        dirs[:] = sorted(d for d in dirs if not d.startswith("."))
        for fname in sorted(files):
            if fname.startswith(".") or fname.endswith(("~", ".bak", ".orig")):
                continue
            full = Path(root) / fname
            rel = full.relative_to(pkg).as_posix()
            if rel != "metadata.json" and not rel.startswith("contents/"):
                continue
            zi = zipfile.ZipInfo(rel, date_time=fixed_dt)
            zi.external_attr = (0o644 & 0xFFFF) << 16
            zf.writestr(zi, full.read_bytes())
PY

cat >"$desc_file" <<EOF
# SunReactor v${version} — Adaptive Brightness & Weather Widget

Solar- and weather-driven adaptive brightness widget for KDE Plasma 6 (both **Panel Popup** and **Scalable Live Desktop Widget**), synced live with \`sunreactord\` across **41 themes / 13 flagship skins** (Nothing OS, Xbox 360 Blades Light/Dark, Nintendo DS, Nintendo Wii, PlayStation 3 XMB, Cyberpunk 2077, Synthwave '84, Casio Digital, macOS, Windows XP, Amiga, Commodore 64, and more).

## One-Command Installation

Installing from KDE Store provides the Plasma QML package. To install \`sunreactord\`, \`sunreactorctl\`, and the native C++ Plasma plugin in one step:

\`\`\`bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash
systemctl --user restart plasma-plasmashell.service
\`\`\`

### Arch / CachyOS (AUR)
\`\`\`bash
yay -S plasma6-applets-sunreactor
\`\`\`

### Uninstall
\`\`\`bash
curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash -s -- --uninstall
\`\`\`

- **Source & Docs:** https://github.com/arcanorca/SunReactor
- **License:** GPL-3.0-or-later
EOF

printf 'Packaged KDE Store plasmoid: %s\n' "$out_file"
printf 'Listing description: %s\n' "$desc_file"
