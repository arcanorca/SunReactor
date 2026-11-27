#!/usr/bin/env python3
"""Generate authentic HiDPI (@1x, @2x, @3x) raster and SVG assets for the
flagship instrument skins: nds, wii, nothing, ps3, xbox360.

Uses SVG + rsvg-convert (Cairo) for sharp vector gradients, specular glass
highlights, bevels, and Gaussian blooms, plus NumPy/Pillow for brushed-metal
and pinstripe textures. Every asset is deterministic and written directly into
package/contents/ui/skins/<id>/assets/.
"""

from __future__ import annotations

import colorsys
import io
import math
import subprocess
from pathlib import Path
import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageEnhance, ImageFilter

HERE = Path(__file__).resolve().parent
SKINS = HERE.parent / "package" / "contents" / "ui" / "skins"
VENDOR = HERE / "vendor"
VENDOR_NOTHING = VENDOR / "nothing"
VENDOR_XBOX = VENDOR / "xbox360"


def render_svg_bytes(svg: str, w: int, h: int) -> Image.Image:
    """Render SVG text via rsvg-convert into an RGBA PIL Image."""
    res = subprocess.run(
        ["rsvg-convert", "-w", str(w), "-h", str(h)],
        input=svg.encode("utf-8"),
        capture_output=True,
        check=True,
    )
    return Image.open(io.BytesIO(res.stdout)).convert("RGBA")


def render_svg(svg: str, out_base: Path, w: int, h: int, scales: tuple[int, ...] = (1, 2, 3)) -> None:
    """Render SVG to out_base.png (1x), @2x.png, @3x.png."""
    out_base.parent.mkdir(parents=True, exist_ok=True)
    for s in scales:
        suffix = "" if s == 1 else f"@{s}x"
        target = out_base.with_name(f"{out_base.name}{suffix}.png")
        res = subprocess.run(
            ["rsvg-convert", "-w", str(w * s), "-h", str(h * s), "-o", str(target)],
            input=svg.encode("utf-8"),
            check=True,
        )
        assert res.returncode == 0


def save_scaled_png(img_3x: Image.Image, out_base: Path, w1: int, h1: int) -> None:
    """Save 3x PIL image downsampled with LANCZOS to 1x, 2x, 3x."""
    out_base.parent.mkdir(parents=True, exist_ok=True)
    for s in (1, 2, 3):
        suffix = "" if s == 1 else f"@{s}x"
        target = out_base.with_name(f"{out_base.name}{suffix}.png")
        if img_3x.size == (w1 * s, h1 * s):
            img_3x.save(target, optimize=True)
        else:
            img_3x.resize((w1 * s, h1 * s), Image.Resampling.LANCZOS).save(target, optimize=True)


# ==============================================================================
# 1. NINTENDO DS (DS System Menu + PictoChat Pixel Aesthetic)
# ==============================================================================

def save_pixel_png(img_1x: Image.Image, out_base: Path) -> None:
    """Save crisp nearest-neighbor @1x, @2x, @3x pixel-art PNGs."""
    out_base.parent.mkdir(parents=True, exist_ok=True)
    w, h = img_1x.size
    for s in (1, 2, 3):
        suffix = "" if s == 1 else f"@{s}x"
        target = out_base.with_name(f"{out_base.name}{suffix}.png")
        out_img = img_1x if s == 1 else img_1x.resize((w * s, h * s), Image.Resampling.NEAREST)
        out_img.save(target, optimize=True)


