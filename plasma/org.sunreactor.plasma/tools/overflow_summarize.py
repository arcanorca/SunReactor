#!/usr/bin/env python3
"""Summarise a tools/overflow_audit.sh run: CLI table, JSON and Markdown reports.

  tools/overflow_summarize.py --run DIR [--top 3] [--package DIR]

Reads DIR/manifest.tsv (one row per combination), DIR/logs/<name>.log (the
viewer's log with the SUNREACTOR_AUDIT_* lines of tools/harness/Audit.qml) and
DIR/shots/<name>.png. Writes DIR/overflow_report.json and DIR/overflow_report.md
and prints the per-theme counts, worst first.

A finding is one defect of one item. The same defect seen in several
combinations (scale, DPI, scenario, page, popup width) is one signature with a
count of combinations; counts per theme are distinct signatures, so the table
says how many defects a theme has, not how often they were seen.
"""

from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import pathlib
import re
import sys

DEFECT_KINDS = ("OVERFLOW", "CLIPPED", "COLLISION")
DECOR_PATTERN = re.compile(r"Glow|MultiEffect|ShaderEffect|QGfxSourceProxy|MouseArea")
KINDS = DEFECT_KINDS + ("ELIDED", "DECOR")
MARKER = re.compile(r"SUNREACTOR_AUDIT_(META|FINDING|END|ERROR) (.*)$")
DEFAULT_PACKAGE = pathlib.Path(__file__).resolve().parent.parent / "package"


def parse_log(path: pathlib.Path) -> dict:
    result: dict = {"meta": None, "findings": [], "end": None, "errors": []}
    if not path.exists():
        result["errors"].append("no log")
        return result
    for line in path.read_text(errors="replace").splitlines():
        match = MARKER.search(line)
        if not match:
            continue
        kind, payload = match.group(1), match.group(2)
        try:
            value = json.loads(payload)
        except json.JSONDecodeError:
            result["errors"].append(f"unparsable {kind}")
            continue
        if kind == "META":
            result["meta"] = value
        elif kind == "FINDING":
            result["findings"].append(value)
        elif kind == "END":
            result["end"] = value
        else:
            result["errors"].append(str(value))
    return result


class FileIndex:
    """QML files of the package by base name, to give a finding a file hint."""

    def __init__(self, package: pathlib.Path) -> None:
        self.ui = package / "contents" / "ui"
        self.by_base: dict[str, list[str]] = collections.defaultdict(list)
        if self.ui.is_dir():
            for path in sorted(self.ui.rglob("*.qml")):
                self.by_base[path.stem].append(str(path.relative_to(self.ui)))

    def relative(self, url: str) -> str:
        if not url:
            return ""
        marker = "/contents/ui/"
        if marker in url:
            return url.split(marker, 1)[1]
        return url

    def hint(self, file_url: str, path: list[str]) -> str:
        if file_url:
            return self.relative(file_url)
        for label in reversed(path):
            base = label.split("#", 1)[0]
            options = self.by_base.get(base, [])
            if len(options) == 1:
                return options[0]
        return ""


def tail(path: list[str], count: int = 3) -> str:
    return "/".join(path[-count:])


def is_decorative(finding: dict) -> bool:
    """Glow layers and mouse hit areas: they paint or catch input beyond their card on purpose."""
    path = finding.get("path", [])
    return bool(path) and any(DECOR_PATTERN.search(label) for label in path[-2:])


def finding_key(kind: str, finding: dict, hint: str) -> str:
    # One overflowing item is one finding, but a panel whose row spills takes
    # many items with it: an overflow is grouped by the container it leaves.
    if kind == "OVERFLOW":
        return f"OVERFLOW|{finding.get('against', '')}|{finding.get('label', '')}|{hint}"
    return f"{kind}|{finding.get('against', '')}|{tail(finding.get('path', []))}|{hint}"


def px_of(finding: dict) -> float:
    if "px" in finding:
        return float(finding["px"])
    detail = finding.get("detail") or {}
    if "natural" in detail and "maximumArtWidth" in detail:
        return float(detail["natural"]) - float(detail["maximumArtWidth"])
    if "contentWidth" in detail and "width" in detail:
        return float(detail["contentWidth"]) - float(detail["width"])
    if "overlapX" in finding.get("detail", {}):
        return min(finding["detail"]["overlapX"], finding["detail"]["overlapY"])
    return 0.0


