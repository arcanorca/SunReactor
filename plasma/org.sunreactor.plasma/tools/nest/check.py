#!/usr/bin/env python3
"""Regression matrix for the SunReactor popup in the nested session.

  check.py [--out DIR] [--themes a,b] [--fonts default,inter12]
           [--sizes plain,stored] [--panels bottom,top] [--jobs N]
  check.py --from DIR       assert over runs already in DIR (no sessions)

Every case is one private session (run.sh). The widget's own script (main.qml,
nest harness) opens the popup, switches Brightness -> Weather -> Brightness ->
Weather, closes it and reopens it. driver.py records the window timeline; this
samples it at the script's step times:

  first     the popup's first geometry
  open      Brightness, before the first tab step          (t = 1.3 s)
  weather   Weather                                        (t = 2.8 s)
  back      Brightness again                               (t = 4.3 s)
  again     Weather again                                  (t = 5.8 s)
  closed    the popup is closed                            (t = 7.0 s)
  reopen    reopened: Brightness, the start page           (t = 8.8 s)

Assertions per case:
  - the popup is shown at every sample but "closed", and it is inside the
    screen at every recorded moment;
  - the popup's bottom edge is on the panel's top edge (bottom panel) or its
    top edge on the panel's bottom edge (top panel), within 2 px;
  - the popup width does not change during the run (within 2 px; the first
    frame, Plasma's default size, is not counted);
  - the height at each page is that page's height within 4 px: open, back and
    reopen are Brightness, weather and again are Weather;
  - a stored case is compared with the plain case of the same theme, font and
    panel (Plasma sizes the window from the page when nothing is stored);
  - the plain case has Brightness and Weather at different heights (else the
    switch proves nothing); the config holds popupHeight/popupWidth only when
    a size was stored.
Exit status 1 when any assertion fails. Prints one row per case.
"""

import argparse
import concurrent.futures
import datetime
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
WIDGET = HERE.parent.parent
RUN = HERE / "run.sh"
SCREEN = (1600, 1000)
HEIGHT_TOL = 4
EDGE_TOL = 2
WIDTH_TOL = 2
MIN_PAGE_DIFF = 8
SAMPLES = [("first", 0.0), ("open", 1.3), ("weather", 2.8), ("back", 4.3),
           ("again", 5.8), ("closed", 7.0), ("reopen", 8.8)]
PAGE_HEIGHT = {"open": "B", "back": "B", "reopen": "B", "weather": "W", "again": "W"}


def case_name(case):
    theme, font, size, panel = case
    return "%s-%s-%s-%s" % (theme, font, size, panel)


def state_at(events, t):
    """The last recorded state at or before t."""
    current = None
    for event in events:
        if event["t"] <= t + 1e-9:
            current = event
        else:
            break
    return current


def assess(case_dir):
    """Per-case measurements and the assertions that do not need a reference."""
    lines = [json.loads(line) for line in (case_dir / "steps.jsonl").read_text().splitlines() if line.strip()]
    ready = next(r for r in lines if r["step"] == "ready")
    session = next(r for r in lines if r["step"] == "session")
    events, location = session["events"], session["location"]
    states = {name: state_at(events, t) for name, t in SAMPLES}
    fails = []
    heights, widths, edge = {}, [], 0
    for name, _ in SAMPLES:
        state = states[name]
        popup = state["popup"] if state else None
        if name == "closed":
            if popup is not None:
                fails.append("closed: the popup is still shown")
            continue
        if popup is None:
            fails.append("%s: no popup" % name)
            continue
        heights[name] = popup["h"]
        if name != "first":  # the first frame is Plasma's default size, before the layout
            widths.append(popup["w"])
        panel = state["panel"]
        if location == "bottom":
            dev = (popup["y"] + popup["h"]) - panel["y"]
        else:
            dev = popup["y"] - (panel["y"] + panel["h"])
        edge = max(edge, abs(dev))
    if edge > EDGE_TOL:
        fails.append("not on the %s panel edge: %d px" % (location, edge))
    if widths and max(widths) - min(widths) > WIDTH_TOL:
        fails.append("width changes: %s" % sorted(set(widths)))
    for event in events:
        p = event["popup"]
        if p and (p["x"] < 0 or p["y"] < 0 or p["x"] + p["w"] > SCREEN[0] or p["y"] + p["h"] > SCREEN[1]):
            fails.append("outside the screen at t=%.1f: %dx%d at %d,%d" % (event["t"], p["w"], p["h"], p["x"], p["y"]))
            break
    for name, _ in SAMPLES:
        if name in ("first", "closed") or name not in heights:
            continue
        ref = heights.get("open") if PAGE_HEIGHT[name] == "B" else heights.get("weather")
        if ref is not None and abs(heights[name] - ref) > HEIGHT_TOL:
            fails.append("%s is %d, page height %d" % (name, heights[name], ref))
    config = ready["config"]
    return {"heights": heights, "edge": edge, "width": (min(widths), max(widths)) if widths else None,
            "config": config, "location": location, "fails": fails}