def build_nds() -> None:
    out = SKINS / "nds" / "assets"
    out.mkdir(parents=True, exist_ok=True)

    # 1a. Top & bottom screen bezels (9-slice: border=8; logical 80x64, opening 6..74 x 6..58)
    # Matches NdsStyle.bezel = 6 so both screens share the exact original dual-screen proportions.
    top_bezel_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="80" height="64" viewBox="0 0 80 64">
      <defs>
        <linearGradient id="shell" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#fcfdff"/>
          <stop offset="0.14" stop-color="#eff3f8"/>
          <stop offset="0.86" stop-color="#e0e6ef"/>
          <stop offset="1" stop-color="#c6d0dc"/>
        </linearGradient>
      </defs>
      <rect x="0.5" y="0.5" width="79" height="63" rx="4" fill="url(#shell)" stroke="#8f9bad" stroke-width="1"/>
      <rect x="4" y="4" width="72" height="56" fill="#2b313c"/>
      <rect x="5" y="5" width="70" height="54" fill="#151921"/>
      <rect x="6" y="6" width="68" height="52" fill="#0d1016"/>
    </svg>'''
    render_svg(top_bezel_svg, out / "ds_top_bezel", 80, 64)

    bot_bezel_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="80" height="64" viewBox="0 0 80 64">
      <defs>
        <linearGradient id="shellB" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#f9fbfd"/>
          <stop offset="0.86" stop-color="#e5eaf2"/>
          <stop offset="1" stop-color="#cdd5e1"/>
        </linearGradient>
      </defs>
      <rect x="0.5" y="0.5" width="79" height="63" rx="4" fill="url(#shellB)" stroke="#8f9bad" stroke-width="1"/>
      <rect x="4" y="4" width="72" height="56" fill="#2b313c"/>
      <rect x="5" y="5" width="70" height="54" fill="#171c25"/>
      <rect x="6" y="6" width="68" height="52" fill="#10141b"/>
    </svg>'''
    render_svg(bot_bezel_svg, out / "ds_bottom_bezel", 80, 64)

    # 1c. Plain Nintendo DS (NTR-001 / USG-001) Hinge barrel (9-slice horizontal: left=36, right=36, top=0, bottom=0; logical 160x18)
    hinge_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="160" height="18" viewBox="0 0 160 18">
      <defs>
        <linearGradient id="cyl" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#cfd6e0"/>
          <stop offset="0.24" stop-color="#ffffff"/>
          <stop offset="0.52" stop-color="#e8edf4"/>
          <stop offset="0.82" stop-color="#b4becb"/>
          <stop offset="1" stop-color="#8b95a5"/>
        </linearGradient>
        <linearGradient id="cylEnd" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#bfc7d3"/>
          <stop offset="0.28" stop-color="#f6f9fc"/>
          <stop offset="0.75" stop-color="#a8b3c2"/>
          <stop offset="1" stop-color="#7c8798"/>
        </linearGradient>
      </defs>
      <rect x="4" y="1" width="152" height="16" rx="6" fill="url(#cyl)" stroke="#8893a3" stroke-width="1"/>
      <rect x="4" y="1" width="24" height="16" rx="6" fill="url(#cylEnd)" stroke="#8893a3" stroke-width="1"/>
      <rect x="132" y="1" width="24" height="16" rx="6" fill="url(#cylEnd)" stroke="#8893a3" stroke-width="1"/>
      <line x1="28" y1="2" x2="28" y2="16" stroke="#8590a0" stroke-width="1"/>
      <line x1="29" y1="2" x2="29" y2="16" stroke="#ffffff" stroke-opacity="0.75" stroke-width="1"/>
      <line x1="132" y1="2" x2="132" y2="16" stroke="#8590a0" stroke-width="1"/>
      <line x1="133" y1="2" x2="133" y2="16" stroke="#ffffff" stroke-opacity="0.75" stroke-width="1"/>
      <rect x="78" y="7" width="4" height="4" rx="1" fill="#2b313c" stroke="#ffffff" stroke-opacity="0.65" stroke-width="0.6"/>
      <rect x="138" y="6" width="5" height="6" fill="#212630" stroke="#7d8796" stroke-width="0.8"/>
      <rect x="145" y="6" width="5" height="6" fill="#212630" stroke="#7d8796" stroke-width="0.8"/>
    </svg>'''
    render_svg(hinge_svg, out / "ds_hinge", 160, 18)

    # 1d. Authentic NTR Home Menu LCD Grid + 2px Scanline tiles (16x16 logical, nearest-neighbor)
    # From NDS-Clock-SVG-Kit nds_theme_catalog.json:
    #   wallpaper_base=#F9F9F9, wallpaper_scanline=#DEDEDE, grid_outer_line=#BEBEBE
    #   card_surface=#FFFFFF, card_surface_scanline=#EFEFEF, grid_inner_line=#D0D0D0
    sky_grid = Image.new("RGBA", (16, 16), (249, 249, 249, 255))
    px_sky = sky_grid.load()
    for y in range(16):
        for x in range(16):
            if y == 0 or x == 0:
                px_sky[x, y] = (190, 190, 190, 255)
            elif y % 2 == 1:
                px_sky[x, y] = (222, 222, 222, 255)
    save_pixel_png(sky_grid, out / "ds_sky_grid")

    picto_grid = Image.new("RGBA", (16, 16), (255, 255, 255, 255))
    px_picto = picto_grid.load()
    for y in range(16):
        for x in range(16):
            if y == 0 or x == 0:
                px_picto[x, y] = (208, 208, 208, 255)
            elif y % 2 == 1:
                px_picto[x, y] = (239, 239, 239, 255)
    save_pixel_png(picto_grid, out / "ds_picto_grid")

    # 1e. Pixelated NTR Home Menu 9-slice buttons/panels (32x32 logical, border=4)
    def make_ds_pixel_panel(
        name: str,
        outer: tuple[int, int, int, int],
        hi: tuple[int, int, int, int],
        shade: tuple[int, int, int, int],
        fill_a: tuple[int, int, int, int],
        fill_b: tuple[int, int, int, int],
        checker: bool = False,
    ) -> None:
        w, h = 32, 32
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        px = img.load()
        for y in range(h):
            for x in range(w):
                # 1px chamfered pixel corners
                if (x in (0, w - 1)) and (y in (0, h - 1)):
                    continue
                if x == 0 or y == 0 or x == w - 1 or y == h - 1 or (x in (1, w - 2) and y in (1, h - 2)):
                    px[x, y] = outer
                elif x == 1 or y == 1:
                    px[x, y] = hi
                elif x == w - 2 or y == h - 2:
                    px[x, y] = shade
                else:
                    if checker:
                        px[x, y] = fill_a if ((x + y) % 2 == 0) else fill_b
                    else:
                        px[x, y] = fill_a if (y % 2 == 0) else fill_b
        save_pixel_png(img, out / name)

    make_ds_pixel_panel(
        "ds_btn_panel",
        outer=(0, 0, 0, 255),
        hi=(255, 255, 255, 255),
        shade=(168, 168, 168, 255),
        fill_a=(255, 255, 255, 255),
        fill_b=(239, 239, 239, 255),
        checker=False,
    )
    # Translucent bevel/scanline overlay so NdsPanel can tint active buttons with theme.accent
    make_ds_pixel_panel(
        "ds_btn_active",
        outer=(0, 0, 0, 255),
        hi=(255, 255, 255, 115),
        shade=(0, 0, 0, 82),
        fill_a=(255, 255, 255, 18),
        fill_b=(0, 0, 0, 22),
        checker=True,
    )
    make_ds_pixel_panel(
        "ds_btn_quiet",
        outer=(120, 120, 120, 255),
        hi=(255, 255, 255, 255),
        shade=(190, 190, 190, 255),
        fill_a=(234, 234, 234, 255),
        fill_b=(222, 222, 222, 255),
        checker=True,
    )
    make_ds_pixel_panel(
        "ds_bubble",
        outer=(122, 100, 48, 255),
        hi=(255, 253, 240, 255),
        shade=(224, 198, 130, 255),
        fill_a=(255, 248, 218, 255),
        fill_b=(250, 239, 196, 255),
        checker=False,
    )

    # 1f. Pixel-art ascending signal bars (16x12: 4 bars indicating proximity to max brightness)
    #     and DS solar-synced battery icon (20x10: off + 0..3 bars)
    def draw_bitmap(rows: list[str], palette: dict[str, tuple[int, int, int, int]]) -> Image.Image:
        h = len(rows)
        w = len(rows[0])
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        px = img.load()
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch in palette:
                    px[x, y] = palette[ch]
        return img

    signal_rows = [
        "................",
        ".............44.",
        ".............4L.",
        ".........33..4L.",
        ".........3L..4L.",
        ".....22..3L..4L.",
        ".....2L..3L..4L.",
        ".11..2L..3L..4L.",
        ".1L..2L..3L..4L.",
        ".1L..2L..3L..4L.",
        ".BB..BB..BB..BB.",
        "................",
    ]
    lit_hi = (74, 224, 102, 255)
    lit_lo = (34, 184, 64, 255)
    lit_base = (24, 128, 46, 255)
    dim_hi = (176, 190, 208, 255)
    dim_lo = (146, 162, 184, 255)
    dim_base = (112, 128, 150, 255)
    off_col = (148, 156, 168, 255)
    off_base = (120, 128, 140, 255)

    save_pixel_png(
        draw_bitmap(
            signal_rows,
            {
                "1": off_col, "2": off_col, "3": off_col, "4": off_col,
                "L": off_col, "B": off_base,
            },
        ),
        out / "ds_signal_off",
    )

    for lvl in (0, 1, 2, 3, 4):
        sig_img = Image.new("RGBA", (16, 12), (0, 0, 0, 0))
        px = sig_img.load()
        bars = [(1, 7, 1), (5, 5, 2), (9, 3, 3), (13, 1, 4)]
        for bx, top_y, bar_idx in bars:
            active = lvl >= bar_idx
            c_hi = lit_hi if active else dim_hi
            c_lo = lit_lo if active else dim_lo
            c_base = lit_base if active else dim_base
            for y in range(top_y, 10):
                px[bx, y] = c_hi
                px[bx + 1, y] = c_lo
            px[bx, 10] = c_base
            px[bx + 1, 10] = c_base
        save_pixel_png(sig_img, out / f"ds_signal_{lvl}")

    bat_rows = [
        ".KKKKKKKKKKKKKKKK...",
        "KWWWWWWWWWWWWWWWWK..",
        "KW1111W2222W3333WKK.",
        "KW1111W2222W3333WKK.",
        "KW1111W2222W3333WKK.",
        "KW1111W2222W3333WKK.",
        "KW1111W2222W3333WKK.",
        "KW1111W2222W3333WKK.",
        "KWWWWWWWWWWWWWWWWK..",
        ".KKKKKKKKKKKKKKKK...",
    ]
    empty_slot = (218, 226, 236, 255)
    for lvl in (0, 1, 2, 3):
        fill_col = (56, 214, 78, 255) if lvl >= 2 else ((242, 172, 28, 255) if lvl == 1 else (228, 62, 62, 255))
        frame_col = (188, 38, 44, 255) if lvl == 0 else (26, 30, 36, 255)
        inner_col = (255, 232, 232, 255) if lvl == 0 else (244, 248, 252, 255)
        slot_empty = (242, 198, 198, 255) if lvl == 0 else empty_slot
        bat_pal = {
            "K": frame_col,
            "W": inner_col,
            "1": fill_col if lvl >= 1 else slot_empty,
            "2": fill_col if lvl >= 2 else slot_empty,
            "3": fill_col if lvl >= 3 else slot_empty,
        }
        save_pixel_png(draw_bitmap(bat_rows, bat_pal), out / f"ds_battery_{lvl}")

    save_pixel_png(
        draw_bitmap(
            bat_rows,
            {
                "K": (124, 132, 144, 255),
                "W": (236, 240, 246, 255),
                "1": empty_slot,
                "2": empty_slot,
                "3": empty_slot,
            },
        ),
        out / "ds_battery_off",
    )

    # 1g. Authentic NTR Top-Bar icons (from NDS-Clock-SVG-Kit top_status_bar_colon_on.svg)
    #     battery_ok_hex=#38B848, battery_low_hex=#D84848, white=#FFFFFF frame;
    #     signal bars carry a 1px dark frame so green bars stay legible on any IPL accent (including Green/Yellow).
    bar_bat_rows = [
        "WWWWWWWWWWWWWWWWWW..",
        "W................W..",
        "W.11112222333344.WW.",
        "W.11112222333344.WW.",
        "W.11112222333344.WW.",
        "W.11112222333344.WW.",
        "W.11112222333344.WW.",
        "W.11112222333344.WW.",
        "W................W..",
        "WWWWWWWWWWWWWWWWWW..",
    ]
    for lvl in (0, 1, 2, 3):
        fill_col = (56, 184, 72, 255) if lvl >= 2 else ((242, 172, 28, 255) if lvl == 1 else (216, 72, 72, 255))
        empty_bar_slot = (0, 0, 0, 64)
        bar_pal = {
            "W": (255, 255, 255, 255),
            "1": fill_col if lvl >= 1 else (216, 72, 72, 255),
            "2": fill_col if lvl >= 2 else empty_bar_slot,
            "3": fill_col if lvl >= 3 else empty_bar_slot,
            "4": fill_col if lvl >= 3 else empty_bar_slot,
        }
        save_pixel_png(draw_bitmap(bar_bat_rows, bar_pal), out / f"ds_bar_battery_{lvl}")

    save_pixel_png(
        draw_bitmap(
            bar_bat_rows,
            {
                "W": (255, 255, 255, 160),
                "1": (255, 255, 255, 45),
                "2": (255, 255, 255, 45),
                "3": (255, 255, 255, 45),
                "4": (255, 255, 255, 45),
            },
        ),
        out / "ds_bar_battery_off",
    )

    def make_bar_signal(lvl: int) -> Image.Image:
        sig_img = Image.new("RGBA", (16, 12), (0, 0, 0, 0))
        px = sig_img.load()
        bars = [(1, 7, 1), (5, 5, 2), (9, 3, 3), (13, 1, 4)]
        outline_col = (20, 24, 30, 255) if lvl >= 0 else (20, 24, 30, 170)
        for bx, top_y, bar_idx in bars:
            # 1px dark border around each ascending bar for contrast on green/yellow/blue IPL bars
            for y in range(top_y - 1, 11):
                for x in range(bx - 1, bx + 3):
                    px[x, y] = outline_col
            active = lvl >= bar_idx
            if lvl < 0:
                c_hi = (176, 186, 198, 220)
                c_lo = (140, 150, 164, 220)
            else:
                c_hi = (108, 240, 128, 255) if active else (235, 242, 248, 210)
                c_lo = (56, 184, 72, 255) if active else (190, 200, 212, 210)
            for y in range(top_y, 10):
                px[bx, y] = c_hi
                px[bx + 1, y] = c_lo
        return sig_img

    save_pixel_png(make_bar_signal(-1), out / "ds_bar_signal_off")
    for lvl in (0, 1, 2, 3, 4):
        save_pixel_png(make_bar_signal(lvl), out / f"ds_bar_signal_{lvl}")

    # 1h. Authentic NTR Home Calendar/Clock digits (8x10)
    # Extracted verbatim from NDS-Clock-SVG-Kit/output/nds_home_calendar/png/.
    ntr_digits = {
        "0": ["########", "########", "##....##", "##....##", "##....##", "##....##", "##....##", "##....##", "########", "########"],
        "1": ["..####..", "..####..", "....##..", "....##..", "....##..", "....##..", "....##..", "....##..", "..######", "..######"],
        "2": ["########", "########", "......##", "......##", "########", "########", "##......", "##......", "########", "########"],
        "3": ["########", "########", "......##", "......##", "########", "########", "......##", "......##", "########", "########"],
        "4": ["##....##", "##....##", "##....##", "##....##", "########", "########", "......##", "......##", "......##", "......##"],
        "5": ["########", "########", "##......", "##......", "########", "########", "......##", "......##", "########", "########"],
        "6": ["########", "########", "##......", "##......", "########", "########", "##....##", "##....##", "########", "########"],
        "7": ["########", "########", "......##", "......##", "....####", "....####", "....##..", "....##..", "....##..", "....##.."],
        "8": ["########", "########", "##....##", "##....##", "########", "########", "##....##", "##....##", "########", "########"],
        "9": ["########", "########", "##....##", "##....##", "########", "########", "......##", "......##", "########", "########"],
        "colon": ["........", "........", "..####..", "..####..", "........", "........", "..####..", "..####..", "........", "........"],
    }
    for key, rows in ntr_digits.items():
        save_pixel_png(draw_bitmap(rows, {"#": (255, 255, 255, 255)}), out / f"ds_cal_{key}")
        save_pixel_png(draw_bitmap(rows, {"#": (24, 30, 38, 255)}), out / f"ds_cal_{key}_dark")

    # 1i. Crisp NTR Home Menu square analog clock dial (72x72 logical, 388x388 canonical)
    clock_dial_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="388" height="388" viewBox="0 0 388 388">
      <rect x="4" y="4" width="380" height="380" fill="#FFFFFF"/>
      <g fill="#D0D0D0">
        <rect x="65" y="4" width="3" height="380"/>
        <rect x="4" y="65" width="380" height="3"/>
        <rect x="129" y="4" width="3" height="380"/>
        <rect x="4" y="129" width="380" height="3"/>
        <rect x="193" y="4" width="3" height="380"/>
        <rect x="4" y="193" width="380" height="3"/>
        <rect x="257" y="4" width="3" height="380"/>
        <rect x="4" y="257" width="380" height="3"/>
        <rect x="321" y="4" width="3" height="380"/>
        <rect x="4" y="321" width="380" height="3"/>
      </g>
      <g fill="#9A9EA6">
        <path d="M162 16 H178 V96 H162 Z"/>
        <path d="M182 16 H226 V64 H198 V80 H226 V96 H182 V48 H210 V32 H182 Z"/>
        <path d="M326 154 H374 V234 H326 V218 H358 V202 H326 V186 H358 V170 H326 Z"/>
        <path fill-rule="evenodd" d="M170 292 H218 V308 H186 V324 H218 V372 H170 Z M186 340 V356 H202 V340 Z"/>
        <path fill-rule="evenodd" d="M14 154 H62 V234 H14 V218 H46 V202 H14 Z M30 170 V186 H46 V170 Z"/>
      </g>
      <g fill="#6E747E">
        <rect x="98" y="34" width="18" height="18"/>
        <rect x="272" y="34" width="18" height="18"/>
        <rect x="34" y="98" width="18" height="18"/>
        <rect x="336" y="98" width="18" height="18"/>
        <rect x="34" y="272" width="18" height="18"/>
        <rect x="336" y="272" width="18" height="18"/>
        <rect x="98" y="336" width="18" height="18"/>
        <rect x="272" y="336" width="18" height="18"/>
      </g>
      <path fill="#000000" fill-rule="evenodd" d="M0 0 H388 V388 H0 Z M5 5 V383 H383 V5 Z"/>
    </svg>'''
    render_svg(clock_dial_svg, out / "ds_clock_dial", 72, 72)





# ==============================================================================
# 2. NINTENDO WII (Wii System Menu + Forecast Channel)
# ==============================================================================

