#!/usr/bin/env python3
"""Pixel-exact comparison of two screenshot directories (tools/shot.sh output).

  tools/pixel_diff.py GOLDEN_DIR CANDIDATE_DIR [--diff-out DIR] [--logs] [--quiet]

Every PNG of GOLDEN_DIR is matched by file name against CANDIDATE_DIR. Per image
the changed-pixel count (any channel differs) is reported; for each changed image
a diff PNG is written to --diff-out (the candidate dimmed, changed pixels in red).
A missing or differently sized image counts as changed. Extra images in the
candidate are reported too.

--logs also compares the viewer logs (the .log next to each PNG): after
normalising the throw-away paths (/tmp/srshot.XXXX) and dropping the popup-size
and terminal-frame records (diagnostics), a line that appears in the candidate log but not in
the golden one is a new warning and counts as a difference.

Exit status 0 only when every image has exactly 0 changed pixels (and, with
--logs, no new log lines). Used to prove that a refactor is not visible.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter
from pathlib import Path

import numpy as np
from PIL import Image

TMP_RE = re.compile(r"/tmp/srshot\.[A-Za-z0-9]+")
NUM_RE = re.compile(r"0x[0-9a-f]+")
# The golden is captured from a copy of the package: compare file names and messages, not
# where the package lived or which line a warning came from.
URL_RE = re.compile(r"file://\S*?/contents/ui/")
POS_RE = re.compile(r"(\.qml|\.js):\d+(:\d+)?")


def load(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.uint8)


def write_diff(gold: np.ndarray, cand: np.ndarray | None, mask: np.ndarray | None, out: Path) -> None:
    base = cand if cand is not None else gold
    dim = (base.astype(np.float32) * 0.35).astype(np.uint8)
    dim[..., 3] = 255
    if mask is not None:
        dim[mask] = (255, 0, 0, 255)
    Image.fromarray(dim, "RGBA").save(out)


def log_lines(path: Path) -> Counter:
    lines: Counter = Counter()
    if not path.exists():
        return lines
    for raw in path.read_text(errors="replace").splitlines():
        # Diagnostic records, not warnings: the popup size (checked by the PNGs) and the terminal
        # frame report, which a 1.5 s timer may or may not have emitted when the viewer is stopped.
        if "SUNREACTOR_HARNESS_POPUP" in raw or "SUNREACTOR_TERM_FRAME" in raw:
            continue
        norm = POS_RE.sub(r"\1", URL_RE.sub("", TMP_RE.sub("/tmp/srshot.X", raw)))
        lines[NUM_RE.sub("0xN", norm)] += 1
    return lines


def dilate(m: np.ndarray) -> np.ndarray:
    out = m.copy()
    out[1:] |= m[:-1]
    out[:-1] |= m[1:]
    out[:, 1:] |= m[:, :-1]
    out[:, :-1] |= m[:, 1:]
    out[1:, 1:] |= m[:-1, :-1]
    out[:-1, :-1] |= m[1:, 1:]
    out[1:, :-1] |= m[:-1, 1:]
    out[:-1, 1:] |= m[1:, :-1]
    return out


def strict(args) -> int:
    bad = 0
    worst_out = 0
    for g in sorted(args.golden.glob("*.png")):
        c, n = args.candidate / g.name, args.noise / g.name
        if not c.exists() or not n.exists():
            print(f"MISSING  {g.name}")
            bad += 1
            continue
        a, b, o = load(g), load(c), load(n)
        if a.shape != b.shape or a.shape != o.shape:
            print(f"SIZE     {g.name}")
            bad += 1
            continue
        noise = np.any(a != o, axis=-1)
        mask = dilate(noise)
        diff = np.any(a != b, axis=-1)
        outside = int((diff & ~mask).sum())
        inside = int((diff & mask).sum())
        base = int(noise.sum())
        worst_out = max(worst_out, outside)
        fail = outside > 0 or inside > base
        bad += fail
        if fail or not args.quiet:
            print(f"{'FAIL' if fail else 'ok  '}     {g.name}: outside-mask {outside}, inside {inside} (noise g1-g2 {base}, mask {int(mask.sum())})")
            if fail and args.diff_out:
                args.diff_out.mkdir(parents=True, exist_ok=True)
                write_diff(a, b, diff & ~mask, args.diff_out / g.name)
    print(f"strict summary: {bad} failing images, max outside-mask diff {worst_out}")
    return 1 if bad else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("golden", type=Path)
    ap.add_argument("candidate", type=Path)
    ap.add_argument("--diff-out", type=Path)
    ap.add_argument("--also", type=Path, action="append", default=[],
                    help="further golden directory (an independent capture of the same tree); a pixel "
                         "counts as changed only if it differs from the golden AND from every --also")
    ap.add_argument("--noise", type=Path, metavar="GOLDEN2",
                    help="strict mode: GOLDEN2 is a second capture of the golden tree. Per image the noise "
                         "mask is every pixel where GOLDEN and GOLDEN2 differ (tolerance 0), dilated by 1 px. "
                         "Pass = 0 candidate pixels differing from GOLDEN outside the mask, and no more inside "
                         "the mask than GOLDEN differs from GOLDEN2")
    ap.add_argument("--tolerance", type=int, default=0,
                    help="largest per-channel difference still counted as equal (default 0 = exact)")
    ap.add_argument("--logs", action="store_true", help="also report new warnings in the viewer logs")
    ap.add_argument("--quiet", action="store_true", help="print only changed images and the summary")
    args = ap.parse_args()

    if args.noise:
        return strict(args)
    gold_files = sorted(p for p in args.golden.glob("*.png"))
    if not gold_files:
        print(f"no PNGs in {args.golden}", file=sys.stderr)
        return 2
    if args.diff_out:
        args.diff_out.mkdir(parents=True, exist_ok=True)

    changed = missing = identical = 0
    total_px = 0
    log_changes = 0
    for g in gold_files:
        c = args.candidate / g.name
        if not c.exists():
            print(f"MISSING  {g.name}")
            missing += 1
            continue
        a, b = load(g), load(c)
        if a.shape != b.shape:
            print(f"SIZE     {g.name}: {a.shape[1]}x{a.shape[0]} -> {b.shape[1]}x{b.shape[0]}")
            if args.diff_out:
                write_diff(a, b, None, args.diff_out / g.name)
            changed += 1
            continue
        def differs(x: np.ndarray, y: np.ndarray) -> np.ndarray:
            return np.any(np.abs(x.astype(np.int16) - y.astype(np.int16)) > args.tolerance, axis=-1)

        mask = differs(a, b)
        for extra_dir in args.also:
            other = extra_dir / g.name
            if other.exists():
                o = load(other)
                if o.shape == b.shape:
                    mask &= differs(o, b)
        n = int(mask.sum())
        if n:
            ys, xs = np.nonzero(mask)
            print(f"CHANGED  {g.name}: {n} px, bbox x {xs.min()}..{xs.max()} y {ys.min()}..{ys.max()}")
            if args.diff_out:
                write_diff(a, b, mask, args.diff_out / g.name)
            changed += 1
            total_px += n
        else:
            identical += 1
            if not args.quiet:
                print(f"same     {g.name}")
        if args.logs:
            new = log_lines(c.with_suffix(".log")) - log_lines(g.with_suffix(".log"))
            for line, count in sorted(new.items()):
                print(f"NEWLOG   {g.name}: {count}x {line[:200]}")
                log_changes += 1
    extra = sorted(p.name for p in args.candidate.glob("*.png") if not (args.golden / p.name).exists())
    for name in extra:
        print(f"EXTRA    {name}")
    print(f"summary: {len(gold_files)} images, {identical} identical, {changed} changed "
          f"({total_px} px), {missing} missing, {len(extra)} extra"
          + (f", {log_changes} new log lines" if args.logs else ""))
    return 0 if not (changed or missing or extra or log_changes) else 1


if __name__ == "__main__":
    sys.exit(main())
