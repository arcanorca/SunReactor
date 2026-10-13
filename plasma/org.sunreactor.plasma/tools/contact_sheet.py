#!/usr/bin/env python3
"""Lays screenshots out on one labelled sheet for review.

  contact_sheet.py OUT.png IMG... [--columns N] [--scale F] [--title TEXT]
  contact_sheet.py OUT.png --dir DIR [--glob 'after_*_popup_*.png']

Images keep their device-pixel size (scale 1 by default, nearest-neighbour when
scaled) so pixel art is reviewed as rendered. Rows group by the label prefix
before the last '_' (e.g. theme_view), columns are whatever varies last
(usually the device pixel ratio).
"""

from __future__ import annotations

import argparse
import glob
import os
import sys

from PIL import Image, ImageDraw, ImageFont

BG = (24, 26, 28)
FG = (220, 222, 224)
MUTED = (140, 144, 150)


def load_font(size: int):
    for path in ("/usr/share/fonts/noto/NotoSans-Regular.ttf",
                 "/usr/share/fonts/TTF/DejaVuSans.ttf",
                 "/usr/share/fonts/liberation/LiberationSans-Regular.ttf"):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out")
    ap.add_argument("images", nargs="*")
    ap.add_argument("--dir")
    ap.add_argument("--glob", default="*.png")
    ap.add_argument("--columns", type=int, default=0, help="0 = one row per group")
    ap.add_argument("--scale", type=float, default=1.0)
    ap.add_argument("--title", default="")
    args = ap.parse_args()

    paths = list(args.images)
    if args.dir:
        paths += sorted(glob.glob(os.path.join(args.dir, args.glob)))
    paths = [p for p in paths if os.path.abspath(p) != os.path.abspath(args.out)]
    if not paths:
        print("no images", file=sys.stderr)
        return 1

    def label(p: str) -> str:
        return os.path.splitext(os.path.basename(p))[0]

    groups: dict[str, list[str]] = {}
    for p in paths:
        key = label(p).rsplit("_", 1)[0]
        groups.setdefault(key, []).append(p)
    rows = list(groups.values())
    if args.columns:
        flat = [p for row in rows for p in row]
        rows = [flat[i:i + args.columns] for i in range(0, len(flat), args.columns)]

    font = load_font(14)
    title_font = load_font(20)
    pad, label_h = 16, 22
    images = [[Image.open(p).convert("RGB") for p in row] for row in rows]
    if args.scale != 1.0:
        images = [[im.resize((max(1, round(im.width * args.scale)), max(1, round(im.height * args.scale))),
                             Image.NEAREST) for im in row] for row in images]

    col_count = max(len(r) for r in images)
    col_w = [0] * col_count
    for row in images:
        for i, im in enumerate(row):
            col_w[i] = max(col_w[i], im.width)
    row_h = [max(im.height for im in row) + label_h for row in images]
    top = pad + (34 if args.title else 0)
    width = pad + sum(w + pad for w in col_w)
    height = top + sum(h + pad for h in row_h)
    sheet = Image.new("RGB", (max(width, 400), height), BG)
    draw = ImageDraw.Draw(sheet)
    if args.title:
        draw.text((pad, pad), args.title, font=title_font, fill=FG)

    y = top
    for r, row in enumerate(images):
        x = pad
        for i, im in enumerate(row):
            draw.text((x, y), label(rows[r][i]), font=font, fill=MUTED)
            sheet.paste(im, (x, y + label_h))
            x += col_w[i] + pad
        y += row_h[r] + pad

    sheet.save(args.out)
    print(f"{args.out} ({sheet.width}x{sheet.height}, {len(paths)} images)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
