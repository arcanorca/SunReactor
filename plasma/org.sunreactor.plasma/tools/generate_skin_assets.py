#!/usr/bin/env python3
"""Generate authentic HiDPI (@1x, @2x, @3x) raster and SVG assets for the
flagship instrument skins: nds, wii, nothing, ps3, xbox360.

Uses SVG + rsvg-convert (Cairo) for sharp vector gradients, specular glass
highlights, bevels, and Gaussian blooms, plus NumPy/Pillow for brushed-metal
and pinstripe textures. Every asset is deterministic and written directly into
package/contents/ui/skins/<id>/assets/.
"""

from __future__ import annotations

import math
import subprocess
from pathlib import Path
import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
SKINS = HERE.parent / "package" / "contents" / "ui" / "skins"


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

    # 1c. DS Hinge barrel (9-slice horizontal: left=36, right=36, top=0, bottom=0; logical 160x18)
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
      <rect x="138" y="6" width="5" height="6" fill="#212630" stroke="#7d8796" stroke-width="0.8"/>
      <rect x="145" y="6" width="5" height="6" fill="#212630" stroke="#7d8796" stroke-width="0.8"/>
    </svg>'''
    render_svg(hinge_svg, out / "ds_hinge", 160, 18)

    cam_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="32" height="14" viewBox="0 0 32 14">
      <rect x="1" y="2" width="30" height="10" rx="3" fill="#232832" stroke="#7c8798" stroke-width="1"/>
      <circle cx="16" cy="7" r="3.4" fill="#0c0f15" stroke="#475163" stroke-width="0.8"/>
      <rect x="14" y="5" width="1.5" height="1.5" fill="#7aa0d4"/>
      <rect x="23" y="6" width="2" height="2" fill="#101318"/>
    </svg>'''
    render_svg(cam_svg, out / "ds_hinge_camera", 32, 14)

    # 1d. Crisp DS LCD Pixel Grid + Scanline tiles (16x16 logical, nearest-neighbor)
    sky_grid = Image.new("RGBA", (16, 16), (235, 244, 255, 255))
    px_sky = sky_grid.load()
    for y in range(16):
        for x in range(16):
            if y == 0 or x == 0:
                px_sky[x, y] = (212, 227, 247, 255)
            elif y % 2 == 1:
                px_sky[x, y] = (228, 239, 253, 255)
    save_pixel_png(sky_grid, out / "ds_sky_grid")

    picto_grid = Image.new("RGBA", (16, 16), (247, 250, 254, 255))
    px_picto = picto_grid.load()
    for y in range(16):
        for x in range(16):
            if y == 0 or x == 0:
                px_picto[x, y] = (214, 222, 234, 255)
            elif y % 2 == 1:
                px_picto[x, y] = (238, 243, 249, 255)
    save_pixel_png(picto_grid, out / "ds_picto_grid")

    # 1e. Pixelated DS System Menu 9-slice buttons/panels (32x32 logical, border=4)
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
        outer=(44, 54, 72, 255),
        hi=(255, 255, 255, 255),
        shade=(196, 208, 224, 255),
        fill_a=(250, 252, 255, 255),
        fill_b=(239, 245, 253, 255),
        checker=False,
    )
    make_ds_pixel_panel(
        "ds_btn_active",
        outer=(20, 42, 92, 255),
        hi=(132, 184, 255, 255),
        shade=(22, 62, 146, 255),
        fill_a=(44, 106, 222, 255),
        fill_b=(34, 92, 204, 255),
        checker=True,
    )
    make_ds_pixel_panel(
        "ds_btn_quiet",
        outer=(124, 136, 154, 255),
        hi=(255, 255, 255, 255),
        shade=(198, 206, 218, 255),
        fill_a=(236, 240, 246, 255),
        fill_b=(224, 230, 238, 255),
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

    # 1g. Pixel-art ascending signal bars (16x12: 4 bars indicating proximity to max brightness)
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
    # Clean outer Glyph LED segments framing DotRing without overlapping dots or adding a conflicting red dot
    glyph_svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120" viewBox="0 0 120 120">
      <defs>
        <filter id="glow"><feGaussianBlur stdDeviation="1.2" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
      </defs>
      <path d="M 32 13 A 55 55 0 0 1 92 16" fill="none" stroke="#ffffff" stroke-opacity="0.28" stroke-width="2.6" stroke-linecap="round" filter="url(#glow)"/>
      <path d="M 103 26 A 55 55 0 0 1 113 72" fill="none" stroke="#ffffff" stroke-opacity="0.22" stroke-width="2.6" stroke-linecap="round"/>
      <path d="M 106 89 A 55 55 0 0 1 40 111" fill="none" stroke="#ffffff" stroke-opacity="0.24" stroke-width="2.6" stroke-linecap="round"/>
      <path d="M 22 100 A 55 55 0 0 1 14 29" fill="none" stroke="#ffffff" stroke-opacity="0.20" stroke-width="2.6" stroke-linecap="round"/>
    </svg>'''
    render_svg(glyph_svg, out / "nothing_glyph_ring", 120, 120)

    # 3a2. Nothing OS Circular Dot-Matrix Refresh Button (40x40)
    dots_ring = []
    for k in range(16):
        a = k * (2 * math.pi / 16) - math.pi / 2
        px = 20 + 16.2 * math.cos(a)
        py = 20 + 16.2 * math.sin(a)
        col = "#d71921" if k == 2 else "#ffffff"
        op = "0.95" if k == 2 else "0.24"
        r = "1.45" if k == 2 else "1.05"
        dots_ring.append(f'<circle cx="{px:.2f}" cy="{py:.2f}" r="{r}" fill="{col}" fill-opacity="{op}"/>')
    nothing_btn_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">
      <circle cx="20" cy="20" r="18.5" fill="#18181b" stroke="#ffffff" stroke-opacity="0.16" stroke-width="1"/>
      {''.join(dots_ring)}
    </svg>'''
    render_svg(nothing_btn_svg, out / "nothing_orb_btn", 40, 40)

    # 3b. Nothing OS Weather Icons (64x64 -> 1x/2x/3x):
    # Authentic Nothing Weather Widget look: geometric dot-halftone circles & clean rounded clouds + red dot signal
    def dot_circle_grid(cx: float, cy: float, r: float, step: float = 4.6, dot_r: float = 1.65, color: str = "#ffffff") -> str:
        dots = []
        n = int(math.ceil(r / step))
        for iy in range(-n, n + 1):
            for ix in range(-n, n + 1):
                px = cx + ix * step
                py = cy + iy * step
                if math.hypot(px - cx, py - cy) <= r - dot_r * 0.4:
                    dots.append(f'<circle cx="{px:.2f}" cy="{py:.2f}" r="{dot_r}" fill="{color}"/>')
        return "".join(dots)

    def n_sun(cx: float, cy: float, r: float, red_core: bool = True) -> str:
        inner = dot_circle_grid(cx, cy, r * 0.72, step=4.0, dot_r=1.45, color="#d71921" if red_core else "#ffffff")
        ring = []
        for k in range(12):
            a = k * math.pi / 6
            px = cx + math.cos(a) * r
            py = cy + math.sin(a) * r
            ring.append(f'<circle cx="{px:.2f}" cy="{py:.2f}" r="1.5" fill="#ffffff"/>')
        return inner + "".join(ring)

    def n_moon(cx: float, cy: float, r: float) -> str:
        dots = []
        step = 4.0
        n = int(math.ceil(r / step))
        for iy in range(-n, n + 1):
            for ix in range(-n, n + 1):
                px, py = cx + ix * step, cy + iy * step
                if math.hypot(px - cx, py - cy) <= r and math.hypot(px - (cx + r * 0.45), py - (cy - r * 0.3)) > r * 0.78:
                    dots.append(f'<circle cx="{px:.2f}" cy="{py:.2f}" r="1.45" fill="#ffffff"/>')
        return "".join(dots)

    def n_cloud(cx: float, cy: float, s: float, dotted_outline: bool = False) -> str:
        # Clean rounded geometric Nothing cloud (solid matte light grey or dot-filled silhouette)
        d = (
            f"M{cx - 12 * s:.2f},{cy + 10 * s:.2f} "
            f"A8.5,8.5 0 1,1 {cx - 10 * s:.2f},{cy - 5 * s:.2f} "
            f"A10.5,10.5 0 1,1 {cx + 10 * s:.2f},{cy - 3 * s:.2f} "
            f"A8,8 0 1,1 {cx + 12 * s:.2f},{cy + 10 * s:.2f} Z"
        )
        if dotted_outline:
            return f'<path d="{d}" fill="#262629" stroke="#ffffff" stroke-width="2.4" stroke-dasharray="0.1 4.2" stroke-linecap="round"/>'
        return f'<path d="{d}" fill="#f2f2f4"/>'

    def n_cloud_dots(cx: float, cy: float, s: float) -> str:
        # True Nothing OS halftone cloud: grid of crisp dots masked to a smooth rounded cloud silhouette!
        dots = []
        step = 4.2 * s
        for iy in range(-5, 5):
            for ix in range(-6, 7):
                px = cx + ix * step
                py = cy + iy * step
                in_left = math.hypot(px - (cx - 9.5 * s), py - (cy + 2 * s)) <= 8.5 * s
                in_top = math.hypot(px - (cx - 0.5 * s), py - (cy - 4 * s)) <= 10.5 * s
                in_right = math.hypot(px - (cx + 10 * s), py - (cy + 2.5 * s)) <= 7.8 * s
                in_base = (cx - 10 * s <= px <= cx + 10 * s) and (cy <= py <= cy + 10.5 * s)
                if in_left or in_top or in_right or in_base:
                    dots.append(f'<circle cx="{px:.2f}" cy="{py:.2f}" r="{1.55 * s:.2f}" fill="#ffffff"/>')
        return "".join(dots)

    def n_drop(x: float, y: float, red: bool = False) -> str:
        col = "#d71921" if red else "#ffffff"
        return (
            f'<circle cx="{x}" cy="{y}" r="1.5" fill="{col}"/>'
            f'<circle cx="{x - 1.5}" cy="{y + 4.5}" r="1.5" fill="{col}"/>'
            f'<circle cx="{x - 3.0}" cy="{y + 9.0}" r="1.6" fill="{col}"/>'
        )

    n_cases = {
        "clear_day": n_sun(32, 32, 20),
        "clear_night": n_moon(29, 34, 17) + '<circle cx="48" cy="16" r="2.2" fill="#d71921"/><circle cx="15" cy="17" r="1.5" fill="#ffffff"/><circle cx="49" cy="46" r="1.5" fill="#ffffff"/>',
        "partly_cloudy_day": n_sun(22, 22, 13.5, red_core=True) + n_cloud_dots(35, 39, 0.92),
        "partly_cloudy_night": n_moon(21, 22, 12.5) + '<circle cx="49" cy="14" r="2" fill="#d71921"/>' + n_cloud_dots(35, 39, 0.92),
        "cloudy": n_cloud_dots(39, 25, 0.68) + n_cloud_dots(29, 38, 0.96),
        "drizzle": n_cloud_dots(32, 25, 0.92) + n_drop(22, 43) + n_drop(34, 46, red=True) + n_drop(45, 43),
        "rain": n_cloud_dots(32, 23, 0.95) + n_drop(19, 41) + n_drop(30, 44, red=True) + n_drop(41, 41) + n_drop(50, 45),
        "heavy_rain": n_cloud_dots(32, 22, 0.96) + n_drop(16, 40) + n_drop(26, 43, red=True) + n_drop(36, 40) + n_drop(46, 43, red=True) + n_drop(54, 40),
        "thunderstorm": n_cloud_dots(32, 22, 0.96) + '<polygon points="34,37 42,37 36,46 42,46 28,59 32,49 27,49" fill="#d71921"/>' + n_drop(18, 43) + n_drop(50, 43),
        "snow": n_cloud_dots(32, 23, 0.92) + '<circle cx="20" cy="47" r="2" fill="#ffffff"/><circle cx="32" cy="52" r="2.2" fill="#d71921"/><circle cx="44" cy="47" r="2" fill="#ffffff"/><circle cx="25" cy="55" r="1.5" fill="#ffffff"/><circle cx="40" cy="55" r="1.5" fill="#ffffff"/>',
        "mist": '<g fill="#ffffff">' + "".join(f'<circle cx="{12 + i * 5}" cy="{y}" r="1.5"/>' for y in (22, 32, 42) for i in range(9) if (y != 32 or i > 0)) + '</g><circle cx="52" cy="32" r="1.8" fill="#d71921"/>',
        "atmospheric": n_sun(32, 24, 12, red_core=True) + '<g fill="#ffffff">' + "".join(f'<circle cx="{12 + i * 5}" cy="{y}" r="1.5"/>' for y in (42, 50) for i in range(9)) + '</g>',
        "fog": n_cloud_dots(32, 22, 0.88) + '<g fill="#ffffff">' + "".join(f'<circle cx="{12 + i * 5}" cy="{y}" r="1.6"/>' for y in (41, 49) for i in range(9)) + '</g><circle cx="52" cy="49" r="1.8" fill="#d71921"/>',
        "unknown": n_cloud_dots(32, 27, 0.92) + '<circle cx="32" cy="48" r="2.4" fill="#d71921"/>',
    }
    for cond, body in n_cases.items():
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">{body}</svg>'
        render_svg(svg, w_out / cond, 64, 64)


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
# 5. XBOX 360 (2005 Launch "Blades" Pearl-Silver Dashboard + Ring of Light)
# ==============================================================================

def _render_xbox_variant(out: Path, dark: bool) -> None:
    w_out = out / "weather"
    out.mkdir(parents=True, exist_ok=True)
    w_out.mkdir(parents=True, exist_ok=True)

    # 5a. 2005 Blades Frost Backdrop (Pearl-Silver Light vs Smoked-Carbon Dark) (380x700 -> 1x/2x)
    w1, h1 = 380, 700
    w2, h2 = w1 * 2, h1 * 2
    yy, xx = np.mgrid[0:h2, 0:w2].astype(np.float32)
    u = xx / w2
    v = yy / h2

    rad_center = np.sqrt(((u - 0.50) / 0.56) ** 2 + ((v - 0.44) / 0.50) ** 2)
    pearl = np.clip(1.0 - 0.72 * (rad_center ** 1.45), 0.0, 1.0)
    rng = np.random.default_rng(361 if dark else 360)
    streak = rng.normal(0.0, 1.0 if dark else 1.1, (h2, 1)).astype(np.float32)

    if dark:
        base_r = 16 + 18 * pearl + streak
        base_g = 20 + 21 * pearl + streak
        base_b = 25 + 24 * pearl + streak
    else:
        base_r = 204 + 44 * pearl + streak
        base_g = 211 + 41 * pearl + streak
        base_b = 219 + 34 * pearl + streak

    halo = np.exp(-(((u - 0.52) / 0.58) ** 2 + ((v - 0.06) / 0.24) ** 2))
    ring_dist = np.sqrt((u - 0.52) ** 2 + ((v - 0.42) * 1.45) ** 2)
    rings = (
        np.exp(-((ring_dist - 0.24) / 0.008) ** 2)
        + 0.7 * np.exp(-((ring_dist - 0.42) / 0.009) ** 2)
        + 0.45 * np.exp(-((ring_dist - 0.60) / 0.011) ** 2)
    )

    if dark:
        r = np.clip(base_r + 14 * halo + 10 * rings, 0, 255)
        g = np.clip(base_g + 42 * halo + 14 * rings, 0, 255)
        b = np.clip(base_b + 8 * halo + 12 * rings, 0, 255)
    else:
        r = np.clip(base_r - 18 * halo + 10 * rings, 0, 255)
        g = np.clip(base_g + 22 * halo + 12 * rings, 0, 255)
        b = np.clip(base_b - 26 * halo + 14 * rings, 0, 255)

    # Stacked 2005 Blade edge slivers along the left (Xbox Live Orange) & right (Media Blue / System Violet)
    s_arc = np.clip(np.sin(np.pi * np.clip(v, 0.0, 1.0)), 0.0, 1.0)
    curve_x_l = 4.0 + 7.5 * (1.0 - s_arc ** 1.15)
    in_left_blade = xx < curve_x_l
    left_rim = (xx >= curve_x_l - 1.5) & (xx <= curve_x_l + 2.0)

    curve_x_r = (w2 - 1.0) - curve_x_l
    in_right_blade = xx > curve_x_r
    right_rim = (xx >= curve_x_r - 2.0) & (xx <= curve_x_r + 1.5)

    l_scale = 0.78 if dark else 1.0
    r = np.where(in_left_blade, np.clip((236 - 36 * v) * l_scale, 0, 255), r)
    g = np.where(in_left_blade, np.clip((144 - 30 * v) * l_scale, 0, 255), g)
    b = np.where(in_left_blade, np.clip((40 + 12 * v) * l_scale, 0, 255), b)

    rb_r = (48 if dark else 56) + 78 * np.clip((v - 0.45) / 0.45, 0, 1)
    rb_g = (118 if dark else 132) - 52 * np.clip((v - 0.45) / 0.45, 0, 1)
    rb_b = (188 if dark else 208) - 16 * np.clip((v - 0.45) / 0.45, 0, 1)
    r = np.where(in_right_blade, rb_r, r)
    g = np.where(in_right_blade, rb_g, g)
    b = np.where(in_right_blade, rb_b, b)

    rail = (84, 94, 106) if dark else (168, 176, 186)
    r = np.where(left_rim | right_rim, rail[0], r)
    g = np.where(left_rim | right_rim, rail[1], g)
    b = np.where(left_rim | right_rim, rail[2], b)

    bg = np.stack([r, g, b, np.full_like(r, 255)], axis=-1).astype(np.uint8)
    img2x = Image.fromarray(bg, "RGBA")
    img2x.save(out / "xbox_backdrop@2x.png", optimize=True)
    img2x.resize((w1, h1), Image.Resampling.LANCZOS).save(out / "xbox_backdrop.png", optimize=True)

    # 5b. 2005 Xenon Chrome vs Smoked-Gunmetal Guide Sphere (56x56)
    if dark:
        ring_stops = ('#7a8896', '#3c4550', '#161b21', '#5c6874')
        ring_stroke = '#2b343e'
        recess_fill = '#090c0f'
        sphere_stops = ('#5c6672', '#2f3740', '#181e24', '#0d1115')
        sphere_stroke = '#4e5965'
        gloss_op = '0.36'
    else:
        ring_stops = ('#ffffff', '#d2dae2', '#76828e', '#e5ebf0')
        ring_stroke = '#5b6672'
        recess_fill = '#1e262e'
        sphere_stops = ('#ffffff', '#dde4eb', '#9ca6b2', '#5d6874')
        sphere_stroke = '#4c5763'
        gloss_op = '0.78'

    guide_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="56" height="56" viewBox="0 0 56 56">
      <defs>
        <linearGradient id="chromeRing" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="{ring_stops[0]}"/>
          <stop offset="0.35" stop-color="{ring_stops[1]}"/>
          <stop offset="0.68" stop-color="{ring_stops[2]}"/>
          <stop offset="1" stop-color="{ring_stops[3]}"/>
        </linearGradient>
        <radialGradient id="silverSphere" cx="0.48" cy="0.34" r="0.68">
          <stop offset="0" stop-color="{sphere_stops[0]}"/>
          <stop offset="0.46" stop-color="{sphere_stops[1]}"/>
          <stop offset="0.82" stop-color="{sphere_stops[2]}"/>
          <stop offset="1" stop-color="{sphere_stops[3]}"/>
        </radialGradient>
        <linearGradient id="xGreen" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#a2f852"/>
          <stop offset="0.5" stop-color="#5ec228"/>
          <stop offset="1" stop-color="#2e7b12"/>
        </linearGradient>
        <linearGradient id="domeGloss" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#ffffff" stop-opacity="{gloss_op}"/>
          <stop offset="1" stop-color="#ffffff" stop-opacity="0.0"/>
        </linearGradient>
      </defs>
      <circle cx="28" cy="29.2" r="25.5" fill="#000000" fill-opacity="0.28"/>
      <circle cx="28" cy="28" r="25.2" fill="url(#chromeRing)" stroke="{ring_stroke}" stroke-width="0.9"/>
      <circle cx="28" cy="28" r="22.8" fill="{recess_fill}" stroke="#343e48" stroke-width="0.7"/>
      <circle cx="28" cy="28" r="18.6" fill="url(#silverSphere)" stroke="{sphere_stroke}" stroke-width="0.9"/>
      <mask id="xSphereMask">
        <circle cx="28" cy="28" r="18.2" fill="white"/>
      </mask>
      <g mask="url(#xSphereMask)">
        <path d="M13,12 C21,18 24,23 28,27.5 C32,23 35,18 43,12 L46,15 C38,22 35,26 31,29.5 C36,36 40,42 45,48 L39,49 C34,42 31,37 28,32.5 C25,37 22,42 17,49 L11,48 C16,42 20,36 25,29.5 C21,26 18,22 10,15 Z"
              fill="#193a0b" fill-opacity="0.55" transform="translate(0, 0.8)"/>
        <path d="M13,12 C21,18 24,23 28,27.5 C32,23 35,18 43,12 L46,15 C38,22 35,26 31,29.5 C36,36 40,42 45,48 L39,49 C34,42 31,37 28,32.5 C25,37 22,42 17,49 L11,48 C16,42 20,36 25,29.5 C21,26 18,22 10,15 Z"
              fill="url(#xGreen)" stroke="#e6ffc2" stroke-opacity="0.65" stroke-width="0.55"/>
        <ellipse cx="28" cy="17.5" rx="12.5" ry="6.2" fill="url(#domeGloss)"/>
      </g>
    </svg>'''
    render_svg(guide_svg, out / "xbox_guide_orb", 56, 56)

    # 5c. Xbox 360 Ring of Light LED Quadrant overlays (56x56)
    quad_paths = {
        1: "M 7.8 26.2 A 20.6 20.6 0 0 1 26.2 7.8",
        2: "M 29.8 7.8 A 20.6 20.6 0 0 1 48.2 26.2",
        3: "M 48.2 29.8 A 20.6 20.6 0 0 1 29.8 48.2",
        4: "M 26.2 48.2 A 20.6 20.6 0 0 1 7.8 29.8",
    }
    rol_specs = {
        "xbox_rol_live": ([1], "#7ef038"),
        "xbox_rol_manual": ([1, 2], "#7ef038"),
        "xbox_rol_paused": ([1, 2, 3, 4], "#f59d1a"),
        "xbox_rol_offline": ([1, 3, 4], "#e53935"),
    }
    rol_off = "#1f2730" if dark else "#36404a"
    for name, (qs, col) in rol_specs.items():
        arcs = []
        for q in range(1, 5):
            if q in qs:
                arcs.append(f'<path d="{quad_paths[q]}" fill="none" stroke="{col}" stroke-width="3.0" stroke-linecap="round" filter="url(#rolBlur)"/>')
                arcs.append(f'<path d="{quad_paths[q]}" fill="none" stroke="#ffffff" stroke-opacity="0.82" stroke-width="1.05" stroke-linecap="round"/>')
            else:
                arcs.append(f'<path d="{quad_paths[q]}" fill="none" stroke="{rol_off}" stroke-opacity="0.88" stroke-width="2.0" stroke-linecap="round"/>')
        rol_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="56" height="56" viewBox="0 0 56 56">
          <defs><filter id="rolBlur" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="1.1" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>
          {''.join(arcs)}
        </svg>'''
        render_svg(rol_svg, out / name, 56, 56)

    # 5d. Authentic 2005 Blades Tabs (Pearl-Silver Light vs Obsidian Carbon Dark) (104x36)
    blade_table = (
        [
            ("xbox_blade_active", "#36414d", "#212830", "#161b22", "#74d834", "#9ef652", "#47a31d", "0.24"),
            ("xbox_blade_idle", "#272f38", "#1c2229", "#13181d", "#485462", "#596674", "#36404a", "0.14"),
        ]
        if dark
        else [
            ("xbox_blade_active", "#ffffff", "#eef3f7", "#d9e1e9", "#4ba822", "#8cf04a", "#3c9418", "0.72"),
            ("xbox_blade_idle", "#e7ecf1", "#cfd7e0", "#b6c0cb", "#85919e", "#aeb8c4", "#75808d", "0.72"),
        ]
    )
    for name, body_top, body_mid, body_bot, rim, spine_top, spine_bot, gloss_alpha in blade_table:
        blade_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="104" height="36" viewBox="0 0 104 36">
          <defs>
            <linearGradient id="bl" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{body_top}"/>
              <stop offset="0.52" stop-color="{body_mid}"/>
              <stop offset="1" stop-color="{body_bot}"/>
            </linearGradient>
            <linearGradient id="sp" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{spine_top}"/>
              <stop offset="1" stop-color="{spine_bot}"/>
            </linearGradient>
            <linearGradient id="blGloss" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="#ffffff" stop-opacity="{gloss_alpha}"/>
              <stop offset="1" stop-color="#ffffff" stop-opacity="0.0"/>
            </linearGradient>
          </defs>
          <path d="M12,4 L92,4 Q98,4 102,19 Q98,34 92,34 L12,34 Q6,34 2,19 Q6,4 12,4 Z"
                fill="#000000" fill-opacity="0.22"/>
          <path d="M12,3 L92,3 Q98,3 102,18 Q98,33 92,33 L12,33 Q6,33 2,18 Q6,3 12,3 Z"
                fill="url(#bl)" stroke="{rim}" stroke-width="1.6"/>
          <path d="M12,3.5 L18,3.5 Q12,18 18,32.5 L12,32.5 Q6.5,32.5 2.6,18 Q6.5,3.5 12,3.5 Z"
                fill="url(#sp)"/>
          <path d="M13,5 L91,5 Q95,5 98,15 L6,15 Q9,5 13,5 Z"
                fill="url(#blGloss)"/>
        </svg>'''
        render_svg(blade_svg, out / name, 104, 36)

    # 5e. Brushed Metallic Blade Card (88x64)
    if dark:
        card_stops = ('#2c343e', '#232a33', '#1c2229', '#161b21')
        edge_stops = ('#52606e', '#323c46')
        card_shine = '0.16'
        card_shadow = '0.28'
    else:
        card_stops = ('#ffffff', '#f3f6f9', '#e5ebf1', '#d7dfe8')
        edge_stops = ('#a5b0bc', '#7c8896')
        card_shine = '0.85'
        card_shadow = '0.12'
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
    render_svg(card_svg, out / "xbox_card", 88, 64)

    # 5f. Blades Selector Pill Buttons (72x30)
    pill_table = (
        [
            ("xbox_pill_idle", "#353f4a", "#252c35", "#1b2128", "#505d6b", "1.3", "0.14"),
            ("xbox_pill_called", "#283f22", "#1e3219", "#152412", "#6ec836", "1.6", "0.16"),
            ("xbox_pill_active", "#6ed434", "#4ba81c", "#2d7810", "#8cf04a", "1.4", "0.34"),
        ]
        if dark
        else [
            ("xbox_pill_idle", "#fbfcfd", "#e4e9ef", "#cfd7e0", "#86929f", "1.3", "0.75"),
            ("xbox_pill_called", "#edf8e4", "#d2edc0", "#b8df9e", "#52b324", "1.6", "0.68"),
            ("xbox_pill_active", "#58be28", "#3e9b18", "#2d7810", "#296910", "1.4", "0.42"),
        ]
    )
    for name, c0, c1, c2, rim, rw, hi in pill_table:
        pill_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="72" height="30" viewBox="0 0 72 30">
          <defs>
            <linearGradient id="xp" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{c0}"/>
              <stop offset="0.52" stop-color="{c1}"/>
              <stop offset="1" stop-color="{c2}"/>
            </linearGradient>
          </defs>
          <rect x="1.5" y="2.2" width="69" height="26.5" rx="13.2" fill="#000000" fill-opacity="0.18"/>
          <rect x="1.5" y="1.2" width="69" height="27" rx="13.5" fill="url(#xp)" stroke="{rim}" stroke-width="{rw}"/>
          <rect x="4" y="2.5" width="64" height="10" rx="5" fill="#ffffff" fill-opacity="{hi}"/>
        </svg>'''
        render_svg(pill_svg, out / name, 72, 30)

    # 5f2. Action Sphere Button (40x40) for RefreshControl
    for name, active in [("xbox_orb_btn", False), ("xbox_orb_btn_active", True)]:
        if active:
            c0, c1, rim, hi, inner_col, inner_op = ("#74dc38", "#2d7810", "#8ef250" if dark else "#25610d", "0.38" if dark else "0.48", "#a9f272", 0.78)
        elif dark:
            c0, c1, rim, hi, inner_col, inner_op = ("#3c4652", "#1c2229", "#556372", "0.18", "#80c342", 0.55)
        else:
            c0, c1, rim, hi, inner_col, inner_op = ("#ffffff", "#ced7e0", "#7e8a98", "0.78", "#52b324", 0.45)
        xorb_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">
          <defs>
            <linearGradient id="xo" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="{c0}"/>
              <stop offset="1" stop-color="{c1}"/>
            </linearGradient>
          </defs>
          <circle cx="20" cy="21" r="16.5" fill="#000000" fill-opacity="0.22"/>
          <circle cx="20" cy="20" r="16.5" fill="url(#xo)" stroke="{rim}" stroke-width="1.4"/>
          <circle cx="20" cy="20" r="14.2" fill="none" stroke="{inner_col}" stroke-opacity="{inner_op}" stroke-width="1"/>
          <ellipse cx="20" cy="12" rx="10" ry="4.5" fill="#ffffff" fill-opacity="{hi}"/>
        </svg>'''
        render_svg(xorb_svg, out / name, 40, 40)

    # 5g. Chrome vs Dark-Gunmetal & Emerald Slider Thumb (24x24)
    if dark:
        t_stops = ("#566270", "#2d353e", "#161b21")
        t_rim, t_core = "#7ce842", "#90f452"
    else:
        t_stops = ("#ffffff", "#cfd8e2", "#7c8896")
        t_rim, t_core = "#3e9418", "#46a21e"
    x_thumb = f'''<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
      <defs>
        <radialGradient id="xt" cx="0.45" cy="0.32" r="0.65">
          <stop offset="0" stop-color="{t_stops[0]}"/>
          <stop offset="0.65" stop-color="{t_stops[1]}"/>
          <stop offset="1" stop-color="{t_stops[2]}"/>
        </radialGradient>
      </defs>
      <circle cx="12" cy="12.6" r="9.0" fill="#000000" fill-opacity="0.26"/>
      <circle cx="12" cy="12" r="8.4" fill="url(#xt)" stroke="{t_rim}" stroke-width="1.8"/>
      <circle cx="12" cy="12" r="3.4" fill="{t_core}"/>
    </svg>'''
    render_svg(x_thumb, out / "xbox_slider_thumb", 24, 24)

    # 5h. Glossy Xbox 360 Jelly Weather Icons (Light vs Dark Card Contrast) (64x64 -> 1x/2x/3x)
    cloud_stops = ("#f6f9fc", "#cdd8e4", "#8b9bb0") if dark else ("#ffffff", "#d8e1eb", "#96a4b4")
    cloud_stroke = "#3d4854" if dark else "#52606e"
    mist_stroke = "#9bb0c4" if dark else "#5e7082"
    q_fill = "#88f048" if dark else "#2d7810"
    star_fill = "#88f048" if dark else "#52b324"
    flake_stroke = "#72b8f5" if dark else "#4478a8"

    x_defs = f'''<defs>
      <radialGradient id="xSunCore" cx="0.4" cy="0.35" r="0.65"><stop offset="0" stop-color="#fff4a3"/><stop offset="0.55" stop-color="#f5a81c"/><stop offset="1" stop-color="#c45c04"/></radialGradient>
      <radialGradient id="xSunHalo"><stop offset="0.35" stop-color="#f5a81c" stop-opacity="0.34"/><stop offset="1" stop-color="#f5a81c" stop-opacity="0"/></radialGradient>
      <linearGradient id="xCloud" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{cloud_stops[0]}"/><stop offset="0.58" stop-color="{cloud_stops[1]}"/><stop offset="1" stop-color="{cloud_stops[2]}"/></linearGradient>
      <linearGradient id="xDrop" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#7ef244"/><stop offset="1" stop-color="#358e18"/></linearGradient>
      <linearGradient id="xMoon" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff099"/><stop offset="0.55" stop-color="#ecc030"/><stop offset="1" stop-color="#b87e12"/></linearGradient>
    </defs>'''

    def x_sun(cx: float, cy: float, r: float, faint: bool = False) -> str:
        op = 'opacity="0.62"' if faint else ""
        rays = "".join(
            f'<line x1="{cx + r * 1.32 * math.cos(a):.2f}" y1="{cy + r * 1.32 * math.sin(a):.2f}" '
            f'x2="{cx + r * 1.84 * math.cos(a):.2f}" y2="{cy + r * 1.84 * math.sin(a):.2f}" '
            f'stroke="#f09218" stroke-width="2.6" stroke-linecap="round"/>'
            for a in [k * math.pi / 4 for k in range(8)]
        )
        return f'<g {op}><circle cx="{cx}" cy="{cy}" r="{r * 1.95}" fill="url(#xSunHalo)"/>{rays}<circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#xSunCore)" stroke="#a65206" stroke-width="1.0"/></g>'

    def x_moon(cx: float, cy: float, r: float) -> str:
        return f'''<mask id="xm_{int(cx)}">
          <rect width="64" height="64" fill="black"/>
          <circle cx="{cx}" cy="{cy}" r="{r}" fill="white"/>
          <circle cx="{cx + r * 0.45}" cy="{cy - r * 0.35}" r="{r * 0.85}" fill="black"/>
        </mask>
        <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#xMoon)" stroke="#7c560a" stroke-width="1.1" mask="url(#xm_{int(cx)})"/>'''

    def x_cloud(cx: float, cy: float, s: float) -> str:
        return f'''<g transform="translate({cx},{cy}) scale({s})">
          <g fill="#000000" fill-opacity="0.22" transform="translate(0, 1.5)">
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
          <path d="M-7,-6 A9,9 0 0,1 8,-7" fill="none" stroke="#ffffff" stroke-width="1.4" stroke-linecap="round"/>
        </g>'''

    def x_drop(x: float, y: float) -> str:
        return f'<line x1="{x}" y1="{y}" x2="{x - 2}" y2="{y + 8}" stroke="url(#xDrop)" stroke-width="2.6" stroke-linecap="round"/>'

    def x_flake(cx: float, cy: float, r: float) -> str:
        lines = "".join(
            f'<line x1="{cx}" y1="{cy}" x2="{cx + r * math.cos(a):.2f}" y2="{cy + r * math.sin(a):.2f}"/>'
            for a in [k * math.pi / 3 - math.pi / 2 for k in range(6)]
        )
        return f'<g stroke="{flake_stroke}" stroke-width="2" stroke-linecap="round">{lines}</g>'

    x_cases = {
        "clear_day": x_sun(32, 32, 12.5),
        "clear_night": x_moon(28, 33, 13) + f'<path d="M47 13 L48.5 16.5 L52 18 L48.5 19.5 L47 23 L45.5 19.5 L42 18 L45.5 16.5 Z" fill="{star_fill}" stroke="#285c12" stroke-width="0.7"/><circle cx="51" cy="33" r="2.2" fill="#e89a1c"/>',
        "partly_cloudy_day": x_sun(22, 22, 9.8) + x_cloud(34, 38, 0.94),
        "partly_cloudy_night": x_moon(21, 22, 9.8) + f'<circle cx="49" cy="14" r="2.2" fill="{star_fill}"/>' + x_cloud(34, 38, 0.94),
        "cloudy": x_cloud(38, 25, 0.72) + x_cloud(29, 38, 0.98),
        "drizzle": x_cloud(32, 25, 0.95) + x_drop(22, 42) + x_drop(33, 45) + x_drop(44, 42),
        "rain": x_cloud(32, 23, 0.96) + x_drop(20, 41) + x_drop(30, 44) + x_drop(40, 41) + x_drop(48, 44),
        "heavy_rain": x_cloud(32, 22, 0.98) + x_drop(16, 40) + x_drop(25, 43) + x_drop(34, 40) + x_drop(43, 43) + x_drop(51, 40),
        "thunderstorm": x_cloud(32, 22, 0.98) + '<polygon points="35,34 44,34 36,44 43,44 29,59 33,47 27,47" fill="#f5a81c" stroke="#965004" stroke-width="0.9"/>' + x_drop(18, 43) + x_drop(50, 43),
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
