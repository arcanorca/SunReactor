#!/usr/bin/env python3
"""Pixel-palette conformance check (DESIGN.md section 4, pixel rule 5).

A pixel-themed area must contain only colours from the theme's palette and
shades - ordered dither included, since dither only ever picks palette entries.
Any intermediate colour (anti-aliasing, linear filtering, a fractional scale,
blending) is a failure.

  conformance.py SHOT.png --theme handheld            palette from fixtures/themes.json
  conformance.py SHOT.png --colors '#141c12,#9bbc0f'  explicit palette
  conformance.py SHOT.png --theme handheld --box 10,20,300,40
  conformance.py SHOT.png --theme handheld --regions SHOT.regions.json
  conformance.py ... --diff offenders.png --tolerance 0

--regions takes the JSON the harness writes ([{"x":..,"y":..,"w":..,"h":..,
"name":..}, ...] in device pixels); every region is checked separately.
Exit status: 0 = all regions conform, 1 = at least one offender, 2 = usage.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from PIL import Image

try:
    import numpy as np
except ImportError:  # pure-python fallback, slower but identical
    np = None

HERE = Path(__file__).resolve().parent


def hex_rgb(value: str) -> tuple[int, int, int]:
    v = value.strip().lstrip("#")
    if len(v) != 6:
        raise ValueError(f"not #rrggbb: {value!r}")
    return int(v[0:2], 16), int(v[2:4], 16), int(v[4:6], 16)


def theme_palette(theme_id: str) -> list[tuple[int, int, int]]:
    themes = json.loads((HERE / "fixtures" / "themes.json").read_text())
    theme = themes[theme_id]
    colours = list(theme["palette"].values())
    colours += theme.get("widget", {}).get("shades", []) or []
    return sorted({hex_rgb(c) for c in colours})


def check(img: Image.Image, palette, box, tolerance: int):
    region = img.crop(box) if box else img
    if np is not None:
        arr = np.asarray(region.convert("RGB"), dtype=np.int16)
        pal = np.asarray(palette, dtype=np.int16)
        # distance of every pixel to its nearest palette entry (max channel delta)
        dist = np.abs(arr[:, :, None, :] - pal[None, None, :, :]).max(axis=3).min(axis=2)
        bad_mask = dist > tolerance
        bad = int(bad_mask.sum())
        offenders = Counter()
        if bad:
            ys, xs = np.nonzero(bad_mask)
            for y, x in zip(ys[:20000], xs[:20000]):
                offenders[tuple(int(c) for c in arr[y, x])] += 1
        total = arr.shape[0] * arr.shape[1]
        coords = list(zip(*np.nonzero(bad_mask)))[:1] if bad else []
        first = (int(coords[0][1]), int(coords[0][0])) if coords else None
        return total, bad, offenders, bad_mask, first
    pixels = list(region.convert("RGB").getdata())
    w = region.width
    bad_mask = []
    offenders = Counter()
    first = None
    for i, p in enumerate(pixels):
        d = min(max(abs(p[0] - q[0]), abs(p[1] - q[1]), abs(p[2] - q[2])) for q in palette)
        is_bad = d > tolerance
        bad_mask.append(is_bad)
        if is_bad:
            offenders[p] += 1
            if first is None:
                first = (i % w, i // w)
    return len(pixels), sum(bad_mask), offenders, bad_mask, first


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("image")
    ap.add_argument("--theme")
    ap.add_argument("--colors")
    ap.add_argument("--box", help="x,y,w,h in device pixels")
    ap.add_argument("--regions", help="JSON list of {x,y,w,h[,name]} in device pixels")
    ap.add_argument("--tolerance", type=int, default=0, help="max per-channel delta still accepted")
    ap.add_argument("--diff", help="write a copy with offending pixels painted magenta")
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    palette = []
    if args.theme:
        palette += theme_palette(args.theme)
    if args.colors:
        palette += [hex_rgb(c) for c in args.colors.split(",") if c.strip()]
    if not palette:
        print("need --theme or --colors", file=sys.stderr)
        return 2
    palette = sorted(set(palette))

    img = Image.open(args.image).convert("RGB")
    regions = []
    if args.regions:
        for r in json.loads(Path(args.regions).read_text()):
            regions.append((r.get("name", "region"), (r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"])))
    if args.box:
        x, y, w, h = (int(v) for v in args.box.split(","))
        regions.append(("box", (x, y, x + w, y + h)))
    if not regions:
        regions.append(("image", None))

    diff = img.copy() if args.diff else None
    failed = False
    for name, box in regions:
        total, bad, offenders, mask, first = check(img, palette, box, args.tolerance)
        ok = bad == 0
        failed |= not ok
        span = f"{box}" if box else f"(0, 0, {img.width}, {img.height})"
        print(f"{'PASS' if ok else 'FAIL'} {name} {span}: {bad}/{total} off-palette pixels"
              + ("" if ok else f" ({100.0 * bad / total:.2f}%), first at {first}"))
        if not ok and not args.quiet:
            for colour, n in offenders.most_common(6):
                print(f"    #{colour[0]:02x}{colour[1]:02x}{colour[2]:02x} x{n}")
        if diff is not None and bad:
            ox, oy = (box[0], box[1]) if box else (0, 0)
            if np is not None:
                ys, xs = np.nonzero(mask)
                for y, x in zip(ys, xs):
                    diff.putpixel((int(x) + ox, int(y) + oy), (255, 0, 255))
            else:
                w = (box[2] - box[0]) if box else img.width
                for i, b in enumerate(mask):
                    if b:
                        diff.putpixel((i % w + ox, i // w + oy), (255, 0, 255))
    if diff is not None:
        diff.save(args.diff)
    if not args.quiet:
        print("palette:", " ".join(f"#{r:02x}{g:02x}{b:02x}" for r, g, b in palette))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
