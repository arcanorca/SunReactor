.pragma library

/*
 * Vector glyph sets for emissive readouts, as SVG path strings for
 * QtQuick.Shapes (PathSvg). No font files: the shapes are part of the design.
 *
 *   segment(text, h)  seven-segment VFD digits, slanted, hexagonal segments;
 *                     returns { lit, ghost, width } - `ghost` holds every
 *                     segment of every cell (the unlit phosphor you can see
 *                     on a real VFD).
 *   nixie(text, h)    cold-cathode tube numerals drawn as centre-line strokes;
 *                     returns { cells: [{ x, w, lit, ghost }], width } with one
 *                     entry per tube, `ghost` = the stacked unlit cathodes.
 *
 * Supported characters: 0-9, '-', ' ', ':', '.', '°'. Anything else renders
 * as a blank cell, so unknown stays visibly unknown ("--").
 */

// --- seven segments ----------------------------------------------------------

var SEGMENTS = {
    "0": "abcdef", "1": "bc", "2": "abged", "3": "abgcd", "4": "fgbc",
    "5": "afgcd", "6": "afgedc", "7": "abc", "8": "abcdefg", "9": "abcdfg",
    "-": "g", " ": "", "°": "abfg"
};

function fmt(v) {
    return (Math.round(v * 100) / 100).toString();
}

function poly(points, slant, h, ox) {
    var out = "";
    for (var i = 0; i < points.length; ++i) {
        var x = ox + points[i][0] + (h - points[i][1]) * slant;
        out += (i === 0 ? "M" : "L") + fmt(x) + " " + fmt(points[i][1]) + " ";
    }
    return out + "Z ";
}

function hseg(x1, x2, yc, t) {
    var k = t / 2;
    return [[x1, yc], [x1 + k, yc - k], [x2 - k, yc - k], [x2, yc], [x2 - k, yc + k], [x1 + k, yc + k]];
}

function vseg(xc, y1, y2, t) {
    var k = t / 2;
    return [[xc, y1], [xc + k, y1 + k], [xc + k, y2 - k], [xc, y2], [xc - k, y2 - k], [xc - k, y1 + k]];
}

/*! Width of one digit cell and the advance between cells, for height h. */
function segmentMetrics(h) {
    return { cell: h * 0.52, advance: h * 0.52 + h * 0.16, narrow: h * 0.22, slant: 0.09 };
}

function segment(text, h) {
    var m = segmentMetrics(h);
    var w = m.cell;
    var t = h * 0.125;
    var g = h * 0.022;
    var L = t / 2, R = w - t / 2, T = t / 2, M = h / 2, B = h - t / 2;
    var shapes = {
        a: hseg(L + g, R - g, T, t),
        g: hseg(L + g, R - g, M, t),
        d: hseg(L + g, R - g, B, t),
        f: vseg(L, T + g, M - g, t),
        b: vseg(R, T + g, M - g, t),
        e: vseg(L, M + g, B - g, t),
        c: vseg(R, M + g, B - g, t)
    };
    var lit = "", ghost = "", x = 0;
    var chars = String(text).split("");
    for (var i = 0; i < chars.length; ++i) {
        var ch = chars[i];
        if (ch === ":" || ch === ".") {
            var s = t * 0.95;
            var cx = x + m.narrow / 2 - s / 2;
            var dots = ch === ":" ? [h * 0.32, h * 0.72] : [h - s];
            for (var d = 0; d < dots.length; ++d) {
                lit += poly([[cx, dots[d]], [cx + s, dots[d]], [cx + s, dots[d] + s], [cx, dots[d] + s]],
                            m.slant, h, 0);
            }
            x += m.narrow;
            continue;
        }
        var on = SEGMENTS.hasOwnProperty(ch) ? SEGMENTS[ch] : "";
        var names = "abcdefg";
        for (var n = 0; n < names.length; ++n) {
            var p = poly(shapes[names[n]], m.slant, h, x);
            ghost += p;
            if (on.indexOf(names[n]) >= 0) {
                lit += p;
            }
        }
        x += m.advance;
    }
    var width = chars.length > 0 ? x - (m.advance - m.cell) + h * m.slant : 0;
    return { lit: lit, ghost: ghost, width: width };
}

// --- nixie numerals ----------------------------------------------------------
// Centre lines in a 60 x 100 box (stroke added by the renderer); only M, L, C
// and Z so every number is an x,y pair and the path scales by plain mapping.