def compare_stored(stored, plain):
    """A stored size must settle on the plain heights of the same theme, font and panel."""
    fails = []
    for name in ("open", "weather", "back", "again", "reopen"):
        if name not in stored["heights"] or name not in plain["heights"]:
            continue
        if abs(stored["heights"][name] - plain["heights"][name]) > HEIGHT_TOL:
            fails.append("%s %d, plain %d" % (name, stored["heights"][name], plain["heights"][name]))
    return fails


def check_config(result, size):
    config = result["config"]
    if size == "stored":
        if config.get("popupHeight") != "494" or config.get("popupWidth") != "342":
            return ["stored size missing from the config: %s" % config]
        return []
    if config.get("popupHeight") is not None or config.get("popupWidth") is not None:
        return ["a size is stored without a stored size: %s" % config]
    return []


def run_case(case, out):
    theme, font, size, panel = case
    directory = out / case_name(case)
    directory.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run([str(RUN), "--theme", theme, "--font", font, "--size", size,
                           "--panel", panel, "--out", str(directory)],
                          capture_output=True, text=True)
    (directory / "run.log").write_text(proc.stdout + proc.stderr)
    return proc.returncode


def row(case, result, status):
    theme, font, size, panel = case
    if result is None:
        return "%-9s %-8s %-6s %-6s  %s" % (theme, font, size, panel, status)
    h = result["heights"]
    cells = " ".join("%5s" % h.get(name, "-") for name in ("first", "open", "weather", "back", "again", "reopen"))
    return "%-9s %-8s %-6s %-6s %s  edge %2dpx  width %s  %s" % (
        theme, font, size, panel, cells, result["edge"],
        "%d-%d" % result["width"] if result["width"] else "-", status)


def main():
    ap = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    ap.add_argument("--out", default=str(WIDGET / "build" / "nest" / "check"))
    ap.add_argument("--from", dest="source", default="")
    ap.add_argument("--themes", default="braun,nothing,handheld,dracula")
    ap.add_argument("--fonts", default="default,inter12")
    ap.add_argument("--sizes", default="plain,stored")
    ap.add_argument("--panels", default="bottom,top")
    ap.add_argument("--jobs", type=int, default=4)
    args = ap.parse_args()

    cases = [(t, f, s, p) for t in args.themes.split(",") for f in args.fonts.split(",")
             for s in args.sizes.split(",") for p in args.panels.split(",")]
    out = pathlib.Path(args.source or args.out).resolve()
    run_codes = {}
    if not args.source:
        out.mkdir(parents=True, exist_ok=True)
        stamp = datetime.datetime.now().strftime("%H:%M:%S")
        print("check: %d cases, %d jobs, out %s (%s)" % (len(cases), args.jobs, out, stamp), flush=True)
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = {pool.submit(run_case, case, out): case for case in cases}
            for future in concurrent.futures.as_completed(futures):
                run_codes[futures[future]] = future.result()

    results, status = {}, {}
    for case in cases:
        directory = out / case_name(case)
        code = run_codes.get(case, 0)
        try:
            results[case] = assess(directory)
            status[case] = []
        except (OSError, StopIteration, KeyError, ValueError) as error:
            results[case] = None
            status[case] = ["no usable steps (%s); run exit %s" % (error.__class__.__name__, code)]
            continue
        if code != 0:
            status[case].append("run exit %d" % code)
        status[case] += results[case]["fails"]
        status[case] += check_config(results[case], case[2])
        if case[2] == "stored":
            plain = results.get((case[0], case[1], "plain", case[3]))
            if plain is None:
                status[case].append("no plain case to compare with")
            else:
                status[case] += ["vs plain: " + m for m in compare_stored(results[case], plain)]
        elif results[case]["heights"].get("weather") is not None and \
                abs(results[case]["heights"]["weather"] - results[case]["heights"]["open"]) < MIN_PAGE_DIFF:
            status[case].append("Brightness and Weather have the same height")

    print("%-9s %-8s %-6s %-6s %5s %5s %5s %5s %5s %5s" % (
        "theme", "font", "size", "panel", "first", "open", "weath", "back", "again", "reopen"))
    failed = 0
    for case in cases:
        verdict = "PASS" if not status[case] else "FAIL " + "; ".join(status[case])
        failed += bool(status[case])
        print(row(case, results[case], verdict))
    print("%d of %d cases pass" % (len(cases) - failed, len(cases)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