def build(run: pathlib.Path, package: pathlib.Path, top: int) -> dict:
    index = FileIndex(package)
    rows = []
    with open(run / "manifest.tsv") as handle:
        header = handle.readline().rstrip("\n").split("\t")
        for line in handle:
            if line.strip():
                rows.append(dict(zip(header, line.rstrip("\n").split("\t"))))

    combos = []
    signatures: dict[str, dict] = {}
    per_theme: dict[str, dict] = {}
    incomplete = []
    for row in rows:
        name = row["name"]
        log = parse_log(run / "logs" / f"{name}.log")
        meta = log["meta"] or {}
        complete = log["end"] is not None
        if not complete:
            incomplete.append({"name": name, "errors": log["errors"] or ["no END marker"]})
        elif log["end"].get("pending"):
            incomplete.append({"name": name, "errors": [f"{log['end']['pending']} Loader/Image still loading"]})
        counts = collections.Counter(f["kind"] for f in log["findings"])
        theme = per_theme.setdefault(row["theme"], {"seen": collections.Counter(),
                                                    "combos_with_defects": 0, "incomplete": 0,
                                                    "heights": [], "widths": [], "scrolled": 0})
        if not complete:
            theme["incomplete"] += 1
        if any(counts.get(k) for k in DEFECT_KINDS):
            theme["combos_with_defects"] += 1
        for finding in log["findings"]:
            kind = finding["kind"]
            if kind == "OVERFLOW" and is_decorative(finding):
                kind = "DECOR"
            hint = index.hint(finding.get("file", ""), finding.get("path", []))
            key = finding_key(kind, finding, hint)
            sig = signatures.setdefault(key, {
                "key": key, "theme": row["theme"], "kind": kind, "against": finding.get("against", ""),
                "path": tail(finding.get("path", []), 4), "file": hint, "text": finding.get("text", ""),
                "combos": [], "max_px": 0.0, "example": None, "items": set(),
            })
            sig["combos"].append(name)
            sig["items"].add(tail(finding.get("path", []), 4))
            if px_of(finding) >= sig["max_px"]:
                sig["max_px"] = round(px_of(finding), 2)
                sig["example"] = {"name": name, "box": finding.get("box"), "bound": finding.get("bound"),
                                  "edge": finding.get("edge"), "px": finding.get("px"),
                                  "detail": finding.get("detail"), "other": finding.get("other")}
            if sig["text"] == "" and finding.get("text"):
                sig["text"] = finding["text"]
            theme["seen"][kind] += 1
        if isinstance(meta.get("popup"), dict):
            theme["heights"].append(meta["popup"]["h"])
            theme["widths"].append(meta["popup"]["w"])
        if any(entry.get("needsVertical") for entry in meta.get("scroll", [])):
            theme["scrolled"] += 1
        combos.append({
            "name": name, "theme": row["theme"], "page": row["page"], "dpr": row["dpr"],
            "dpi": row["dpi"], "scenario": row["scen"], "units": row["units"],
            "fontpt": row.get("fontpt", ""),
            "effective_dpr": meta.get("dpr"), "popup": meta.get("popup"),
            "popup_max_height": meta.get("popupMaxHeight"), "scroll": meta.get("scroll", []),
            "complete": complete, "findings": len(log["findings"]), "shot": f"shots/{name}.png",
            "kinds": dict(counts),
        })

    for sig in signatures.values():
        sig["combos"] = sorted(set(sig["combos"]))
        sig["items"] = sorted(sig["items"])

    sig_counts = collections.Counter((s["theme"], s["kind"]) for s in signatures.values())
    themes = []
    for theme_id, data in per_theme.items():
        sigs = {kind: sig_counts[(theme_id, kind)] for kind in KINDS}
        worst = max((s for s in signatures.values() if s["theme"] == theme_id),
                    key=lambda s: (s["kind"] in DEFECT_KINDS, len(s["combos"]), s["max_px"]), default=None)
        themes.append({
            "theme": theme_id,
            "OVERFLOW": sigs.get("OVERFLOW", 0), "CLIPPED": sigs.get("CLIPPED", 0),
            "COLLISION": sigs.get("COLLISION", 0), "ELIDED": sigs.get("ELIDED", 0),
            "DECOR": sigs.get("DECOR", 0),
            "defects": sum(sigs.get(k, 0) for k in DEFECT_KINDS),
            "occurrences": dict(data["seen"]),
            "combos_with_defects": data["combos_with_defects"],
            "incomplete": data["incomplete"],
            "worst": worst["key"] if worst else None,
            "popup_height": size_range(data["heights"]),
            "popup_width": size_range(data["widths"]),
            "scrolled_combos": data["scrolled"],
        })
    themes.sort(key=lambda t: (-t["defects"], -t["ELIDED"], t["theme"]))

    top_by_theme = {}
    for theme in themes:
        mine = [s for s in signatures.values() if s["theme"] == theme["theme"]]
        mine.sort(key=lambda s: (s["kind"] not in DEFECT_KINDS, -len(s["combos"]), -s["max_px"], s["key"]))
        top_by_theme[theme["theme"]] = mine[:top]

    return {
        "generated": dt.datetime.now().isoformat(timespec="seconds"),
        "run": str(run),
        "combinations": len(combos),
        "incomplete": incomplete,
        "themes": themes,
        "top": top_by_theme,
        "signatures": sorted(signatures.values(), key=lambda s: (s["theme"], s["kind"], s["key"])),
        "combos": combos,
    }


