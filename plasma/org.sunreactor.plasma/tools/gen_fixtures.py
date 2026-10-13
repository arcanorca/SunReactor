#!/usr/bin/env python3
"""Regenerate tools/fixtures/themes.json from the daemon's real theme data.

Every built-in theme is exported through the real CLI
(`sunreactorctl theme list --json` and `theme show <id> --json`), i.e. the
exact `status.theme` / `get_theme` object the daemon sends (DESIGN.md
section 3), family defaults included. The CLI runs with an empty, throwaway
XDG_CONFIG_HOME so user themes in ~/.config/sunreactor/themes never leak into
the fixtures.

Two synthetic entries exercise widget-side derivation and stay hand-written:
  edge_partial  - a widget block with only density and modules (extends-style)
  edge_noshades - pixel theme without shades/modules (shades derived in QML)

  tools/gen_fixtures.py [--ctl PATH]     default: cargo run -q --bin sunreactorctl
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
OUT = HERE / "fixtures" / "themes.json"


def run(ctl: list[str], args: list[str], env: dict[str, str]) -> str:
    return subprocess.run(ctl + args, cwd=REPO, env=env, check=True, capture_output=True, text=True,
                          timeout=600).stdout


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--ctl", help="path to a built sunreactorctl (default: cargo run)")
    args = ap.parse_args()
    ctl = [args.ctl] if args.ctl else ["cargo", "run", "-q", "--bin", "sunreactorctl", "--"]

    with tempfile.TemporaryDirectory(prefix="srfix.") as empty:
        env = dict(os.environ, XDG_CONFIG_HOME=empty)
        summaries = json.loads(run(ctl, ["theme", "list", "--json"], env))
        themes: dict[str, dict] = {}
        for summary in summaries:
            tid = summary["id"]
            themes[tid] = json.loads(run(ctl, ["theme", "show", tid, "--json"], env))

    if len(themes) != 35:
        raise SystemExit(f"expected 35 built-in themes, the CLI listed {len(themes)}")

    c64 = json.loads(json.dumps(themes["commodore64"]))
    c64["id"], c64["name"] = "edge_noshades", "Edge: no shades"
    for key in ("shades", "modules"):
        c64["widget"].pop(key, None)
    nord = json.loads(json.dumps(themes["nord"]))
    nord["id"], nord["name"] = "edge_partial", "Edge: partial widget"
    nord["widget"] = {"density": "comfortable",
                      "modules": ["telemetry", "displays", "curve", "automation", "suntimes"]}

    doc = {
        "_comment": "status.theme objects exported by `sunreactorctl theme show <id> --json` for all 35 "
                    "built-ins (regenerate with tools/gen_fixtures.py; do not edit by hand). The two "
                    "edge_* entries are synthetic and exercise widget-side derivation.",
        **themes,
        "edge_noshades": c64,
        "edge_partial": nord,
    }
    OUT.write_text(json.dumps(doc, indent=1) + "\n")
    families: dict[str, list[str]] = {}
    for tid, t in themes.items():
        families.setdefault(t["family"], []).append(tid)
    for fam, ids in sorted(families.items()):
        print(f"{fam:10} {len(ids):2}  {' '.join(ids)}")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