var NIXIE = {
    "0": "M30 5 C47 5 55 24 55 50 C55 76 47 95 30 95 C13 95 5 76 5 50 C5 24 13 5 30 5 Z",
    "1": "M22 17 L33 5 L33 95",
    "2": "M7 25 C7 12 17 5 30 5 C44 5 53 13 53 26 C53 42 38 54 7 95 L55 95",
    "3": "M8 7 L52 7 L28 41 C45 40 55 51 55 67 C55 84 44 95 29 95 C18 95 9 90 5 81",
    "4": "M42 95 L42 5 L5 70 L58 70",
    "5": "M52 6 L13 6 L9 44 C15 40 22 38 30 38 C46 38 55 50 55 66 C55 84 44 95 28 95 C17 95 9 90 5 82",
    "6": "M47 9 C42 6 36 5 31 5 C15 5 6 24 6 55 C6 81 15 95 31 95 C46 95 55 84 55 68 C55 52 46 41 31 41 C18 41 9 49 6 59",
    "7": "M5 6 L55 6 L21 95",
    "8": "M30 46 C17 46 9 38 9 25 C9 13 18 5 30 5 C42 5 51 13 51 25 C51 38 43 46 30 46 C15 46 5 56 5 71 C5 85 16 95 30 95 C44 95 55 85 55 71 C55 56 45 46 30 46 Z",
    "9": "M13 91 C18 94 24 95 29 95 C45 95 54 76 54 45 C54 19 45 5 29 5 C14 5 5 16 5 32 C5 48 14 59 29 59 C42 59 51 51 54 41",
    "-": "M15 55 L45 55",
    "°": "M30 8 C38 8 43 13 43 21 C43 29 38 34 30 34 C22 34 17 29 17 21 C17 13 22 8 30 8 Z"
};

function scalePath(path, sx, sy, ox, oy) {
    var tokens = path.split(/\s+/);
    var out = [];
    var expectX = true;
    for (var i = 0; i < tokens.length; ++i) {
        var tok = tokens[i];
        if (tok.length === 0) {
            continue;
        }
        var head = tok.charAt(0);
        if (head === "M" || head === "L" || head === "C" || head === "Z") {
            out.push(head);
            tok = tok.slice(1);
            expectX = true;
            if (tok.length === 0) {
                continue;
            }
        }
        var v = parseFloat(tok);
        out.push(fmt(expectX ? ox + v * sx : oy + v * sy));
        expectX = !expectX;
    }
    return out.join(" ") + " ";
}

/*! Tube metrics for numeral height h (the glass is a little larger). */
function nixieMetrics(h) {
    return { cell: h * 0.62, tube: h * 0.86, gap: h * 0.08, inset: h * 0.12, narrow: h * 0.3 };
}

function nixie(text, h) {
    var m = nixieMetrics(h);
    var sx = m.cell / 60, sy = h / 100;
    var cells = [];
    var x = 0;
    var chars = String(text).split("");
    var digits = "0123456789";
    for (var i = 0; i < chars.length; ++i) {
        var ch = chars[i];
        if (ch === ":" || ch === ".") {
            // Neon dot lamps between tubes.
            var r = h * 0.045;
            var cx = x + m.narrow / 2;
            var ys = ch === ":" ? [h * 0.36 + m.inset, h * 0.7 + m.inset] : [h * 0.94 + m.inset];
            var dots = "";
            for (var d = 0; d < ys.length; ++d) {
                dots += "M" + fmt(cx - r) + " " + fmt(ys[d]) + " C" + fmt(cx - r) + " " + fmt(ys[d] - r * 1.33) + " "
                    + fmt(cx + r) + " " + fmt(ys[d] - r * 1.33) + " " + fmt(cx + r) + " " + fmt(ys[d]) + " C"
                    + fmt(cx + r) + " " + fmt(ys[d] + r * 1.33) + " " + fmt(cx - r) + " " + fmt(ys[d] + r * 1.33)
                    + " " + fmt(cx - r) + " " + fmt(ys[d]) + " Z ";
            }
            cells.push({ x: x, w: m.narrow, lit: dots, ghost: "", lamp: true });
            x += m.narrow + m.gap;
            continue;
        }
        var ox = x + (m.tube - m.cell) / 2;
        var oy = m.inset;
        var ghost = "";
        for (var k = 0; k < digits.length; ++k) {
            if (digits[k] !== ch) {
                ghost += scalePath(NIXIE[digits[k]], sx, sy, ox, oy);
            }
        }
        cells.push({
            x: x,
            w: m.tube,
            lit: NIXIE.hasOwnProperty(ch) ? scalePath(NIXIE[ch], sx, sy, ox, oy) : "",
            ghost: ghost,
            lamp: false
        });
        x += m.tube + m.gap;
    }
    return { cells: cells, width: chars.length > 0 ? x - m.gap : 0, height: h + m.inset * 2 };
}