def size_range(values: list) -> str:
    if not values:
        return "-"
    low, high = min(values), max(values)
    return f"{low:g}" if low == high else f"{low:g}-{high:g}"


def cli_table(report: dict) -> str:
    lines = [f"{'theme':18} {'OVERFLOW':>8} {'CLIPPED':>7} {'COLLIDE':>7} {'ELIDED':>6} {'DECOR':>5} {'SCROLL':>6}  top offender"]
    for theme in report["themes"]:
        top = report["top"].get(theme["theme"], [])
        first = top[0] if top else None
        label = "" if first is None else f"{first['kind']} {first['path']} [{first['file']}] x{len(first['combos'])} combos"
        lines.append(f"{theme['theme']:18} {theme['OVERFLOW']:>8} {theme['CLIPPED']:>7} {theme['COLLISION']:>7} "
                     f"{theme['ELIDED']:>6} {theme['DECOR']:>5} {theme['scrolled_combos']:>6}  {label}")
    if report["incomplete"]:
        lines.append(f"incomplete runs: {len(report['incomplete'])} (see overflow_report.json)")
    return "\n".join(lines)


def markdown(report: dict, run: pathlib.Path) -> str:
    out = []
    out.append("# Popup layout audit")
    out.append("")
    out.append(f"Run: `{run}`, generated {report['generated']}, {report['combinations']} combinations.")
    out.append("")
    out.append("Counts are distinct defects per theme (a defect seen in several combinations counts once).")
    out.append("OVERFLOW: an item leaves its frame, clipping item, scroll viewport or the popup horizontally.")
    out.append("CLIPPED: text cut without an ellipsis. COLLISION: two text items in one row overlap.")
    out.append("ELIDED: text cut with an ellipsis (often intended, but the status line should not lose words).")
    out.append("DECOR: overflow of glow layers and mouse hit areas (painted or clickable beyond the card on purpose; not counted as defects).")
    out.append("")
    out.append("| theme | OVERFLOW | CLIPPED | COLLISION | ELIDED | DECOR | combos with defects | popup w (px) | popup h (px) | combos needing a scrollbar |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for theme in report["themes"]:
        out.append(f"| {theme['theme']} | {theme['OVERFLOW']} | {theme['CLIPPED']} | {theme['COLLISION']} | "
                   f"{theme['ELIDED']} | {theme['DECOR']} | {theme['combos_with_defects']} | {theme['popup_width']} | "
                   f"{theme['popup_height']} | {theme['scrolled_combos']} |")
    out.append("")
    for theme in report["themes"]:
        top = report["top"].get(theme["theme"], [])
        if not top:
            continue
        out.append(f"## {theme['theme']}")
        out.append("")
        for sig in top:
            example = sig["example"] or {}
            where = sig["file"] or "(file unknown)"
            detail = example.get("detail") or {}
            extra = []
            if example.get("edge"):
                extra.append(f"{example['edge']} edge by {example['px']} px vs {sig['against']}")
            if detail:
                extra.append(", ".join(f"{k}={v}" for k, v in detail.items() if v is not None))
            if sig["text"]:
                extra.append(f"text \"{sig['text'][:60]}\"")
            if sig["kind"] == "OVERFLOW":
                head = f"OVERFLOW out of `{sig['key'].split('|')[2] or '?'}` ({sig['against']})"
            else:
                head = f"**{sig['kind']}** `{sig['path']}`"
            out.append(f"- {head} in `{where}`, {len(sig['items'])} item(s) "
                       f"({len(sig['combos'])} combinations, e.g. `{sig['combos'][0]}`)")
            if extra:
                out.append(f"  - {'; '.join(extra)}")
        out.append("")
    if report["incomplete"]:
        out.append("## Incomplete runs")
        out.append("")
        for item in report["incomplete"]:
            out.append(f"- `{item['name']}`: {', '.join(item['errors'])}")
        out.append("")
    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--run", required=True, type=pathlib.Path)
    parser.add_argument("--top", type=int, default=3)
    parser.add_argument("--package", type=pathlib.Path, default=DEFAULT_PACKAGE)
    args = parser.parse_args()
    run = args.run.resolve()
    report = build(run, args.package.resolve(), args.top)
    (run / "overflow_report.json").write_text(json.dumps(report, indent=1) + "\n")
    (run / "overflow_report.md").write_text(markdown(report, run))
    print(cli_table(report))
    print(f"wrote {run / 'overflow_report.md'} and {run / 'overflow_report.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