def build_wii() -> None:
    out = SKINS / "wii" / "assets"
    w_out = out / "weather"
    out.mkdir(parents=True, exist_ok=True)
    w_out.mkdir(parents=True, exist_ok=True)

    # 2a. Wii Menu horizontal pinstripe tile (tileable 8x4 logical -> 8x4, 16x8, 24x12)
    for s in (1, 2, 3):
        w, h = 8 * s, 4 * s
        arr = np.zeros((h, w, 4), dtype=np.uint8)
        arr[: h // 2, :] = [242, 246, 250, 255]
        arr[h // 2 :, :] = [233, 239, 245, 255]
        Image.fromarray(arr, "RGBA").save(out / (f"wii_pinstripes{'@' + str(s) + 'x' if s > 1 else ''}.png"))

    # 2b. Wii Channel Tile (idle & lit) - 9-slice: border=15, logical 88x64
    for name, glow in [("wii_channel_idle", False), ("wii_channel_lit", True)]:
        rim_col = "#00a9e0" if glow else "#b8c4d1"
        rim_w = "2.0" if glow else "1.2"
        aura = '<rect x="1.5" y="1.5" width="85" height="60" rx="15" fill="none" stroke="#00a9e0" stroke-opacity="0.34" stroke-width="3.0"/>' if glow else ""
        tile_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="88" height="64" viewBox="0 0 88 64">
          <defs>
            <linearGradient id="face" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="#ffffff"/>
              <stop offset="0.55" stop-color="#f7fafd"/>
              <stop offset="1" stop-color="#e7eff6"/>
            </linearGradient>
            <linearGradient id="gloss" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="#ffffff" stop-opacity="0.95"/>
              <stop offset="1" stop-color="#ffffff" stop-opacity="0.0"/>
            </linearGradient>
            <linearGradient id="bevel" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="#c6d0db"/>
              <stop offset="1" stop-color="#edf2f7"/>
            </linearGradient>
          </defs>
          <rect x="2.5" y="4" width="83" height="58" rx="14" fill="#2b3a4a" fill-opacity="0.13"/>
          {aura}
          <rect x="2" y="2" width="84" height="59" rx="14" fill="url(#bevel)" stroke="{rim_col}" stroke-width="{rim_w}"/>
          <rect x="3.5" y="3.5" width="81" height="56" rx="12.5" fill="url(#face)" stroke="#ffffff" stroke-width="0.9"/>
          <rect x="5" y="4.5" width="78" height="20" rx="10" fill="url(#gloss)"/>
        </svg>'''
        render_svg(tile_svg, out / name, 88, 64)

    # 2c. Wii Glossy Capsule Pills (idle, outline/active, fill) - 9-slice border=14, logical 72x32
    for name, c0, c1, rim, rw, shadow in [
        ("wii_pill_idle", "#ffffff", "#e1e8f0", "#b3c0cd", "1.3", "0.14"),
        ("wii_pill_outline", "#ffffff", "#edf6fc", "#00a9e0", "2.2", "0.16"),
        ("wii_pill_fill", "#34c6f4", "#007bb8", "#0069a3", "1.5", "0.18"),
    ]:
        pill_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="72" height="32" viewBox="0 0 72 32">
          <defs>
            <linearGradient id="pbg" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{c0}"/>
              <stop offset="1" stop-color="{c1}"/>
            </linearGradient>
            <linearGradient id="pgl" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="#ffffff" stop-opacity="0.72"/>
              <stop offset="1" stop-color="#ffffff" stop-opacity="0.0"/>
            </linearGradient>
          </defs>
          <rect x="2" y="3" width="68" height="28" rx="14" fill="#1e2d3d" fill-opacity="{shadow}"/>
          <rect x="2" y="1.5" width="68" height="27" rx="13.5" fill="url(#pbg)" stroke="{rim}" stroke-width="{rw}"/>
          <rect x="4" y="3" width="64" height="12" rx="6" fill="url(#pgl)"/>
        </svg>'''
        render_svg(pill_svg, out / name, 72, 32)

    # 2d. Wii Circular Glossy Orb Button (36x36) & Curved Bottom Dock (180x40, 9-slice left=32 right=32 top=12 bottom=0)
    orb_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 36 36">
      <defs>
        <radialGradient id="orbFace" cx="0.5" cy="0.32" r="0.65">
          <stop offset="0" stop-color="#ffffff"/>
          <stop offset="0.6" stop-color="#ebf2f8"/>
          <stop offset="1" stop-color="#cbd7e4"/>
        </radialGradient>
        <linearGradient id="orbGloss" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#ffffff" stop-opacity="0.95"/>
          <stop offset="1" stop-color="#ffffff" stop-opacity="0.0"/>
        </linearGradient>
      </defs>
      <circle cx="18" cy="18.6" r="15.2" fill="#1c2a38" fill-opacity="0.20"/>
      <circle cx="18" cy="17.5" r="15.2" fill="url(#orbFace)" stroke="#8fa1b4" stroke-width="1.3"/>
      <circle cx="18" cy="17.5" r="13.4" fill="none" stroke="#00a9e0" stroke-opacity="0.52" stroke-width="1.1"/>
      <ellipse cx="18" cy="11.2" rx="9.5" ry="5.0" fill="url(#orbGloss)"/>
    </svg>'''
    render_svg(orb_svg, out / "wii_orb", 36, 36)

    dock_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="180" height="40" viewBox="0 0 180 40">
      <defs>
        <linearGradient id="dockFill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#e3e9f0"/>
          <stop offset="0.35" stop-color="#d2dbe5"/>
          <stop offset="1" stop-color="#becad6"/>
        </linearGradient>
      </defs>
      <path d="M0 10 Q28 10 42 3 L138 3 Q152 10 180 10 L180 40 L0 40 Z" fill="url(#dockFill)"/>
      <path d="M0 10 Q28 10 42 3 L138 3 Q152 10 180 10" fill="none" stroke="#9aa9b9" stroke-width="1.3"/>
      <path d="M0 11.2 Q28 11.2 42 4.2 L138 4.2 Q152 11.2 180 11.2" fill="none" stroke="#ffffff" stroke-opacity="0.88" stroke-width="1.0"/>
    </svg>'''
    render_svg(dock_svg, out / "wii_dock", 180, 40)

    # 2e. Wii Slider Thumb (24x24)
    thumb_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
      <defs>
        <radialGradient id="puck" cx="0.45" cy="0.3" r="0.68">
          <stop offset="0" stop-color="#ffffff"/>
          <stop offset="0.65" stop-color="#eef4fa"/>
          <stop offset="1" stop-color="#c3d1e0"/>
        </radialGradient>
      </defs>
      <circle cx="12" cy="13.2" r="9.5" fill="#1c2a38" fill-opacity="0.22"/>
      <circle cx="12" cy="12" r="9.5" fill="url(#puck)" stroke="#00a9e0" stroke-width="2"/>
      <ellipse cx="12" cy="8.5" rx="5.5" ry="2.8" fill="#ffffff" fill-opacity="0.85"/>
    </svg>'''
    render_svg(thumb_svg, out / "wii_slider_thumb", 24, 24)

    # 2f. Wii Forecast Channel 3D Glossy Weather Art (64x64 -> 1x/2x/3x)
    def wii_sun_svg(cx: float, cy: float, r: float, faint: bool = False) -> str:
        op = 'opacity="0.55"' if faint else ""
        rays = []
        for k in range(8):
            ang = math.radians(k * 45 - 90)
            ux, uy = math.cos(ang), math.sin(ang)
            nx, ny = -uy, ux
            rb, rt, hb, ht = r * 1.32, r * 1.92, r * 0.22, r * 0.06
            pts = (
                f"{cx + rb * ux + hb * nx:.2f},{cy + rb * uy + hb * ny:.2f} "
                f"{cx + rt * ux + ht * nx:.2f},{cy + rt * uy + ht * ny:.2f} "
                f"{cx + rt * ux - ht * nx:.2f},{cy + rt * uy - ht * ny:.2f} "
                f"{cx + rb * ux - hb * nx:.2f},{cy + rb * uy - hb * ny:.2f}"
            )
            rays.append(f'<polygon points="{pts}" fill="url(#wSunRay)" stroke="#ee9900" stroke-width="0.4"/>')
        return f'''<g {op}>
          <circle cx="{cx}" cy="{cy}" r="{r * 2.15}" fill="url(#wSunHalo)"/>
          {''.join(rays)}
          <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#wSunCore)" stroke="#ea8c00" stroke-width="0.8"/>
          <ellipse cx="{cx - r * 0.25}" cy="{cy - r * 0.35}" rx="{r * 0.48}" ry="{r * 0.26}" fill="#ffffff" fill-opacity="0.55" transform="rotate(-25 {cx - r * 0.25} {cy - r * 0.35})"/>
        </g>'''

    def wii_moon_svg(cx: float, cy: float, r: float) -> str:
        return f'''<g>
          <circle cx="{cx}" cy="{cy}" r="{r * 1.9}" fill="url(#wMoonHalo)"/>
          <mask id="mMask_{int(cx)}_{int(cy)}">
            <rect width="64" height="64" fill="black"/>
            <circle cx="{cx}" cy="{cy}" r="{r}" fill="white"/>
            <circle cx="{cx + r * 0.42}" cy="{cy - r * 0.35}" r="{r * 0.88}" fill="black"/>
          </mask>
          <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#wMoonGrad)" stroke="#5589bd" stroke-width="0.9" mask="url(#mMask_{int(cx)}_{int(cy)})"/>
          <circle cx="{cx - r * 0.4}" cy="{cy + r * 0.32}" r="{r * 0.16}" fill="#4d7fb2" fill-opacity="0.45" mask="url(#mMask_{int(cx)}_{int(cy)})"/>
        </g>'''

    def wii_star_svg(cx: float, cy: float, r: float) -> str:
        k = r * 0.22
        return f'''<g>
          <circle cx="{cx}" cy="{cy}" r="{r * 2.1}" fill="url(#wStarHalo)"/>
          <path d="M{cx},{cy - r} Q{cx + k},{cy - k} {cx + r},{cy} Q{cx + k},{cy + k} {cx},{cy + r} Q{cx - k},{cy + k} {cx - r},{cy} Q{cx - k},{cy - k} {cx},{cy - r} Z" fill="#fff2a6" stroke="#7fb3dc" stroke-width="0.6"/>
        </g>'''

    def wii_cloud_svg(cx: float, cy: float, s: float, storm: bool = False) -> str:
        grad = "wCloudStorm" if storm else "wCloudBody"
        edge = "#7d9ab5" if storm else "#a7c3dc"
        return f'''<g transform="translate({cx},{cy}) scale({s})">
          <ellipse cx="0" cy="11" rx="18" ry="3.2" fill="#2a6ea0" fill-opacity="0.18"/>
          <g stroke="{edge}" stroke-width="1.3" fill="url(#{grad})">
            <circle cx="-10" cy="3" r="8.5"/>
            <circle cx="0" cy="-3.5" r="10.5"/>
            <circle cx="11" cy="2" r="8.5"/>
            <rect x="-12" y="1" width="24" height="10.5" rx="5.2"/>
          </g>
          <g fill="url(#{grad})">
            <circle cx="-10" cy="3" r="8.5"/>
            <circle cx="0" cy="-3.5" r="10.5"/>
            <circle cx="11" cy="2" r="8.5"/>
            <rect x="-12" y="1" width="24" height="10.5" rx="5.2"/>
          </g>
          <path d="M-8,-6 A9.5,9.5 0 0,1 8,-7" fill="none" stroke="#ffffff" stroke-width="1.6" stroke-linecap="round" stroke-opacity="0.9"/>
        </g>'''

    def wii_drop_svg(x: float, top: float, w: float, h: float) -> str:
        r = w / 2
        yc = top + h - r
        return f'''<g>
          <path d="M{x},{top} C{x + 0.6 * r},{top + 0.42 * h} {x + r},{yc - 0.5 * r} {x + r},{yc} A{r},{r} 0 1,1 {x - r},{yc} C{x - r},{yc - 0.5 * r} {x - 0.6 * r},{top + 0.42 * h} {x},{top} Z"
                fill="url(#wDropGrad)" stroke="#066ca1" stroke-width="0.6"/>
          <line x1="{x - 0.42 * r}" y1="{yc - 0.1 * r}" x2="{x - 0.38 * r}" y2="{yc - 0.72 * r}" stroke="#ffffff" stroke-opacity="0.85" stroke-width="{0.36 * r:.2f}" stroke-linecap="round"/>
        </g>'''

    def wii_flake_svg(cx: float, cy: float, r: float) -> str:
        lines = []
        for k in range(6):
            a = math.radians(k * 60 - 90)
            ux, uy = math.cos(a), math.sin(a)
            lines.append(f'<line x1="{cx}" y1="{cy}" x2="{cx + r * ux:.2f}" y2="{cy + r * uy:.2f}"/>')
        s = ''.join(lines)
        return f'<g stroke="#7bc3ea" stroke-width="{r * 0.42:.2f}" stroke-linecap="round">{s}</g><g stroke="#ffffff" stroke-width="{r * 0.22:.2f}" stroke-linecap="round">{s}</g>'

    wii_defs = '''<defs>
      <radialGradient id="wSunHalo"><stop offset="0.35" stop-color="#ffe066" stop-opacity="0.55"/><stop offset="1" stop-color="#ffc82e" stop-opacity="0"/></radialGradient>
      <radialGradient id="wSunCore" cx="0.38" cy="0.35" r="0.65"><stop offset="0" stop-color="#fff9b8"/><stop offset="0.5" stop-color="#ffcf33"/><stop offset="1" stop-color="#ff8c1a"/></radialGradient>
      <linearGradient id="wSunRay" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffe566"/><stop offset="1" stop-color="#ffa31a"/></linearGradient>
      <radialGradient id="wMoonHalo"><stop offset="0.4" stop-color="#87c8f7" stop-opacity="0.45"/><stop offset="1" stop-color="#87c8f7" stop-opacity="0"/></radialGradient>
      <radialGradient id="wMoonGrad" cx="0.3" cy="0.3" r="0.8"><stop offset="0" stop-color="#eff7ff"/><stop offset="1" stop-color="#7eb1e0"/></radialGradient>
      <radialGradient id="wStarHalo"><stop offset="0" stop-color="#ffffff" stop-opacity="0.7"/><stop offset="1" stop-color="#ffffff" stop-opacity="0"/></radialGradient>
      <linearGradient id="wCloudBody" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="0.55" stop-color="#f2f8fd"/><stop offset="1" stop-color="#c4def4"/></linearGradient>
      <linearGradient id="wCloudStorm" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ebf2f8"/><stop offset="0.55" stop-color="#cddfeb"/><stop offset="1" stop-color="#8eb0cc"/></linearGradient>
      <linearGradient id="wDropGrad" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#33c4f5"/><stop offset="1" stop-color="#0075b5"/></linearGradient>
      <linearGradient id="wBoltGrad" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff078"/><stop offset="1" stop-color="#ffa600"/></linearGradient>
    </defs>'''

    wii_cases = {
        "clear_day": wii_sun_svg(32, 32, 13.5),
        "clear_night": wii_moon_svg(31, 33, 13.5) + wii_star_svg(14, 15, 4.2) + wii_star_svg(50, 17, 3.0) + wii_star_svg(48, 49, 3.6),
        "partly_cloudy_day": wii_sun_svg(22, 22, 10.5) + wii_cloud_svg(34, 37, 0.96),
        "partly_cloudy_night": wii_moon_svg(21, 22, 10.5) + wii_star_svg(49, 13, 3.4) + wii_cloud_svg(34, 37, 0.96),
        "cloudy": wii_cloud_svg(39, 25, 0.72) + wii_cloud_svg(29, 38, 1.0),
        "drizzle": wii_cloud_svg(32, 26, 0.96) + wii_drop_svg(20, 41, 4.8, 10) + wii_drop_svg(32, 46, 4.8, 10) + wii_drop_svg(44, 41, 4.8, 10),
        "rain": wii_cloud_svg(32, 24, 0.98) + wii_drop_svg(18, 40, 5.4, 12.5) + wii_drop_svg(32, 46, 5.4, 12.5) + wii_drop_svg(46, 40, 5.4, 12.5),
        "heavy_rain": wii_cloud_svg(32, 23, 1.0, storm=True) + wii_drop_svg(14, 40, 5.0, 12) + wii_drop_svg(26, 46, 5.0, 12) + wii_drop_svg(38, 40, 5.0, 12) + wii_drop_svg(50, 46, 5.0, 12),
        "thunderstorm": wii_cloud_svg(32, 22, 1.0, storm=True) + '<polygon points="36,29 46,29 37,41 44,41 29,59 33,46 26,46" fill="url(#wBoltGrad)" stroke="#d98700" stroke-width="0.9" stroke-linejoin="round"/>' + wii_drop_svg(15, 43, 4.5, 10) + wii_drop_svg(50, 42, 4.5, 10),
        "snow": wii_cloud_svg(32, 24, 0.96) + wii_flake_svg(18, 48, 5.5) + wii_flake_svg(33, 54, 4.8) + wii_flake_svg(47, 47, 5.2),
        "mist": '<g stroke-linecap="round"><g stroke="#96b9d4" stroke-width="5.6"><line x1="8" y1="24" x2="56" y2="24"/><line x1="16" y1="34" x2="48" y2="34"/><line x1="7" y1="44" x2="44" y2="44"/><line x1="19" y1="53" x2="56" y2="53"/></g><g stroke="#ffffff" stroke-width="3.6"><line x1="8" y1="24" x2="56" y2="24"/><line x1="16" y1="34" x2="48" y2="34"/><line x1="7" y1="44" x2="44" y2="44"/><line x1="19" y1="53" x2="56" y2="53"/></g></g>',
        "atmospheric": wii_sun_svg(32, 25, 11.5, faint=True) + '<g stroke-linecap="round"><g stroke="#96b9d4" stroke-width="6"><line x1="10" y1="42" x2="54" y2="42"/><line x1="18" y1="52" x2="46" y2="52"/></g><g stroke="#ffffff" stroke-width="4"><line x1="10" y1="42" x2="54" y2="42"/><line x1="18" y1="52" x2="46" y2="52"/></g></g>',
        "fog": wii_cloud_svg(32, 22, 0.92) + '<g stroke-linecap="round"><g stroke="#96b9d4" stroke-width="6.2"><line x1="7" y1="38" x2="51" y2="38"/><line x1="15" y1="47" x2="57" y2="47"/><line x1="9" y1="55" x2="46" y2="55"/></g><g stroke="#ffffff" stroke-width="4.2"><line x1="7" y1="38" x2="51" y2="38"/><line x1="15" y1="47" x2="57" y2="47"/><line x1="9" y1="55" x2="46" y2="55"/></g></g>',
        "unknown": wii_cloud_svg(32, 28, 1.0) + '<text x="32" y="34" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="18" fill="#0078b5">?</text>',
    }
    for cond, body in wii_cases.items():
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">{wii_defs}{body}</svg>'
        render_svg(svg, w_out / cond, 64, 64)


# ==============================================================================
# 3. NOTHING OS (Glyph Interface + Halftone Dot Weather Art)
# ==============================================================================

def build_nothing() -> None:
    out = SKINS / "nothing" / "assets"
    w_out = out / "weather"
    out.mkdir(parents=True, exist_ok=True)
    w_out.mkdir(parents=True, exist_ok=True)

    # 3a. Nothing Phone Glyph Interface Segmented Outer Bezel (120x120)
    # Razor-sharp unblurred Glyph LED arcs framing DotRing with matte OLED dial recess.
    glyph_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120" viewBox="0 0 120 120">
      <circle cx="60" cy="60" r="51.5" fill="#101012" stroke="#ffffff" stroke-opacity="0.08" stroke-width="0.9"/>
      <path d="M 32 13 A 55 55 0 0 1 92 16" fill="none" stroke="#ffffff" stroke-opacity="0.28" stroke-width="1.8" stroke-linecap="round"/>
      <path d="M 103 26 A 55 55 0 0 1 113 72" fill="none" stroke="#ffffff" stroke-opacity="0.22" stroke-width="1.8" stroke-linecap="round"/>
      <path d="M 106 89 A 55 55 0 0 1 40 111" fill="none" stroke="#ffffff" stroke-opacity="0.24" stroke-width="1.8" stroke-linecap="round"/>
      <path d="M 22 100 A 55 55 0 0 1 14 29" fill="none" stroke="#ffffff" stroke-opacity="0.20" stroke-width="1.8" stroke-linecap="round"/>
      <circle cx="60" cy="4.8" r="1.05" fill="#ffffff" fill-opacity="0.40"/>
      <circle cx="115.2" cy="60" r="1.05" fill="#ffffff" fill-opacity="0.22"/>
      <circle cx="60" cy="115.2" r="1.05" fill="#ffffff" fill-opacity="0.22"/>
      <circle cx="4.8" cy="60" r="1.05" fill="#ffffff" fill-opacity="0.22"/>
    </svg>'''
    render_svg(glyph_svg, out / "nothing_glyph_ring", 120, 120)

    # 3a2. Nothing OS Circular Dot-Matrix Refresh Button (40x40)
    dots_ring = []
    for k in range(24):
        a = k * (2 * math.pi / 24) - math.pi / 2
        px = 20 + 16.0 * math.cos(a)
        py = 20 + 16.0 * math.sin(a)
        dots_ring.append(f'<circle cx="{px:.2f}" cy="{py:.2f}" r="0.82" fill="#ffffff" fill-opacity="0.22"/>')
    nothing_btn_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">
      <circle cx="20" cy="20" r="18.6" fill="#18181a" stroke="#ffffff" stroke-opacity="0.14" stroke-width="0.9"/>
      {''.join(dots_ring)}
    </svg>'''
    render_svg(nothing_btn_svg, out / "nothing_orb_btn", 40, 40)

    # 3a3. Nothing OS Solar Horizon Icons (36x20 -> 1x/2x/3x), modeled on Nothing Weather sunrise/sunset SVGs
    for name, arrow_up in [("nothing_sunrise", True), ("nothing_sunset", False)]:
        arrow = (
            '<line x1="18" y1="12.5" x2="18" y2="7.5" stroke="#ffffff" stroke-width="1.2" stroke-linecap="round"/>'
            '<polyline points="15.5,9.6 18,7.1 20.5,9.6" fill="none" stroke="#ffffff" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>'
            if arrow_up
            else '<line x1="18" y1="7.3" x2="18" y2="12.3" stroke="#ffffff" stroke-width="1.2" stroke-linecap="round"/>'
            '<polyline points="15.5,10.2 18,12.7 20.5,10.2" fill="none" stroke="#ffffff" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>'
        )
        solar_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="36" height="20" viewBox="0 0 36 20">
          <path d="M 5.5 14.5 A 12.5 11.2 0 0 1 30.5 14.5" fill="none" stroke="#ffffff" stroke-opacity="0.26" stroke-width="1.0"/>
          <path d="M 8.8 14.5 A 9.2 8.2 0 0 1 27.2 14.5" fill="none" stroke="#ffffff" stroke-opacity="0.88" stroke-width="1.15"/>
          <circle cx="18" cy="3.3" r="1.3" fill="#d71921"/>
          {arrow}
          <line x1="2.5" y1="14.5" x2="33.5" y2="14.5" stroke="#ffffff" stroke-opacity="0.72" stroke-width="1.1" stroke-linecap="round"/>
          <line x1="7.5" y1="17.0" x2="28.5" y2="17.0" stroke="#ffffff" stroke-opacity="0.34" stroke-width="1.0" stroke-linecap="round"/>
          <line x1="13.0" y1="19.1" x2="23.0" y2="19.1" stroke="#ffffff" stroke-opacity="0.18" stroke-width="0.9" stroke-linecap="round"/>
        </svg>'''
        render_svg(solar_svg, out / name, 36, 20)

    # 3b. Official Nothing OS Weather Icons (64x64 -> 1x/2x/3x):
    # Direct upstream SVGs from jaxparrow07/nothing-kde-widgets (GPL-3.0) on the 389x389 grid
    # (center 194.5, pitch 34.2, dot r=17.1) + exact-grid companions for all 14 conditions.
    vendor_map = {
        "clear_day": "sunny.svg",
        "cloudy": "cloudy.svg",
        "partly_cloudy_day": "partly_cloudy_day.svg",
        "partly_cloudy_night": "partly_cloudy_night.svg",
        "rain": "rain_or_mist.svg",
        "snow": "snow_fall.svg",
        "thunderstorm": "thunder.svg",
    }
    for cond, filename in vendor_map.items():
        raw_svg = (VENDOR_NOTHING / filename).read_text(encoding="utf-8")
        # Normalize Android dip units so rsvg-convert scales the 389x389 viewBox cleanly to 64/128/192px
        norm_svg = raw_svg.replace('width="73.0dip"', 'width="389.0px"').replace('height="73.0dip"', 'height="389.0px"')
        norm_svg = norm_svg.replace("dip", "px")
        render_svg(norm_svg, w_out / cond, 64, 64)

    def nothing_grid_svg(dots: list[tuple[int, int, str]]) -> str:
        """Render dots on the exact official Nothing 389x389 grid (center 194.5, pitch 34.2, r 16.9)."""
        circles = []
        for gx, gy, col in dots:
            cx = 194.5 + gx * 34.2
            cy = 194.5 + gy * 34.2
            circles.append(f'<circle cx="{cx:.2f}" cy="{cy:.2f}" r="16.9" fill="{col}"/>')
        return f'<svg xmlns="http://www.w3.org/2000/svg" width="389" height="389" viewBox="0 0 389 389">{"".join(circles)}</svg>'

    cloud_top = [
        (1, -4, "#ffffff"), (2, -4, "#ffffff"), (3, -4, "#ffffff"),
        (-3, -3, "#ffffff"), (-2, -3, "#ffffff"), (0, -3, "#ffffff"), (1, -3, "#ffffff"),
        (2, -3, "#ffffff"), (3, -3, "#ffffff"), (4, -3, "#ffffff"),
        (-4, -2, "#ffffff"), (-3, -2, "#ffffff"), (-2, -2, "#ffffff"), (-1, -2, "#ffffff"),
        (0, -2, "#ffffff"), (1, -2, "#ffffff"), (2, -2, "#ffffff"), (3, -2, "#ffffff"),
        (4, -2, "#ffffff"), (5, -2, "#ffffff"),
        (-5, -1, "#ffffff"), (-4, -1, "#ffffff"), (-3, -1, "#ffffff"), (-2, -1, "#ffffff"),
        (-1, -1, "#ffffff"), (0, -1, "#ffffff"), (1, -1, "#ffffff"), (2, -1, "#ffffff"),
        (3, -1, "#ffffff"), (4, -1, "#ffffff"), (5, -1, "#ffffff"),
        (-5, 0, "#ffffff"), (-4, 0, "#ffffff"), (-3, 0, "#ffffff"), (-2, 0, "#ffffff"),
        (-1, 0, "#ffffff"), (0, 0, "#ffffff"), (1, 0, "#ffffff"), (2, 0, "#ffffff"),
        (3, 0, "#ffffff"), (4, 0, "#ffffff"), (5, 0, "#ffffff"),
        (-4, 1, "#ffffff"), (-3, 1, "#ffffff"), (-2, 1, "#ffffff"), (-1, 1, "#ffffff"),
        (0, 1, "#ffffff"), (1, 1, "#ffffff"), (2, 1, "#ffffff"), (3, 1, "#ffffff"),
        (4, 1, "#ffffff"),
    ]
    clear_night_dots = [
        (-2, -4, "#ffffff"), (-1, -4, "#ffffff"), (0, -4, "#ffffff"),
        (-3, -3, "#ffffff"), (-2, -3, "#ffffff"), (-1, -3, "#ffffff"),
        (-4, -2, "#ffffff"), (-3, -2, "#ffffff"), (-2, -2, "#ffffff"),
        (-4, -1, "#ffffff"), (-3, -1, "#ffffff"), (-2, -1, "#ffffff"),
        (-4, 0, "#ffffff"), (-3, 0, "#ffffff"), (-2, 0, "#ffffff"),
        (-3, 1, "#ffffff"), (-2, 1, "#ffffff"), (-1, 1, "#ffffff"),
        (-2, 2, "#ffffff"), (-1, 2, "#ffffff"), (0, 2, "#ffffff"), (1, 2, "#ffffff"), (2, 2, "#ffffff"),
        (-1, 3, "#ffffff"), (0, 3, "#ffffff"), (1, 3, "#ffffff"),
        # 4-dot star
        (3, -4, "#ffffff"), (2, -3, "#ffffff"), (4, -3, "#ffffff"), (3, -2, "#ffffff"),
    ]
    drizzle_dots = cloud_top + [
        (-2, 3, "#ffffff"), (1, 3, "#ffffff"), (4, 3, "#ffffff"),
        (-3, 4, "#ffffff"), (0, 4, "#ffffff"), (3, 4, "#ffffff"),
    ]
    heavy_rain_dots = cloud_top + [
        (-3, 3, "#ffffff"), (-1, 3, "#ffffff"), (1, 3, "#ffffff"), (3, 3, "#ffffff"),
        (-4, 4, "#ffffff"), (-2, 4, "#ffffff"), (0, 4, "#ffffff"), (2, 4, "#ffffff"),
        (4, 4, "#ffffff"),
    ]
    fog_dots = [
        (x, y, "#ffffff")
        for y, xs in [(-3, (-4, -2, 0, 2, 4)), (-1, (-3, -1, 1, 3)), (1, (-4, -2, 0, 2, 4)), (3, (-3, -1, 1, 3))]
        for x in xs
    ]
    mist_dots = [
        (x, y, "#ffffff")
        for y, xs in [(-2, (-4, -2, 0, 2, 4)), (0, (-3, -1, 1, 3)), (2, (-4, -2, 0, 2, 4))]
        for x in xs
    ]
    # Hazy sunshine: official Nothing hazy sun + staggered mist dots
    atmo_dots = [
        (-2, -4, "#ffffff"),
        (-4, -3, "#ffffff"), (-2, -3, "#ffffff"), (0, -3, "#ffffff"),
        (-3, -2, "#ffffff"), (-2, -2, "#ffffff"), (-1, -2, "#ffffff"),
        (-4, -1, "#ffffff"), (-3, -1, "#ffffff"), (-2, -1, "#ffffff"), (-1, -1, "#ffffff"), (0, -1, "#ffffff"),
        (-3, 0, "#ffffff"), (-2, 0, "#ffffff"), (-1, 0, "#ffffff"),
        (1, 1, "#ffffff"), (3, 1, "#ffffff"),
        (-3, 2, "#ffffff"), (-1, 2, "#ffffff"), (2, 2, "#ffffff"), (4, 2, "#ffffff"),
        (-2, 3, "#ffffff"), (0, 3, "#ffffff"), (3, 3, "#ffffff"),
    ]
    # Hollow dot diamond (error.svg geometry) with signature red signal core
    unknown_dots = [
        (-1, -4, "#ffffff"), (0, -4, "#ffffff"), (1, -4, "#ffffff"),
        (-2, -3, "#ffffff"), (-1, -3, "#ffffff"), (1, -3, "#ffffff"), (2, -3, "#ffffff"),
        (-3, -2, "#ffffff"), (-2, -2, "#ffffff"), (2, -2, "#ffffff"), (3, -2, "#ffffff"),
        (-4, -1, "#ffffff"), (-3, -1, "#ffffff"), (3, -1, "#ffffff"), (4, -1, "#ffffff"),
        (-4, 0, "#ffffff"), (-3, 0, "#ffffff"), (0, 0, "#d71921"), (3, 0, "#ffffff"), (4, 0, "#ffffff"),
        (-4, 1, "#ffffff"), (-3, 1, "#ffffff"), (3, 1, "#ffffff"), (4, 1, "#ffffff"),
        (-3, 2, "#ffffff"), (-2, 2, "#ffffff"), (2, 2, "#ffffff"), (3, 2, "#ffffff"),
        (-2, 3, "#ffffff"), (-1, 3, "#ffffff"), (1, 3, "#ffffff"), (2, 3, "#ffffff"),
        (-1, 4, "#ffffff"), (0, 4, "#ffffff"), (1, 4, "#ffffff"),
    ]

    extra_nothing = {
        "clear_night": clear_night_dots,
        "drizzle": drizzle_dots,
        "heavy_rain": heavy_rain_dots,
        "fog": fog_dots,
        "mist": mist_dots,
        "atmospheric": atmo_dots,
        "unknown": unknown_dots,
    }
    for cond, dots in extra_nothing.items():
        render_svg(nothing_grid_svg(dots), w_out / cond, 64, 64)


# ==============================================================================
# 4. PLAYSTATION 3 (XMB Etched-Crystal Icons + Spotlight + Info Bar)
# ==============================================================================

def build_ps3() -> None:
    out = SKINS / "ps3" / "assets"
    w_out = out / "weather"
    out.mkdir(parents=True, exist_ok=True)
    w_out.mkdir(parents=True, exist_ok=True)

    ps3_defs = '''<defs>
      <filter id="xmbGlow" x="-30%" y="-30%" width="160%" height="160%">
        <feGaussianBlur stdDeviation="2.4" result="blur"/>
        <feColorMatrix in="blur" type="matrix" values="0 0 0 0 0.78  0 0 0 0 0.9  0 0 0 0 1  0 0 0 0.65 0" result="tint"/>
        <feMerge><feMergeNode in="tint"/><feMergeNode in="SourceGraphic"/></feMerge>
      </filter>
      <linearGradient id="crystal" gradientUnits="userSpaceOnUse" x1="8" y1="6" x2="40" y2="58">
        <stop offset="0" stop-color="#ffffff" stop-opacity="0.98"/>
        <stop offset="0.48" stop-color="#dcebff" stop-opacity="0.88"/>
        <stop offset="1" stop-color="#9cc6fa" stop-opacity="0.72"/>
      </linearGradient>
      <linearGradient id="crystalSoft" gradientUnits="userSpaceOnUse" x1="8" y1="6" x2="8" y2="58">
        <stop offset="0" stop-color="#ffffff" stop-opacity="0.75"/>
        <stop offset="1" stop-color="#8ab8f0" stop-opacity="0.32"/>
      </linearGradient>
    </defs>'''

    # 4a. XMB Category Icons: Brightness (Display/Sun crystal) & Weather (Network/Cloud crystal)
    cat_bright = f'''<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48">
      {ps3_defs}
      <g filter="url(#xmbGlow)">
        <circle cx="24" cy="24" r="9.5" fill="url(#crystal)" stroke="#ffffff" stroke-width="0.9"/>
        <ellipse cx="22" cy="20" rx="5.5" ry="2.8" fill="#ffffff" fill-opacity="0.65"/>
        {''.join(f'<line x1="{24 + 13.5 * math.cos(a):.2f}" y1="{24 + 13.5 * math.sin(a):.2f}" x2="{24 + 18.5 * math.cos(a):.2f}" y2="{24 + 18.5 * math.sin(a):.2f}" stroke="url(#crystal)" stroke-width="2.6" stroke-linecap="round"/>' for a in [k * math.pi / 4 for k in range(8)])}
      </g>
    </svg>'''
    render_svg(cat_bright, out / "xmb_cat_brightness", 48, 48)

    cat_weather = f'''<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48">
      {ps3_defs}
      <g filter="url(#xmbGlow)">
        <circle cx="17" cy="28" r="7.5" fill="url(#crystal)"/>
        <circle cx="25.5" cy="21" r="9.8" fill="url(#crystal)"/>
        <circle cx="33" cy="27.5" r="7.2" fill="url(#crystal)"/>
        <rect x="16" y="25" width="17" height="10.5" rx="5" fill="url(#crystal)"/>
        <path d="M18,16 A8.5,8.5 0 0,1 32,16" fill="none" stroke="#ffffff" stroke-width="1.3" stroke-opacity="0.85" stroke-linecap="round"/>
      </g>
    </svg>'''
    render_svg(cat_weather, out / "xmb_cat_weather", 48, 48)

    # 4b. XMB Horizontal Spotlight Beam (stretchable 256x48)
    for s in (1, 2):
        w, h = 256 * s, 48 * s
        x = np.linspace(-1.0, 1.0, w)[None, :]
        y = np.linspace(-1.0, 1.0, h)[:, None]
        beam = np.exp(-3.2 * (x ** 2)) * np.exp(-3.8 * (y ** 2))
        rgba = np.zeros((h, w, 4), dtype=np.uint8)
        rgba[..., 0] = 215
        rgba[..., 1] = 235
        rgba[..., 2] = 255
        rgba[..., 3] = np.clip(beam * 58, 0, 255).astype(np.uint8)
        Image.fromarray(rgba, "RGBA").save(out / (f"ps3_spotlight{'@' + str(s) + 'x' if s > 1 else ''}.png"))

    # 4c. XMB Info Pill (9-slice border=12, logical 96x28)
    infobar_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="96" height="28" viewBox="0 0 96 28">
      <defs>
        <linearGradient id="ib" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#ffffff" stop-opacity="0.16"/>
          <stop offset="1" stop-color="#6090d8" stop-opacity="0.06"/>
        </linearGradient>
      </defs>
      <rect x="0.8" y="0.8" width="94.4" height="26.4" rx="13.2" fill="url(#ib)" stroke="#ffffff" stroke-opacity="0.28" stroke-width="1"/>
    </svg>'''
    render_svg(infobar_svg, out / "ps3_infobar", 96, 28)

    # 4d. XMB Slider Crystal Orb Thumb (24x24) & Etched-Glass Refresh Button (40x40)
    orb_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
      {ps3_defs}
      <circle cx="12" cy="12" r="10.5" fill="#9fd4ff" fill-opacity="0.24"/>
      <circle cx="12" cy="12" r="6.8" fill="url(#crystal)" stroke="#ffffff" stroke-width="0.9"/>
      <ellipse cx="10.5" cy="9.5" rx="3.2" ry="1.7" fill="#ffffff" fill-opacity="0.85" transform="rotate(-20 10.5 9.5)"/>
    </svg>'''
    render_svg(orb_svg, out / "ps3_slider_orb", 24, 24)

    ps3_btn_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">
      <defs>
        <radialGradient id="ps3OrbGlow" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0.45" stop-color="#9fd4ff" stop-opacity="0.28"/>
          <stop offset="1" stop-color="#9fd4ff" stop-opacity="0"/>
        </radialGradient>
        <linearGradient id="ps3Glass" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#ffffff" stop-opacity="0.26"/>
          <stop offset="0.5" stop-color="#9cc6fa" stop-opacity="0.10"/>
          <stop offset="1" stop-color="#5e8cd8" stop-opacity="0.18"/>
        </linearGradient>
      </defs>
      <circle cx="20" cy="20" r="19" fill="url(#ps3OrbGlow)"/>
      <circle cx="20" cy="20" r="15.5" fill="url(#ps3Glass)" stroke="#ffffff" stroke-opacity="0.48" stroke-width="1.1"/>
      <ellipse cx="20" cy="12" rx="9" ry="4" fill="#ffffff" fill-opacity="0.22"/>
    </svg>'''
    render_svg(ps3_btn_svg, out / "ps3_orb_btn", 40, 40)

    # 4e. PS3 XMB Etched-Crystal Weather Icons (64x64 -> 1x/2x/3x)
    def p_sun(cx: float, cy: float, r: float, faint: bool = False) -> str:
        op = 'opacity="0.5"' if faint else ""
        rays = "".join(
            f'<line x1="{cx + r * 1.32 * math.cos(a):.2f}" y1="{cy + r * 1.32 * math.sin(a):.2f}" '
            f'x2="{cx + r * 1.85 * math.cos(a):.2f}" y2="{cy + r * 1.85 * math.sin(a):.2f}" '
            f'stroke="url(#crystal)" stroke-width="2.4" stroke-linecap="round"/>'
            for a in [k * math.pi / 4 for k in range(8)]
        )
        return f'<g {op}><circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#crystal)" stroke="#ffffff" stroke-width="0.8"/>{rays}</g>'

    def p_moon(cx: float, cy: float, r: float) -> str:
        return f'''<mask id="pm_{int(cx)}">
          <rect width="64" height="64" fill="black"/>
          <circle cx="{cx}" cy="{cy}" r="{r}" fill="white"/>
          <circle cx="{cx + r * 0.45}" cy="{cy - r * 0.35}" r="{r * 0.85}" fill="black"/>
        </mask>
        <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#crystal)" mask="url(#pm_{int(cx)})"/>'''

    def p_cloud(cx: float, cy: float, s: float) -> str:
        return f'''<g transform="translate({cx},{cy}) scale({s})">
          <circle cx="-9.5" cy="3" r="8" fill="url(#crystal)"/>
          <circle cx="0.5" cy="-3.5" r="10.2" fill="url(#crystal)"/>
          <circle cx="10.5" cy="2.5" r="7.8" fill="url(#crystal)"/>
          <rect x="-11" y="1" width="22" height="10" rx="5" fill="url(#crystal)"/>
          <path d="M-7,-6 A9,9 0 0,1 8,-7" fill="none" stroke="#ffffff" stroke-width="1.3" stroke-opacity="0.8" stroke-linecap="round"/>
        </g>'''

    def p_drop(x: float, y: float) -> str:
        return f'<line x1="{x}" y1="{y}" x2="{x - 2.5}" y2="{y + 8}" stroke="url(#crystal)" stroke-width="2.4" stroke-linecap="round"/>'

    def p_flake(cx: float, cy: float, r: float) -> str:
        lines = "".join(
            f'<line x1="{cx}" y1="{cy}" x2="{cx + r * math.cos(a):.2f}" y2="{cy + r * math.sin(a):.2f}"/>'
            for a in [k * math.pi / 3 - math.pi / 2 for k in range(6)]
        )
        return f'<g stroke="url(#crystal)" stroke-width="1.8" stroke-linecap="round">{lines}</g>'

    ps3_cases = {
        "clear_day": p_sun(32, 32, 12.5),
        "clear_night": p_moon(30, 33, 13) + '<circle cx="48" cy="17" r="2" fill="#ffffff"/><circle cx="16" cy="18" r="1.4" fill="#dcebff"/><circle cx="47" cy="47" r="1.6" fill="#dcebff"/>',
        "partly_cloudy_day": p_sun(22, 22, 9.5) + p_cloud(34, 38, 0.94),
        "partly_cloudy_night": p_moon(21, 22, 9.5) + '<circle cx="48" cy="14" r="1.8" fill="#ffffff"/>' + p_cloud(34, 38, 0.94),
        "cloudy": p_cloud(38, 25, 0.72) + p_cloud(29, 38, 0.98),
        "drizzle": p_cloud(32, 25, 0.95) + p_drop(22, 42) + p_drop(33, 45) + p_drop(44, 42),
        "rain": p_cloud(32, 23, 0.96) + p_drop(20, 41) + p_drop(30, 44) + p_drop(40, 41) + p_drop(48, 44),
        "heavy_rain": p_cloud(32, 22, 0.98) + p_drop(16, 40) + p_drop(25, 43) + p_drop(34, 40) + p_drop(43, 43) + p_drop(51, 40),
        "thunderstorm": p_cloud(32, 22, 0.98) + '<polygon points="35,34 44,34 36,44 43,44 29,59 33,47 27,47" fill="#ffffff"/>' + p_drop(18, 43) + p_drop(50, 43),
        "snow": p_cloud(32, 23, 0.95) + p_flake(19, 47, 5) + p_flake(33, 52, 4.5) + p_flake(46, 47, 4.8),
        "mist": '<g stroke="url(#crystal)" stroke-width="3.6" stroke-linecap="round"><line x1="10" y1="24" x2="54" y2="24"/><line x1="16" y1="33" x2="48" y2="33"/><line x1="9" y1="42" x2="44" y2="42"/><line x1="20" y1="51" x2="54" y2="51"/></g>',
        "atmospheric": p_sun(32, 24, 11, faint=True) + '<g stroke="url(#crystal)" stroke-width="3.8" stroke-linecap="round"><line x1="11" y1="42" x2="53" y2="42"/><line x1="18" y1="51" x2="46" y2="51"/></g>',
        "fog": p_cloud(32, 22, 0.90) + '<g stroke="url(#crystal)" stroke-width="4" stroke-linecap="round"><line x1="9" y1="40" x2="50" y2="40"/><line x1="16" y1="49" x2="55" y2="49"/></g>',
        "unknown": p_cloud(32, 27, 0.95) + '<text x="32" y="54" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="16" fill="#ffffff">?</text>',
    }
    for cond, body in ps3_cases.items():
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">{ps3_defs}<g filter="url(#xmbGlow)">{body}</g></svg>'
        render_svg(svg, w_out / cond, 64, 64)


# ==============================================================================
# 5. XBOX 360 (2005 Launch "Blades" Dashboard + Real Xenon / Guide Assets)
# ==============================================================================

def _tile_texture(tex: Image.Image, w: int, h: int) -> Image.Image:
    """Mirror-tile an RGBA texture cleanly to (w, h) so there are zero tile seams."""
    out = Image.new("RGBA", (w, h))
    tw, th = tex.size
    tex_x = tex.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    tex_y = tex.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
    tex_xy = tex_x.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
    for row, y in enumerate(range(0, h, th)):
        for col, x in enumerate(range(0, w, tw)):
            tile = (tex if row % 2 == 0 else tex_y) if col % 2 == 0 else (tex_x if row % 2 == 0 else tex_xy)
            out.paste(tile, (x, y))
    return out


def _hue_shift_rgba(img: Image.Image, target_deg: float, sat_scale: float = 1.0) -> Image.Image:
    """Recolour an RGBA image to target hue (degrees) while preserving luminance & specular highlights."""
    arr = np.array(img).astype(np.float32) / 255.0
    r, g, b, a = arr[..., 0], arr[..., 1], arr[..., 2], arr[..., 3]
    out = np.zeros_like(arr)
    target_h = (target_deg % 360.0) / 360.0
    for y in range(arr.shape[0]):
        for x in range(arr.shape[1]):
            if a[y, x] <= 0.0:
                continue
            _, s, v = colorsys.rgb_to_hsv(float(r[y, x]), float(g[y, x]), float(b[y, x]))
            nr, ng, nb = colorsys.hsv_to_rgb(target_h, min(1.0, s * sat_scale), v)
            out[y, x] = (nr, ng, nb, a[y, x])
    return Image.fromarray(np.clip(out * 255.0, 0, 255).astype(np.uint8), "RGBA")


def _render_xbox_variant(out: Path, dark: bool) -> None:
    w_out = out / "weather"
    out.mkdir(parents=True, exist_ok=True)
    w_out.mkdir(parents=True, exist_ok=True)

    glass_tile = Image.open(VENDOR_XBOX / "guide" / "glass.png").convert("RGBA")
    glass_overlay = Image.open(VENDOR_XBOX / "glass-overlay.png").convert("RGBA")

    # 5a. 2005 Blades & Guide Backdrop (380x700 -> 1x/2x) composited from real upstream assets
    w1, h1 = 380, 700
    w2, h2 = w1 * 2, h1 * 2
    bg_name = "carbon_guideline_background.png" if dark else "guideline_background.png"
    src_bg = Image.open(VENDOR_XBOX / bg_name).convert("RGBA")
    # Center-crop aspect ratio to 380:700 then resize smoothly to 760x1400
    sw, sh = src_bg.size
    target_ratio = w2 / h2
    crop_w = min(sw, int(round(sh * target_ratio)))
    x_off = (sw - crop_w) // 2
    canvas = src_bg.crop((x_off, 0, x_off + crop_w, sh)).resize((w2, h2), Image.Resampling.LANCZOS)

    # Subtle brushed aluminum micro-grain from real guide/glass.png
    tiled_glass = _tile_texture(glass_tile, w2, h2)
    arr_c = np.array(canvas, dtype=np.float32)
    arr_g = np.array(tiled_glass, dtype=np.float32)
    grain_mix = 0.07 if dark else 0.10
    arr_c[..., :3] = np.clip(arr_c[..., :3] * (1.0 - grain_mix) + arr_g[..., :3] * grain_mix, 0, 255)
    canvas = Image.fromarray(arr_c.astype(np.uint8), "RGBA")

    # Composite real Xbox 360 concentric energy rings (background_energy_spin.svg) softly
    spin_svg = (VENDOR_XBOX / "background_energy_spin.svg").read_text(encoding="utf-8")
    spin_img = render_svg_bytes(spin_svg, 680, 680)
    spin_arr = np.array(spin_img, dtype=np.float32)
    spin_arr[..., 3] *= (0.14 if dark else 0.11)
    spin_soft = Image.fromarray(np.clip(spin_arr, 0, 255).astype(np.uint8), "RGBA")
    canvas.alpha_composite(spin_soft, (40, 220))

    # Subtle Guide curved header/footer framing wash
    hdr_svg = (VENDOR_XBOX / "guide" / "header.svg").read_text(encoding="utf-8")
    ftr_svg = (VENDOR_XBOX / "guide" / "footer.svg").read_text(encoding="utf-8")
    hdr_img = render_svg_bytes(hdr_svg, w2, 84)
    ftr_img = render_svg_bytes(ftr_svg, w2, 64)
    hdr_arr = np.array(hdr_img, dtype=np.float32)
    hdr_arr[..., 3] *= (0.28 if dark else 0.16)
    ftr_arr = np.array(ftr_img, dtype=np.float32)
    ftr_arr[..., 3] *= (0.30 if dark else 0.18)
    canvas.alpha_composite(Image.fromarray(hdr_arr.astype(np.uint8), "RGBA"), (0, 0))
    canvas.alpha_composite(Image.fromarray(ftr_arr.astype(np.uint8), "RGBA"), (0, h2 - 64))

    # Real stacked side blade slivers strictly along the outer 6-8px (12-16px @2x) edges
    j_orange = render_svg_bytes((VENDOR_XBOX / "blades" / "blade_1_jewel.svg").read_text(encoding="utf-8"), 64, h2)
    j_blue = render_svg_bytes((VENDOR_XBOX / "blades" / "blade_3_jewel.svg").read_text(encoding="utf-8"), 64, h2 // 2 + 120)
    j_violet = render_svg_bytes((VENDOR_XBOX / "blades" / "blade_4_jewel.svg").read_text(encoding="utf-8"), 64, h2 // 2 + 120)
    rim_img = render_svg_bytes((VENDOR_XBOX / "blades" / "wing-rim.svg").read_text(encoding="utf-8"), 28, h2)
    rim_arr = np.array(rim_img, dtype=np.float32)
    rim_arr[..., 3] *= (0.78 if dark else 0.85)
    rim_img = Image.fromarray(rim_arr.astype(np.uint8), "RGBA")

    edge_strip = Image.new("RGBA", (w2, h2), (0, 0, 0, 0))
    # Left Xbox Live Orange blade sliver
    left_sliver = j_orange.crop((48, 0, 62, h2))
    if dark:
        left_sliver = ImageEnhance.Brightness(left_sliver).enhance(0.76)
    edge_strip.alpha_composite(left_sliver, (-4, 0))
    edge_strip.alpha_composite(rim_img.crop((14, 0, 28, h2)), (-2, 0))

    # Right Media Blue + System Violet blade slivers
    right_top = j_blue.transpose(Image.Transpose.FLIP_LEFT_RIGHT).crop((2, 0, 16, h2 // 2 + 120))
    right_bot = j_violet.transpose(Image.Transpose.FLIP_LEFT_RIGHT).crop((2, 0, 16, h2 // 2 + 120))
    if dark:
        right_top = ImageEnhance.Brightness(right_top).enhance(0.82)
        right_bot = ImageEnhance.Brightness(right_bot).enhance(0.82)
    edge_strip.alpha_composite(right_bot, (w2 - 10, h2 // 2 - 120))
    edge_strip.alpha_composite(right_top, (w2 - 10, 0))
    rim_r = rim_img.transpose(Image.Transpose.FLIP_LEFT_RIGHT).crop((0, 0, 14, h2))
    edge_strip.alpha_composite(rim_r, (w2 - 12, 0))

    canvas.alpha_composite(edge_strip)
    canvas.save(out / "xbox_backdrop@2x.png", optimize=True)
    canvas.resize((w1, h1), Image.Resampling.LANCZOS).save(out / "xbox_backdrop.png", optimize=True)

    # 5b. Real Xenon Guide Sphere + 4-Quadrant Ring of Light Housing (56x56 -> 1x/2x/3x)
    # Drawn at 168x168 (3x) from real notify_bg.png (107x109) + real xenonLogo.png (108x108).
    notify_bg = Image.open(VENDOR_XBOX / "notifications" / "notify_bg.png").convert("RGBA")
    xenon_logo = Image.open(VENDOR_XBOX / "notifications" / "xenonLogo.png").convert("RGBA")

    # Extract tight sphere from xenonLogo.png (opaque bbox 28,29..82,81)
    x_crop = xenon_logo.crop((27, 27, 83, 83))
    orb_3x = Image.new("RGBA", (168, 168), (0, 0, 0, 0))
    # Subtle drop shadow
    sh_draw = ImageDraw.Draw(orb_3x)
    sh_draw.ellipse((14, 17, 154, 157), fill=(0, 0, 0, 68))
    orb_3x = orb_3x.filter(ImageFilter.GaussianBlur(2.5))
    # Recessed dark core behind the transparent center of notify_bg
    core_draw = ImageDraw.Draw(orb_3x)
    core_fill = (11, 14, 18, 255) if dark else (26, 34, 42, 255)
    core_draw.ellipse((24, 24, 144, 144), fill=core_fill)

    bg_scaled = notify_bg.resize((152, 154), Image.Resampling.LANCZOS)
    if dark:
        bg_scaled = ImageEnhance.Brightness(bg_scaled).enhance(0.72)
    orb_3x.alpha_composite(bg_scaled, (8, 7))

    # Place real 3D Xenon sphere cleanly inside the Ring of Light inner cutout
    sphere_sz = 100
    sphere_scaled = x_crop.resize((sphere_sz, sphere_sz), Image.Resampling.LANCZOS)
    if dark:
        sphere_scaled = ImageEnhance.Contrast(ImageEnhance.Brightness(sphere_scaled).enhance(0.92)).enhance(1.08)
    orb_3x.alpha_composite(sphere_scaled, ((168 - sphere_sz) // 2, (168 - sphere_sz) // 2))
    save_scaled_png(orb_3x, out / "xbox_guide_orb", 56, 56)

    # 5c. Real Ring of Light LED Quadrant overlays (56x56 -> 1x/2x/3x) using notify_Indicator1.png
    ind_base = Image.open(VENDOR_XBOX / "notifications" / "notify_Indicator1.png").convert("RGBA")
    ind_amber = _hue_shift_rgba(ind_base, 36.0, 1.05)
    ind_red = _hue_shift_rgba(ind_base, 2.0, 1.08)

    # Quadrant placement inside notify_bg (107x109) scaled to 152x154 at (8,7)
    # In 107x109 space, Q1 (47x47) sits at (5, 6); mapped to 168x168 canvas:
    sx, sy = 152.0 / 107.0, 154.0 / 109.0
    qw, qh = int(round(47 * sx)), int(round(47 * sy))
    ox, oy = 8 + int(round(5 * sx)), 7 + int(round(6 * sy))
    q_poses = {
        1: (ox, oy, Image.Transpose.ROTATE_90),  # placeholder, overwritten below
    }
    def make_quad(src: Image.Image, q: int) -> Image.Image:
        q_img = src
        if q == 2:
            q_img = src.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        elif q == 3:
            q_img = src.transpose(Image.Transpose.ROTATE_180)
        elif q == 4:
            q_img = src.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
        return q_img.resize((qw, qh), Image.Resampling.LANCZOS)

    rol_specs = {
        "xbox_rol_live": ([1], ind_base),
        "xbox_rol_manual": ([1, 2], ind_base),
        "xbox_rol_paused": ([1, 2, 3, 4], ind_amber),
        "xbox_rol_offline": ([1, 3, 4], ind_red),
    }
    coords = {
        1: (ox, oy),
        2: (168 - ox - qw, oy),
        3: (168 - ox - qw, 168 - oy - qh),
        4: (ox, 168 - oy - qh),
    }
    for name, (qs, src_led) in rol_specs.items():
        rol_3x = Image.new("RGBA", (168, 168), (0, 0, 0, 0))
        for q in qs:
            rol_3x.alpha_composite(make_quad(src_led, q), coords[q])
        save_scaled_png(rol_3x, out / name, 56, 56)

    # 5d. Authentic 2005 Blades Tabs (104x36 -> 1x/2x/3x) with real curved jewel spines
    # Brightness active uses Games Green (blade_2_jewel.svg); Weather active uses Media Blue (blade_3_jewel.svg);
    # Idle uses brushed silver (blade_grey_left.svg).
    jewel_assets = {
        "xbox_blade_active": ("blade_2_jewel.svg", "blade_2_bgcolor.svg", True, "#5a9612" if not dark else "#74d834", "#98e848"),
        "xbox_blade_weather_active": ("blade_3_jewel.svg", "blade_3_bgcolor.svg", True, "#347fc3" if not dark else "#67b8fa", "#88c8ff"),
        "xbox_blade_idle": ("blade_grey_left.svg", None, False, "#7c8894" if not dark else "#46525e", "#c8d0d8"),
    }
    w_b, h_b = 104 * 3, 36 * 3
    for name, (svg_file, bg_svg_file, is_active, rim_col, inner_hi) in jewel_assets.items():
        if dark:
            b0, b1, b2 = ("#323c46", "#232a32", "#161b22") if is_active else ("#252d36", "#1b2128", "#13181e")
            gloss_op = "0.22" if is_active else "0.12"
        else:
            b0, b1, b2 = ("#ffffff", "#f1f5f8", "#d7e0e8") if is_active else ("#e9eef2", "#d0d8e0", "#b6c0ca")
            gloss_op = "0.78" if is_active else "0.62"

        base_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="104" height="36" viewBox="0 0 104 36">
          <defs>
            <linearGradient id="bl" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{b0}"/>
              <stop offset="0.52" stop-color="{b1}"/>
              <stop offset="1" stop-color="{b2}"/>
            </linearGradient>
            <linearGradient id="blGloss" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="#ffffff" stop-opacity="{gloss_op}"/>
              <stop offset="1" stop-color="#ffffff" stop-opacity="0.0"/>
            </linearGradient>
          </defs>
          <path d="M12,4 L92,4 Q98,4 102,19 Q98,34 92,34 L12,34 Q6,34 2,19 Q6,4 12,4 Z" fill="#000000" fill-opacity="0.24"/>
          <path d="M12,3 L92,3 Q98,3 102,18 Q98,33 92,33 L12,33 Q6,33 2,18 Q6,3 12,3 Z" fill="url(#bl)"/>
          <path d="M13,4.5 L91,4.5 Q95,4.5 98,14.5 L6,14.5 Q9,4.5 13,4.5 Z" fill="url(#blGloss)"/>
        </svg>'''
        tab_img = render_svg_bytes(base_svg, w_b, h_b)

        # Clip mask for the curved left jewel spine inside the blade silhouette
        clip_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="104" height="36" viewBox="0 0 104 36">
          <path d="M12,3 L19,3 Q13,18 19,33 L12,33 Q6,33 2,18 Q6,3 12,3 Z" fill="#ffffff"/>
        </svg>'''
        spine_mask = render_svg_bytes(clip_svg, w_b, h_b).getchannel("A")

        jewel_raw = render_svg_bytes((VENDOR_XBOX / "blades" / svg_file).read_text(encoding="utf-8"), 136, 480)
        if bg_svg_file:
            # Composite translucent real jewel gloss over its matching real Xbox 360 blade gradient
            bg_raw = render_svg_bytes((VENDOR_XBOX / "blades" / bg_svg_file).read_text(encoding="utf-8"), 64, h_b)
            jewel_band = jewel_raw.crop((50, 175, 90, 305)).resize((62, h_b), Image.Resampling.LANCZOS)
            spine_fill = bg_raw.crop((0, 0, 62, h_b))
            spine_fill.alpha_composite(jewel_band)
        else:
            # Crop strictly inside the brushed silver body of blade_grey_left.svg (no outer black shadow)
            spine_fill = jewel_raw.crop((46, 175, 76, 305)).resize((62, h_b), Image.Resampling.LANCZOS)

        spine_layer = Image.new("RGBA", (w_b, h_b), (0, 0, 0, 0))
        spine_layer.paste(spine_fill, (0, 0))
        spine_layer.putalpha(spine_mask)
        tab_img.alpha_composite(spine_layer)

        # Blend faint brushed metal grain on the tab face & draw crisp outer bezel rim
        mask_body = render_svg_bytes(
            '<svg xmlns="http://www.w3.org/2000/svg" width="104" height="36" viewBox="0 0 104 36">'
            '<path d="M12,3 L92,3 Q98,3 102,18 Q98,33 92,33 L12,33 Q6,33 2,18 Q6,3 12,3 Z" fill="#fff"/></svg>',
            w_b, h_b,
        ).getchannel("A")
        grain_b = _tile_texture(glass_tile, w_b, h_b)
        arr_t = np.array(tab_img, dtype=np.float32)
        arr_gb = np.array(grain_b, dtype=np.float32)
        arr_mb = np.array(mask_body, dtype=np.float32) / 255.0
        arr_t[..., :3] = np.clip(arr_t[..., :3] * (1.0 - 0.06 * arr_mb[..., None]) + arr_gb[..., :3] * (0.06 * arr_mb[..., None]), 0, 255)
        tab_img = Image.fromarray(arr_t.astype(np.uint8), "RGBA")

        rim_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="104" height="36" viewBox="0 0 104 36">
          <path d="M12,3 L92,3 Q98,3 102,18 Q98,33 92,33 L12,33 Q6,33 2,18 Q6,3 12,3 Z" fill="none" stroke="{rim_col}" stroke-width="1.5"/>
          <path d="M13,4.2 L91,4.2" fill="none" stroke="{inner_hi}" stroke-opacity="{0.45 if is_active else 0.55}" stroke-width="0.8"/>
        </svg>'''
        tab_img.alpha_composite(render_svg_bytes(rim_svg, w_b, h_b))
        save_scaled_png(tab_img, out / name, 104, 36)

    # 5e. Brushed Metallic Guide Card (88x64 -> 1x/2x/3x) backed by mirror-tiled guide/glass.png
    if dark:
        card_stops = ("#2c353e", "#222931", "#1b2128", "#151a20")
        edge_stops = ("#566472", "#343e48")
        card_shine = "0.18"
        card_shadow = "0.32"
        glass_mix = 0.05
    else:
        card_stops = ("#ffffff", "#f2f5f8", "#e4eaf0", "#d4dde6")
        edge_stops = ("#9aa6b2", "#75828f")
        card_shine = "0.88"
        card_shadow = "0.14"
        glass_mix = 0.06

    card_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="88" height="64" viewBox="0 0 88 64">
      <defs>
        <linearGradient id="cbg" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="{card_stops[0]}"/>
          <stop offset="0.18" stop-color="{card_stops[1]}"/>
          <stop offset="0.82" stop-color="{card_stops[2]}"/>
          <stop offset="1" stop-color="{card_stops[3]}"/>
        </linearGradient>
        <linearGradient id="cEdge" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="{edge_stops[0]}"/>
          <stop offset="1" stop-color="{edge_stops[1]}"/>
        </linearGradient>
      </defs>
      <rect x="1.5" y="2.5" width="85" height="60" rx="9" fill="#000000" fill-opacity="{card_shadow}"/>
      <rect x="1" y="1" width="86" height="60.5" rx="9" fill="url(#cbg)" stroke="url(#cEdge)" stroke-width="1.2"/>
      <line x1="9" y1="2.4" x2="79" y2="2.4" stroke="#ffffff" stroke-opacity="{card_shine}" stroke-width="1.0"/>
    </svg>'''
    cw3, ch3 = 88 * 3, 64 * 3
    card_3x = render_svg_bytes(card_svg, cw3, ch3)
    card_mask = render_svg_bytes(
        '<svg xmlns="http://www.w3.org/2000/svg" width="88" height="64" viewBox="0 0 88 64">'
        '<rect x="2" y="2" width="84" height="58.5" rx="8" fill="#ffffff"/></svg>',
        cw3, ch3,
    ).getchannel("A")
    # Blend subtle seamless brushed aluminum texture from guide/glass.png (zero horizontal banding when 9-sliced)
    grain_c = _tile_texture(glass_tile, cw3, ch3)
    arr_card = np.array(card_3x, dtype=np.float32)
    arr_gc = np.array(grain_c, dtype=np.float32)
    arr_mc = np.array(card_mask, dtype=np.float32) / 255.0
    arr_card[..., :3] = np.clip(
        arr_card[..., :3] * (1.0 - glass_mix * arr_mc[..., None])
        + arr_gc[..., :3] * (glass_mix * arr_mc[..., None]),
        0, 255,
    )
    save_scaled_png(Image.fromarray(arr_card.astype(np.uint8), "RGBA"), out / "xbox_card", 88, 64)

    # 5f. Blades Selector Pill Buttons (72x30 -> 1x/2x/3x) matching guide_menu_button.png & blades.css
    pill_table = (
        [
            ("xbox_pill_idle", "#35404b", "#242c35", "#181e25", "#52606e", "1.3", "0.18", "#7c8b99"),
            ("xbox_pill_called", "#2b4523", "#1f3419", "#152512", "#74d438", "1.6", "0.20", "#a6ec68"),
            ("xbox_pill_active", "#a9d43c", "#6cb822", "#5a9612", "#c8f278", "1.4", "0.42", "#e8ffc0"),
        ]
        if dark
        else [
            ("xbox_pill_idle", "#f8faf8", "#dfe6ea", "#b4bfc3", "#7e8b96", "1.3", "0.80", "#ffffff"),
            ("xbox_pill_called", "#eef8e4", "#d2edc0", "#b6de9a", "#5a9612", "1.6", "0.72", "#ffffff"),
            ("xbox_pill_active", "#a9d43c", "#6fb824", "#5a9612", "#3b680a", "1.4", "0.52", "#d9f59a"),
        ]
    )
    for name, c0, c1, c2, rim, rw, hi, inset_hi in pill_table:
        pill_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="72" height="30" viewBox="0 0 72 30">
          <defs>
            <linearGradient id="xp" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{c0}"/>
              <stop offset="0.52" stop-color="{c1}"/>
              <stop offset="1" stop-color="{c2}"/>
            </linearGradient>
          </defs>
          <rect x="1.5" y="2.2" width="69" height="26.5" rx="13.2" fill="#000000" fill-opacity="0.20"/>
          <rect x="1.5" y="1.2" width="69" height="27" rx="13.5" fill="url(#xp)" stroke="{rim}" stroke-width="{rw}"/>
          <path d="M14,2.3 L58,2.3" stroke="{inset_hi}" stroke-opacity="0.85" stroke-width="0.9" stroke-linecap="round"/>
          <rect x="4" y="2.5" width="64" height="10" rx="5" fill="#ffffff" fill-opacity="{hi}"/>
        </svg>'''
        render_svg(pill_svg, out / name, 72, 30)

    # 5f2. Action Sphere Button (40x40 -> 1x/2x/3x) for RefreshControl
    for name, active in [("xbox_orb_btn", False), ("xbox_orb_btn_active", True)]:
        if active:
            c0, c1, rim, hi, inner_col, inner_op = ("#a9d43c", "#5a9612", "#c8f278" if dark else "#366008", "0.42" if dark else "0.52", "#e8ffc0", 0.85)
        elif dark:
            c0, c1, rim, hi, inner_col, inner_op = ("#3e4956", "#1c222a", "#586776", "0.20", "#83c52b", 0.60)
        else:
            c0, c1, rim, hi, inner_col, inner_op = ("#ffffff", "#c4cfd6", "#7a8692", "0.82", "#5a9612", 0.50)
        xorb_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">
          <defs>
            <linearGradient id="xo" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{c0}"/>
              <stop offset="1" stop-color="{c1}"/>
            </linearGradient>
          </defs>
          <circle cx="20" cy="21" r="16.5" fill="#000000" fill-opacity="0.24"/>
          <circle cx="20" cy="20" r="16.5" fill="url(#xo)" stroke="{rim}" stroke-width="1.4"/>
          <circle cx="20" cy="20" r="14.2" fill="none" stroke="{inner_col}" stroke-opacity="{inner_op}" stroke-width="1"/>
          <ellipse cx="20" cy="12" rx="10" ry="4.5" fill="#ffffff" fill-opacity="{hi}"/>
        </svg>'''
        render_svg(xorb_svg, out / name, 40, 40)

    # 5g. Chrome vs Dark-Gunmetal & Emerald Slider Thumb (24x24 -> 1x/2x/3x)
    if dark:
        t_stops = ("#5c6876", "#2f3842", "#161b22")
        t_rim, t_core = "#83c52b", "#a9d43c"
    else:
        t_stops = ("#ffffff", "#d0dae2", "#7c8896")
        t_rim, t_core = "#4b8a10", "#6cc020"
    x_thumb = f'''<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
      <defs>
        <radialGradient id="xt" cx="0.45" cy="0.32" r="0.65">
          <stop offset="0" stop-color="{t_stops[0]}"/>
          <stop offset="0.65" stop-color="{t_stops[1]}"/>
          <stop offset="1" stop-color="{t_stops[2]}"/>
        </radialGradient>
      </defs>
      <circle cx="12" cy="12.6" r="9.0" fill="#000000" fill-opacity="0.28"/>
      <circle cx="12" cy="12" r="8.4" fill="url(#xt)" stroke="{t_rim}" stroke-width="1.8"/>
      <circle cx="12" cy="12" r="3.4" fill="{t_core}"/>
    </svg>'''
    render_svg(x_thumb, out / "xbox_slider_thumb", 24, 24)

    # 5h. Glossy 2005 Xenon / Frutiger-Aero Weather Icons (64x64 -> 1x/2x/3x)
    cloud_stops = ("#ffffff", "#dbe4ee", "#9aa9ba") if dark else ("#ffffff", "#e2eaf2", "#a2b0c0")
    cloud_stroke = "#414e5c" if dark else "#566575"
    mist_stroke = "#a5bad0" if dark else "#5a6d80"
    q_fill = "#a9d43c" if dark else "#3f7d0c"
    star_fill = "#a9d43c" if dark else "#5a9612"
    flake_stroke = "#7ec4ff" if dark else "#3d7ab8"

    x_defs = f'''<defs>
      <radialGradient id="xSunCore" cx="0.38" cy="0.34" r="0.66">
        <stop offset="0" stop-color="#fff8bf"/>
        <stop offset="0.52" stop-color="#ffb828"/>
        <stop offset="1" stop-color="#d46308"/>
      </radialGradient>
      <radialGradient id="xSunHalo">
        <stop offset="0.32" stop-color="#ffbe33" stop-opacity="0.42"/>
        <stop offset="1" stop-color="#ffbe33" stop-opacity="0"/>
      </radialGradient>
      <linearGradient id="xCloud" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="{cloud_stops[0]}"/>
        <stop offset="0.55" stop-color="{cloud_stops[1]}"/>
        <stop offset="1" stop-color="{cloud_stops[2]}"/>
      </linearGradient>
      <linearGradient id="xDrop" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#6ee2ff"/>
        <stop offset="0.55" stop-color="#229ee6"/>
        <stop offset="1" stop-color="#0a5fa3"/>
      </linearGradient>
      <linearGradient id="xMoon" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#fff4ad"/>
        <stop offset="0.55" stop-color="#f0c438"/>
        <stop offset="1" stop-color="#b87e12"/>
      </linearGradient>
    </defs>'''

    def x_sun(cx: float, cy: float, r: float, faint: bool = False) -> str:
        op = 'opacity="0.64"' if faint else ""
        rays = "".join(
            f'<line x1="{cx + r * 1.32 * math.cos(a):.2f}" y1="{cy + r * 1.32 * math.sin(a):.2f}" '
            f'x2="{cx + r * 1.84 * math.cos(a):.2f}" y2="{cy + r * 1.84 * math.sin(a):.2f}" '
            f'stroke="#f59818" stroke-width="2.6" stroke-linecap="round"/>'
            for a in [k * math.pi / 4 for k in range(8)]
        )
        gloss = f'<ellipse cx="{cx - r * 0.24:.2f}" cy="{cy - r * 0.34:.2f}" rx="{r * 0.45:.2f}" ry="{r * 0.24:.2f}" fill="#ffffff" fill-opacity="0.52" transform="rotate(-24 {cx - r * 0.24:.2f} {cy - r * 0.34:.2f})"/>'
        return f'<g {op}><circle cx="{cx}" cy="{cy}" r="{r * 1.98}" fill="url(#xSunHalo)"/>{rays}<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#xSunCore)" stroke="#a85006" stroke-width="1.0"/>{gloss}</g>'

    def x_moon(cx: float, cy: float, r: float) -> str:
        return f'''<mask id="xm_{int(cx)}">
          <rect width="64" height="64" fill="black"/>
          <circle cx="{cx}" cy="{cy}" r="{r}" fill="white"/>
          <circle cx="{cx + r * 0.45}" cy="{cy - r * 0.35}" r="{r * 0.85}" fill="black"/>
        </mask>
        <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#xMoon)" stroke="#7c560a" stroke-width="1.1" mask="url(#xm_{int(cx)})"/>'''

    def x_cloud(cx: float, cy: float, s: float) -> str:
        return f'''<g transform="translate({cx},{cy}) scale({s})">
          <g fill="#000000" fill-opacity="0.22" transform="translate(0, 1.8)">
            <circle cx="-9.5" cy="3" r="8"/><circle cx="0.5" cy="-3.5" r="10.2"/><circle cx="10.5" cy="2.5" r="7.8"/><rect x="-11" y="1" width="22" height="10" rx="5"/>
          </g>
          <g fill="url(#xCloud)" stroke="{cloud_stroke}" stroke-width="1.3">
            <circle cx="-9.5" cy="3" r="8"/>
            <circle cx="0.5" cy="-3.5" r="10.2"/>
            <circle cx="10.5" cy="2.5" r="7.8"/>
            <rect x="-11" y="1" width="22" height="10" rx="5"/>
          </g>
          <g fill="url(#xCloud)">
            <circle cx="-9.5" cy="3" r="8"/>
            <circle cx="0.5" cy="-3.5" r="10.2"/>
            <circle cx="10.5" cy="2.5" r="7.8"/>
            <rect x="-11" y="1" width="22" height="10" rx="5"/>
          </g>
          <path d="M-7,-6 A9,9 0 0,1 8,-7" fill="none" stroke="#ffffff" stroke-width="1.5" stroke-linecap="round"/>
        </g>'''

    def x_drop(x: float, top: float, w: float = 4.6, h: float = 9.5) -> str:
        r = w / 2.0
        yc = top + h - r
        return (
            f'<path d="M{x},{top} C{x + 0.6 * r},{top + 0.42 * h} {x + r},{yc - 0.5 * r} {x + r},{yc} '
            f'A{r},{r} 0 1,1 {x - r},{yc} C{x - r},{yc - 0.5 * r} {x - 0.6 * r},{top + 0.42 * h} {x},{top} Z" '
            f'fill="url(#xDrop)" stroke="#084c82" stroke-width="0.65"/>'
            f'<line x1="{x - 0.35 * r:.2f}" y1="{yc - 0.1 * r:.2f}" x2="{x - 0.32 * r:.2f}" y2="{yc - 0.65 * r:.2f}" '
            f'stroke="#ffffff" stroke-opacity="0.82" stroke-width="{0.35 * r:.2f}" stroke-linecap="round"/>'
        )

    def x_flake(cx: float, cy: float, r: float) -> str:
        lines = "".join(
            f'<line x1="{cx}" y1="{cy}" x2="{cx + r * math.cos(a):.2f}" y2="{cy + r * math.sin(a):.2f}"/>'
            for a in [k * math.pi / 3 - math.pi / 2 for k in range(6)]
        )
        return f'<g stroke="{flake_stroke}" stroke-width="2.1" stroke-linecap="round">{lines}</g><g stroke="#ffffff" stroke-width="0.9" stroke-linecap="round">{lines}</g>'

    x_cases = {
        "clear_day": x_sun(32, 32, 12.5),
        "clear_night": x_moon(28, 33, 13) + f'<path d="M47 13 L48.5 16.5 L52 18 L48.5 19.5 L47 23 L45.5 19.5 L42 18 L45.5 16.5 Z" fill="{star_fill}" stroke="#285c12" stroke-width="0.7"/><circle cx="51" cy="33" r="2.2" fill="#e89a1c"/>',
        "partly_cloudy_day": x_sun(22, 22, 9.8) + x_cloud(34, 38, 0.94),
        "partly_cloudy_night": x_moon(21, 22, 9.8) + f'<circle cx="49" cy="14" r="2.2" fill="{star_fill}"/>' + x_cloud(34, 38, 0.94),
        "cloudy": x_cloud(38, 25, 0.72) + x_cloud(29, 38, 0.98),
        "drizzle": x_cloud(32, 25, 0.95) + x_drop(22, 41, 4.2, 8.8) + x_drop(33, 45, 4.2, 8.8) + x_drop(44, 41, 4.2, 8.8),
        "rain": x_cloud(32, 23, 0.96) + x_drop(20, 40, 4.6, 10.2) + x_drop(30, 44, 4.6, 10.2) + x_drop(40, 40, 4.6, 10.2) + x_drop(49, 44, 4.6, 10.2),
        "heavy_rain": x_cloud(32, 22, 0.98) + x_drop(16, 40, 4.4, 9.8) + x_drop(25, 44, 4.4, 9.8) + x_drop(34, 40, 4.4, 9.8) + x_drop(43, 44, 4.4, 9.8) + x_drop(51, 40, 4.4, 9.8),
        "thunderstorm": x_cloud(32, 22, 0.98) + '<polygon points="35,34 44,34 36,44 43,44 29,59 33,47 27,47" fill="#ffb828" stroke="#965004" stroke-width="0.9"/>' + x_drop(18, 42, 4.2, 9.2) + x_drop(50, 42, 4.2, 9.2),
        "snow": x_cloud(32, 23, 0.95) + x_flake(19, 47, 5) + x_flake(33, 52, 4.5) + x_flake(46, 47, 4.8),
        "mist": f'<g stroke="{mist_stroke}" stroke-width="3.6" stroke-linecap="round"><line x1="10" y1="24" x2="54" y2="24"/><line x1="16" y1="33" x2="48" y2="33"/><line x1="9" y1="42" x2="44" y2="42"/><line x1="20" y1="51" x2="54" y2="51"/></g>',
        "atmospheric": x_sun(32, 24, 11, faint=True) + f'<g stroke="{mist_stroke}" stroke-width="3.8" stroke-linecap="round"><line x1="11" y1="42" x2="53" y2="42"/><line x1="18" y1="51" x2="46" y2="51"/></g>',
        "fog": x_cloud(32, 22, 0.90) + f'<g stroke="{mist_stroke}" stroke-width="4" stroke-linecap="round"><line x1="9" y1="40" x2="50" y2="40"/><line x1="16" y1="49" x2="55" y2="49"/></g>',
        "unknown": x_cloud(32, 27, 0.95) + f'<text x="32" y="54" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="16" fill="{q_fill}">?</text>',
    }
    for cond, body in x_cases.items():
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">{x_defs}{body}</svg>'
        render_svg(svg, w_out / cond, 64, 64)


def build_xbox360() -> None:
    base = SKINS / "xbox360" / "assets"
    _render_xbox_variant(base, dark=False)
    _render_xbox_variant(base / "dark", dark=True)


def main() -> None:
    build_nds()
    build_wii()
    build_nothing()
    build_ps3()
    build_xbox360()
    print("Generated assets for nds, wii, nothing, ps3, xbox360.")


if __name__ == "__main__":
    main()
