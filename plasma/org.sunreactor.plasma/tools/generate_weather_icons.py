#!/usr/bin/env python3
"""Weather sprites for the panel and the popup, as 16x16 pixel art.

DESIGN LANGUAGE
---------------
Every sprite in this family follows the same rules, so a row of them reads as
one set rather than thirteen drawings:

1. Grid. 16x16, drawn at whole multiples of that grid in the UI (16, 32, 48).
   Nothing touches the outermost ring except sun rays.
2. Light. One source, upper left. Highlight on the upper left edge, core in
   the middle, shade on the lower right, and the darkest tone only as the
   grounding shadow along the bottom. No black outlines anywhere.
3. Bodies. Sun and moon share one authored 8x8 disc, so a day sprite and its
   night twin carry exactly the same visual weight. The crescent is that disc
   with a shifted copy of itself removed - never a separate shape.
3b. Crescents open to the right: the disc is carved from the lower right, so
   the lit limb sits where the light comes from.
4. Clouds. One silhouette in two sizes. A cloud never floats: its bottom row
   is the shadow tone, and whatever falls out of it starts one row below.
5. Rhythm. Precipitation runs on a staggered two-row beat, always the same
   columns for the same intensity, so drizzle, rain and heavy rain look like
   the same weather getting worse.
6. Night. The whole palette is dimmed by one fixed factor, never redrawn.
7. Palette. The one the terminal cockpit uses (`src/tui/ui/weather_art.rs`),
   so both surfaces look like the same product.

PIXEL ATLASES
-------------
The pixel skins cannot use these colours: a pixel theme may only show its own
palette and shades (DESIGN.md section 4, rule 4). So the same drawings are
also written as indexed PNG atlases whose pixels are *tone indices*, and the
plugin's PixelSprite recolours index i with the i-th colour a skin passes in
(derived from the theme). Index meaning (TONES below):

    0 transparent      1 highlight   2 light      3 mid        4 shadow
    5 warm core        6 warm edge   7 water      8 outline    9 ray

`outline` is generated: every empty pixel next to a drawn one. Skins on light
"paper" (Game Boy LCD, 1-bit Mac) map it to ink so the silhouette survives;
skins on dark paper leave it transparent.

  icons/pixel/weather16.png   13 frames of 16x16, in ATLAS_ORDER
  icons/pixel/ui8.png         small 8x8 symbols for the pixel skins (UI8_ORDER)

Run:  python3 tools/generate_weather_icons.py   (from the widget directory;
the repository .gitignore hides *.py, this file is tracked with `git add -f`
via the widget's own .gitignore exception for tools/*.py)
"""

from __future__ import annotations

from pathlib import Path

PACKAGE = Path(__file__).resolve().parent.parent / "package" / "contents"
OUT_DIR = PACKAGE / "icons" / "weather"
PIXEL_DIR = PACKAGE / "icons" / "pixel"
SIZE = 16

# --- Palette (shared with the terminal cockpit) ---------------------------

SUN_HIGHLIGHT = "#fff6c4"
SUN_CORE = "#ffd64a"
SUN_SHADE = "#fbb134"
SUN_RIM = "#e8851f"
SUN_RAY = "#ffc84d"

MOON_LIGHT = "#f8f4de"
MOON_CORE = "#f0e9c6"
MOON_SHADE = "#d9d1a8"
MOON_RIM = "#bfb58a"

STAR_BRIGHT = "#fff3b8"
STAR_DIM = "#8e9cc4"

RAIN = "#5da9ef"
RAIN_LIGHT = "#a9daff"
SNOW = "#ffffff"
SNOW_SHADE = "#cfe0f2"
BOLT = "#fff28f"
BOLT_EDGE = "#f2b11a"
FOG = "#c8cfdb"
FOG_SHADE = "#8b94a4"

