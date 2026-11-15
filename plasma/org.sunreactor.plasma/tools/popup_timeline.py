#!/usr/bin/env python3
"""The popup's size over time, from a viewer log written by tools/shot.sh.

  tools/popup_timeline.py LOG [--all]

The harness containment (tools/harness) logs SUNREACTOR_HARNESS_POPUP records (the
popup's size, the window's frame-swap count and a millisecond clock) and
SUNREACTOR_HARNESS_EVENT records (page switches, expanded true/false). A size that is
replaced inside the same frame swap was never on screen; one that survives a swap was.
Without --all this prints, per run, only the sizes that were on screen (the last one
logged in each frame), with the events in between, so a jump on first open or on a tab
switch shows as consecutive lines:

    +0ms     f0  513x494
    +649ms   f2  513x527
    +649ms   f3  513x803
    +3526ms  f7  event page weather
    +3543ms  f7  513x551

Drive it with, for instance:
  tools/shot.sh --out DIR --themes handheld --views popup \\
      --config "open=1 tabs=weather,brightness reopen=1 openDelay=1500"
"""

from __future__ import annotations

import argparse
import json
import re
import sys

REC = re.compile(r"SUNREACTOR_HARNESS_(POPUP|EVENT) (\{.*\})")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("log")
    ap.add_argument("--all", action="store_true", help="print every record, including sizes replaced within a frame")
    args = ap.parse_args()

    records = []
    for line in open(args.log, errors="replace"):
        m = REC.search(line)
        if not m:
            continue
        try:
            d = json.loads(m.group(2))
        except ValueError:
            continue
        if "t" in d:
            records.append((m.group(1), d))
    if not records:
        print("no SUNREACTOR_HARNESS_POPUP records with a clock: is the harness current?", file=sys.stderr)
        return 1
    t0 = records[0][1]["t"]

    shown = []
    for i, (kind, d) in enumerate(records):
        if kind == "EVENT":
            shown.append((d, f"event {d['event']}"))
            continue
        nxt = next((r for r in records[i + 1:] if r[0] == "POPUP"), None)
        if args.all or nxt is None or nxt[1]["frame"] != d["frame"]:
            shown.append((d, f"{d['width']}x{d['height']}"))
    for d, text in shown:
        print(f"+{d['t'] - t0}ms".ljust(10) + f"f{d['frame']}".ljust(5) + text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
