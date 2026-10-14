#!/usr/bin/env python3
"""Stall sweep: one nested session per built-in theme, default font, no stored size.

  sweep.py [--part N/M] [--themes a,b] [--jobs 4] [--out DIR] [--write FILE] [--report]

Each session is run.sh's script: the popup opens, switches Brightness -> Weather
-> Brightness -> Weather, closes and reopens. A session is a STALL when, in the
12 s after the popup first shows, either
  - the plasmashell main thread runs above 90% CPU for more than 6 s, or
  - the fake daemon logs no status poll between 1 s and 11 s after it shows.
Results accumulate in DIR/results.json; --report only rewrites the markdown.
"""

import argparse
import concurrent.futures
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
WIDGET = HERE.parent.parent
RUN = HERE / "run.sh"
sys.path.insert(0, str(HERE))
import check  # noqa: E402  (the per-session measurements)

STALL_PCT = 90.0
STALL_SECONDS = 6.0
WINDOW = 12.0


def fixture_themes():
    data = json.loads((WIDGET / "tools" / "fixtures" / "themes.json").read_text())
    return [key for key in data if not key.startswith("_")]


def cpu_stall_seconds(samples, start):
    """The longest stretch above STALL_PCT on the main thread inside the window."""
    points = [(t, c) for t, c in samples if start <= t <= start + WINDOW]
    longest, run_start = 0.0, None
    for (t0, c0), (t1, c1) in zip(points, points[1:]):
        if t1 <= t0:
            continue
        if (c1 - c0) / (t1 - t0) * 100 > STALL_PCT:
            run_start = t0 if run_start is None else run_start
            longest = max(longest, t1 - run_start)
        else:
            run_start = None
    return longest


def status_polls(requests_log, start):
    polls = 0
    for line in requests_log.read_text().splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        if row.get("request") == "status" and start + 1 <= row["t"] <= start + 11:
            polls += 1
    return polls


def analyse(directory, code):
    result = {"verdict": "ERROR", "heights": {}, "cpu": 0.0, "polls": 0, "note": ""}
    try:
        lines = [json.loads(line) for line in (directory / "steps.jsonl").read_text().splitlines() if line.strip()]
    except OSError:
        lines = []
    session = next((r for r in lines if r["step"] == "session"), None)
    if session is None or session.get("first_ms") is None:
        result["verdict"] = "NO POPUP" if code == 0 else "ERROR"
        result["note"] = "the popup never showed" if code == 0 else "run.sh exit %d" % code
        return result
    start = session["first_ms"] / 1000.0
    cpu = next((r for r in lines if r["step"] == "cpu"), {"samples": []})
    result["cpu"] = cpu_stall_seconds(cpu["samples"], start)
    result["polls"] = status_polls(directory / "requests.log", start)
    try:
        measured = check.assess(directory)
        result["heights"] = measured["heights"]
        result["note"] = "; ".join(measured["fails"])[:90]
    except (KeyError, StopIteration, ValueError) as error:
        result["note"] = "no usable steps (%s)" % error.__class__.__name__
    stalled = result["cpu"] >= STALL_SECONDS or result["polls"] == 0
    if stalled:
        result["verdict"] = "STALL"
    elif code == 0:
        result["verdict"] = "OK"
    else:
        result["verdict"] = "ERROR"
        result["note"] = "run.sh exit %d; %s" % (code, result["note"])
    return result


def run_theme(theme, out):
    directory = out / theme
    directory.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run([str(RUN), "--theme", theme, "--font", "default", "--size", "plain",
                           "--panel", "bottom", "--out", str(directory)],
                          capture_output=True, text=True)
    (directory / "run.log").write_text(proc.stdout + proc.stderr)
    return analyse(directory, proc.returncode)


def render(themes, results):
    lines = [
        "# Stall sweep",
        "",
        "One nested session per built-in theme (`tools/nest/sweep.py`), default font, no stored",
        "size, bottom panel. Each session: open, Brightness -> Weather -> Brightness -> Weather,",
        "close, reopen. STALL = plasmashell's main thread above 90% CPU for more than 6 s in the",
        "12 s after the popup shows, or no daemon status poll 1-11 s after it shows. Heights are",
        "the popup's window height in px at open / weather / back / reopen.",
        "",
        "| theme | verdict | open | weather | back | reopen | main-thread busy | status polls 1-11 s | checks |",
        "|---|---|---:|---:|---:|---:|---:|---:|---|",
    ]
    counts = {}
    for theme in themes:
        r = results.get(theme)
        if r is None:
            lines.append("| %s | not run | | | | | | | |" % theme)
            continue
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
        h = r["heights"]
        lines.append("| %s | %s | %s | %s | %s | %s | %.0f s | %d | %s |" % (
            theme, r["verdict"], h.get("open", "-"), h.get("weather", "-"), h.get("back", "-"),
            h.get("reopen", "-"), r["cpu"], r["polls"], r["note"] or "ok"))
    lines += ["", "Totals: " + ", ".join("%s %d" % (k, v) for k, v in sorted(counts.items())) + "."]
    lines += [
        "",
        "Notes",
        "",
        "- The handheld stall was a feedback loop in the page layout, fixed in",
        "  `FullRepresentation.qml`: a page's column took its width from the scroll view's",
        "  available width, so a scrollbar that appeared narrowed the text; the taller",
        "  three-line status line then made the page scroll again, and the width toggled",
        "  every pass (main thread busy in `Repeater::setModel`). The column now keeps one",
        "  grid unit for the scrollbar whether or not the page scrolls.",
        "- Sessions with a stored popup size are not part of this sweep (see `check.py`).",
    ]
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    ap.add_argument("--part", default="", help="N/M: run the Nth of M slices of the themes")
    ap.add_argument("--themes", default="")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--out", default=str(WIDGET / "build" / "nest" / "sweep"))
    ap.add_argument("--write", default=str(HERE / "last_sweep.md"))
    ap.add_argument("--report", action="store_true")
    args = ap.parse_args()

    themes = fixture_themes()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    results_path = out / "results.json"
    results = json.loads(results_path.read_text()) if results_path.exists() else {}
    run = themes
    if args.part:
        n, m = (int(x) for x in args.part.split("/"))
        run = themes[(n - 1) * len(themes) // m: n * len(themes) // m]
    if args.themes:
        run = args.themes.split(",")
    if not args.report:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = {pool.submit(run_theme, theme, out): theme for theme in run}
            for future in concurrent.futures.as_completed(futures):
                theme = futures[future]
                results[theme] = future.result()
                results_path.write_text(json.dumps(results, indent=1))
                r = results[theme]
                print("%-18s %-8s open %s weather %s back %s reopen %s cpu %.0fs polls %d %s" % (
                    theme, r["verdict"], r["heights"].get("open", "-"), r["heights"].get("weather", "-"),
                    r["heights"].get("back", "-"), r["heights"].get("reopen", "-"), r["cpu"], r["polls"],
                    r["note"]), flush=True)
    pathlib.Path(args.write).write_text(render(themes, results))
    print("wrote", args.write, "(%d themes with results)" % len(results))
    return 0


if __name__ == "__main__":
    sys.exit(main())