# Five tones per cloud, lightest first: highlight, light, mid, shadow, deep.
CLOUD_WHITE = ["#ffffff", "#eef2f8", "#d3ddea", "#a8b6c9", "#7b8aa2"]
CLOUD_GRAY = ["#dee3ec", "#c2c9d5", "#a2abba", "#7e8798", "#5a6272"]
CLOUD_STORM = ["#9ca4b4", "#7f8899", "#666e7f", "#4e5566", "#383e4c"]

TONE_KEYS = "HLMSD"
NIGHT_FACTORS = (0.62, 0.66, 0.78)


def night(color: str) -> str:
    """Cools and darkens a colour for the night variants."""
    channels = [int(color[index : index + 2], 16) for index in (1, 3, 5)]
    return "#" + "".join(
        f"{round(value * factor):02x}" for value, factor in zip(channels, NIGHT_FACTORS)
    )


# --- Authored shapes ------------------------------------------------------

# The one disc every sun and moon is built from: 8 across, shaded along the
# light axis. H highlight, C core, S shade, R rim.
DISC_8 = [
    "......HHCC......",
    ".....HHCCCC.....",
    "....HHCCCCCS....",
    "....HCCCCCSS....",
    "....HCCCCCSS....",
    "....CCCCCSSR....",
    ".....CCCSSR.....",
    "......CSSR......",
]
DISC_8_AT = (0, 4)

# The same disc at 6 across, for the sprites where a cloud shares the frame.
DISC_6 = [
    "..CC..",
    ".HCCC.",
    "HCCCCS",
    "HCCCCS",
    ".CCSSR",
    "..CSS.",
]

# Rays: two pixels thick on the cardinals, one on the diagonals, always one
# clear pixel away from the disc.
RAYS_8 = [
    (7, 1), (8, 1), (7, 2), (8, 2),      # north
    (7, 13), (8, 13), (7, 14), (8, 14),  # south
    (1, 7), (1, 8), (2, 7), (2, 8),      # west
    (13, 7), (13, 8), (14, 7), (14, 8),  # east
    (2, 2), (3, 3),                      # north west
    (13, 2), (12, 3),                    # north east
    (2, 13), (3, 12),                    # south west
    (13, 13), (12, 12),                  # south east
]

# The same idea around a 6-wide disc placed at (3, 2): only the free side,
# because a cloud covers the rest.
RAYS_6 = [(6, 0), (7, 0), (0, 4), (0, 5), (1, 1)]

BIG_CLOUD = [
    ".....HHHH.......",
    "...HHLLLLHH.....",
    "..HLLLLLLLLH.HH.",
    ".HLLLLLLLLLLHLLH",
    "HLLLLLLLLLLLLLLM",
    "MMLLLLLLLLLLLLMS",
    ".SSMMMMMMMMMMSS.",
]

MED_CLOUD = [
    "...HHHH....",
    ".HHLLLLHH..",
    "HLLLLLLLLHH",
    "MLLLLLLLLLM",
    ".SMMMMMMMS.",
]

# Light comes from the same corner here too: the leading edge is the bright
# one, the trailing edge takes the darker tone.
BOLT_MASK = [
    "..BBE.",
    ".BBE..",
    "BBBBE.",
    ".BBBE.",
    "..BBE.",
    ".BBE..",
    ".BE...",
]


class Canvas:
    def __init__(self) -> None:
        self.pixels: dict[tuple[int, int], str] = {}

    def set(self, x: int, y: int, color: str) -> None:
        if 0 <= x < SIZE and 0 <= y < SIZE:
            self.pixels[(x, y)] = color

    def stamp(self, mask: list[str], ox: int, oy: int, tones: dict[str, str]) -> None:
        """Draws a character mask; '.' leaves the pixel untouched."""
        for dy, row in enumerate(mask):
            for dx, key in enumerate(row):
                if key != ".":
                    self.set(ox + dx, oy + dy, tones[key])

    def to_svg(self, name: str) -> str:
        """One rect per horizontal run of equal colour."""
        parts = [
            '<?xml version="1.0" encoding="UTF-8"?>',
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {SIZE} {SIZE}"',
            f'     width="{SIZE}" height="{SIZE}" shape-rendering="crispEdges">',
            f"  <title>{name}</title>",
        ]
        for y in range(SIZE):
            x = 0
            while x < SIZE:
                color = self.pixels.get((x, y))
                if color is None:
                    x += 1
                    continue
                run = 1
                while x + run < SIZE and self.pixels.get((x + run, y)) == color:
                    run += 1
                parts.append(
                    f'  <rect x="{x}" y="{y}" width="{run}" height="1" fill="{color}"/>'
                )
                x += run
        parts.append("</svg>")
        return "\n".join(parts) + "\n"


def mask_cells(mask: list[str], ox: int, oy: int) -> set[tuple[int, int]]:
    """Where a mask would draw, without drawing it."""
    return {
        (ox + dx, oy + dy)
        for dy, row in enumerate(mask)
        for dx, key in enumerate(row)
        if key != "."
    }


def cloud_tones(tones: list[str], dim: bool = False) -> dict[str, str]:
    values = [night(tone) for tone in tones] if dim else tones
    return dict(zip(TONE_KEYS, values))


def body_tones(
    highlight: str, core: str, shade: str, rim: str, dim: bool = False
) -> dict[str, str]:
    values = {"H": highlight, "C": core, "S": shade, "R": rim}
    return {key: (night(value) if dim else value) for key, value in values.items()}


def sun_tones(dim: bool = False) -> dict[str, str]:
    return body_tones(SUN_HIGHLIGHT, SUN_CORE, SUN_SHADE, SUN_RIM, dim)


def moon_tones(dim: bool = False) -> dict[str, str]:
    return body_tones(MOON_LIGHT, MOON_CORE, MOON_SHADE, MOON_RIM, dim)


# --- Drawing --------------------------------------------------------------


def draw_sun(
    canvas: Canvas,
    mask: list[str],
    at: tuple[int, int],
    rays: list[tuple[int, int]],
    dim: bool = False,
) -> None:
    ray_color = night(SUN_RAY) if dim else SUN_RAY
    for x, y in rays:
        canvas.set(x, y, ray_color)
    canvas.stamp(mask, at[0], at[1], sun_tones(dim))


def draw_moon(
    canvas: Canvas,
    mask: list[str],
    at: tuple[int, int],
    shift: tuple[int, int],
    dim: bool = False,
) -> None:
    """The crescent: the disc minus a shifted copy of itself."""
    ox, oy = at
    carved = mask_cells(mask, ox + shift[0], oy + shift[1])
    tones = moon_tones(dim)
    for dy, row in enumerate(mask):
        for dx, key in enumerate(row):
            cell = (ox + dx, oy + dy)
            if key == "." or cell in carved:
                continue
            # The inner edge of a crescent is its terminator: keep it dark.
            inner = (cell[0] - shift[0], cell[1] - shift[1]) in carved
            canvas.set(cell[0], cell[1], tones["S" if inner else key])


def draw_stars(canvas: Canvas, points: list[tuple[int, int, bool]]) -> None:
    for x, y, bright in points:
        canvas.set(x, y, STAR_BRIGHT if bright else STAR_DIM)


def draw_drops(canvas: Canvas, columns: list[int], top: int, length: int) -> None:
    """Drops on a staggered beat: every other column starts one row lower."""
    for index, x in enumerate(columns):
        y = top + (index % 2)
        canvas.set(x, y, RAIN_LIGHT)
        for step in range(1, length):
            canvas.set(x, y + step, RAIN)


def draw_flakes(canvas: Canvas, points: list[tuple[int, int]]) -> None:
    """Plus-shaped flakes: a bright centre with four dimmer arms."""
    for x, y in points:
        for dx, dy in ((0, -1), (-1, 0), (1, 0), (0, 1)):
            canvas.set(x + dx, y + dy, SNOW_SHADE)
        canvas.set(x, y, SNOW)


def draw_fog(canvas: Canvas, bars: list[tuple[int, int, int]]) -> None:
    """Bars fade out to the right, like the underside of a cloud."""
    for x, y, width in bars:
        for step in range(width):
            canvas.set(x + step, y, FOG if step < width - 2 else FOG_SHADE)


# --- Sprites --------------------------------------------------------------

# Shared placement: a cloud with something falling out of it sits high, a
# cloud with nothing under it sits in the middle of the frame.
CLOUD_HIGH = 1
CLOUD_MIDDLE = 5
PRECIPITATION_TOP = 10


def clear(night_variant: bool) -> Canvas:
    canvas = Canvas()
    if night_variant:
        draw_stars(canvas, [(2, 3, True), (13, 4, False), (3, 12, False), (13, 12, True)])
        draw_moon(canvas, DISC_8, DISC_8_AT, (4, -1))
    else:
        draw_sun(canvas, DISC_8, DISC_8_AT, RAYS_8)
    return canvas


def partly_cloudy(night_variant: bool) -> Canvas:
    canvas = Canvas()
    if night_variant:
        draw_stars(canvas, [(13, 1, False), (11, 4, True)])
        draw_moon(canvas, DISC_6, (3, 2), (3, -1))
    else:
        draw_sun(canvas, DISC_6, (3, 2), RAYS_6)
    canvas.stamp(BIG_CLOUD, 0, 8, cloud_tones(CLOUD_WHITE, dim=night_variant))
    return canvas


def cloudy() -> Canvas:
    canvas = Canvas()
    # Two layers: a pale one behind, the weather one in front.
    canvas.stamp(MED_CLOUD, 2, 1, cloud_tones(CLOUD_WHITE))
    canvas.stamp(BIG_CLOUD, 0, CLOUD_MIDDLE, cloud_tones(CLOUD_GRAY))
    return canvas


def precipitation(
    tones: list[str], columns: list[int], length: int, second_beat: bool = False
) -> Canvas:
    canvas = Canvas()
    canvas.stamp(BIG_CLOUD, 0, CLOUD_HIGH, cloud_tones(tones))
    draw_drops(canvas, columns, PRECIPITATION_TOP, length)
    if second_beat:
        draw_drops(canvas, [column + 2 for column in columns[:-1]], PRECIPITATION_TOP, 2)
    return canvas


def thunderstorm() -> Canvas:
    canvas = Canvas()
    canvas.stamp(BIG_CLOUD, 0, 0, cloud_tones(CLOUD_STORM))
    draw_drops(canvas, [2, 13], PRECIPITATION_TOP - 1, 3)
    canvas.stamp(BOLT_MASK, 6, 8, {"B": BOLT, "E": BOLT_EDGE})
    return canvas


def snow() -> Canvas:
    canvas = Canvas()
    canvas.stamp(BIG_CLOUD, 0, CLOUD_HIGH, cloud_tones(CLOUD_WHITE))
    draw_flakes(canvas, [(3, 11), (8, 13), (13, 11)])
    return canvas


def mist() -> Canvas:
    canvas = Canvas()
    canvas.stamp(MED_CLOUD, 2, 1, cloud_tones(CLOUD_WHITE))
    draw_fog(canvas, [(2, 9, 11), (1, 11, 13), (3, 13, 10)])
    return canvas


def fog() -> Canvas:
    canvas = Canvas()
    draw_fog(canvas, [(2, 3, 12), (1, 6, 14), (3, 9, 11), (1, 12, 13)])
    return canvas


def unknown() -> Canvas:
    canvas = Canvas()
    canvas.stamp(BIG_CLOUD, 0, 3, cloud_tones(CLOUD_GRAY))
    # The beat of the rain sprites, drawn as absence.
    for x in (4, 8, 12):
        canvas.set(x, 12, FOG_SHADE)
        canvas.set(x + 1, 12, FOG_SHADE)
    return canvas


SPRITES = {
    "clear-day": lambda: clear(False),
    "clear-night": lambda: clear(True),
    "partly-cloudy-day": lambda: partly_cloudy(False),
    "partly-cloudy-night": lambda: partly_cloudy(True),
    "cloudy": cloudy,
    "drizzle": lambda: precipitation(CLOUD_WHITE, [4, 8, 12], 2),
    "rain": lambda: precipitation(CLOUD_GRAY, [3, 7, 11], 3),
    "heavy-rain": lambda: precipitation(CLOUD_STORM, [2, 6, 10, 14], 3, second_beat=True),
    "thunderstorm": thunderstorm,
    "snow": snow,
    "mist": mist,
    "fog": fog,
    "unknown": unknown,
}


# --- Indexed atlases for the pixel skins -------------------------------------

# Authored colour -> tone index (see the module docstring).
TONES = {
    SUN_HIGHLIGHT: 1, SUN_CORE: 5, SUN_SHADE: 6, SUN_RIM: 6, SUN_RAY: 9,
    MOON_LIGHT: 1, MOON_CORE: 2, MOON_SHADE: 3, MOON_RIM: 3,
    STAR_BRIGHT: 1, STAR_DIM: 3,
    RAIN: 7, RAIN_LIGHT: 2,
    SNOW: 1, SNOW_SHADE: 2,
    BOLT: 5, BOLT_EDGE: 6,
    FOG: 2, FOG_SHADE: 3,
}
for _tones, _map in ((CLOUD_WHITE, (1, 2, 2, 3, 4)), (CLOUD_GRAY, (2, 2, 3, 3, 4)),
                     (CLOUD_STORM, (3, 3, 4, 4, 4))):
    for _colour, _index in zip(_tones, _map):
        TONES.setdefault(_colour, _index)
# Night variants are the same drawings dimmed: same tones.
for _colour, _index in list(TONES.items()):
    TONES.setdefault(night(_colour), _index)
OUTLINE = 8
TONE_COUNT = 10

# Frame order of weather16.png; skins/pixel/Sprites.js mirrors it.
ATLAS_ORDER = [
    "clear-day", "clear-night", "partly-cloudy-day", "partly-cloudy-night", "cloudy",
    "drizzle", "rain", "heavy-rain", "thunderstorm", "snow", "mist", "fog", "unknown",
]

# 8x8 symbols, authored directly in tones: '.' empty, H L M D = 1..4,
# W warm core, E warm edge, B water, R ray (outline is added like above).
UI8_KEYS = {".": 0, "H": 1, "L": 2, "M": 3, "D": 4, "W": 5, "E": 6, "B": 7, "R": 9}
UI8 = {
    "sun": [
        "R..R..R.",
        ".R...R..",
        "..HWW...",
        "RRWWERR.",
        "..WEE...",
        ".R...R..",
        "R..R..R.",
        "........",
    ],
    "moon": [
        "..HLL...",
        ".HL.....",
        "HL......",
        "HL......",
        "LM......",
        ".LM.....",
        "..MMM...",
        "........",
    ],
    "display": [
        "DDDDDDD.",
        "D.....D.",
        "D.....D.",
        "D.....D.",
        "DDDDDDD.",
        "...D....",
        ".DDDDD..",
        "........",
    ],
    "pause": [
        "........",
        ".DD.DD..",
        ".DD.DD..",
        ".DD.DD..",
        ".DD.DD..",
        ".DD.DD..",
        "........",
        "........",
    ],
    "clock": [
        "..DDD...",
        ".D...D..",
        "D..D..D.",
        "D..DD.D.",
        "D.....D.",
        ".D...D..",
        "..DDD...",
        "........",
    ],
    "hand": [
        "...D....",
        "...D....",
        "...DDD..",
        ".D.DDDD.",
        ".DDDDDD.",
        "..DDDDD.",
        "...DDD..",
        "........",
    ],
    "drop": [
        "...B....",
        "...B....",
        "..BBB...",
        ".BBBBB..",
        ".BHBBB..",
        ".BBBBB..",
        "..BBB...",
        "........",
    ],
    "rise": [
        "...R....",
        "..RRR...",
        "...R....",
        "........",
        "..WWW...",
        ".WWWWW..",
        "DDDDDDD.",
        "........",
    ],
    "set": [
        "..RRR...",
        "...R....",
        "........",
        "........",
        "..WWW...",
        ".WWWWW..",
        "DDDDDDD.",
        "........",
    ],
    "noon": [
        "...R....",
        ".R.W.R..",
        "..WWW...",
        "RWWWWWR.",
        "..WWW...",
        ".R.W.R..",
        "...R....",
        "........",
    ],
    "lock": [
        "..DDD...",
        ".D...D..",
        ".D...D..",
        "DDDDDDD.",
        "DDD.DDD.",
        "DDD.DDD.",
        "DDDDDDD.",
        "........",
    ],
}
UI8_ORDER = ["sun", "moon", "display", "pause", "clock", "hand", "drop", "rise", "set", "noon", "lock"]


def indexed_frame(canvas: Canvas) -> list[list[int]]:
    frame = [[0] * SIZE for _ in range(SIZE)]
    for (x, y), colour in canvas.pixels.items():
        if colour not in TONES:
            raise SystemExit(f"colour {colour} has no tone index")
        frame[y][x] = TONES[colour]
    return frame


def with_outline(frame: list[list[int]]) -> list[list[int]]:
    h, w = len(frame), len(frame[0])
    out = [row[:] for row in frame]
    for y in range(h):
        for x in range(w):
            if frame[y][x]:
                continue
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h and frame[ny][nx] not in (0, OUTLINE):
                    out[y][x] = OUTLINE
                    break
    return out


def write_atlas(path: Path, frames: list[list[list[int]]]) -> None:
    from PIL import Image  # only the atlases need Pillow

    fh, fw = len(frames[0]), len(frames[0][0])
    image = Image.new("P", (fw * len(frames), fh), 0)
    # The palette is only a preview for humans; the plugin reads the indices.
    preview = [(0, 0, 0), (255, 255, 255), (200, 200, 200), (130, 130, 130), (60, 60, 60),
               (255, 210, 70), (230, 130, 30), (90, 170, 240), (20, 20, 20), (255, 200, 80)]
    flat = [c for rgb in preview for c in rgb]
    image.putpalette(flat + [0] * (768 - len(flat)))
    for i, frame in enumerate(frames):
        for y, row in enumerate(frame):
            for x, value in enumerate(row):
                image.putpixel((i * fw + x, y), value)
    image.save(path, transparency=0, optimize=False)


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name, build in SPRITES.items():
        (OUT_DIR / f"{name}.svg").write_text(build().to_svg(name))
    print(f"wrote {len(SPRITES)} sprites to {OUT_DIR}")

    PIXEL_DIR.mkdir(parents=True, exist_ok=True)
    weather = [with_outline(indexed_frame(SPRITES[name]())) for name in ATLAS_ORDER]
    write_atlas(PIXEL_DIR / "weather16.png", weather)
    ui8 = [with_outline([[UI8_KEYS[k] for k in row] for row in UI8[name]]) for name in UI8_ORDER]
    write_atlas(PIXEL_DIR / "ui8.png", ui8)
    print(f"wrote weather16.png ({len(weather)} frames) and ui8.png ({len(ui8)} frames) to {PIXEL_DIR}")


if __name__ == "__main__":
    main()
