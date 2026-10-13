.pragma library

/*
 * Geometry and text helpers for the casiodigital skin. Paths are SVG strings in
 * logical px (PathSvg); every vertex snaps to a device pixel (dpr), so the edges
 * stay crisp at 1.0, 1.35 and 2.0. Pure functions, no state, no animation.
 *
 *   seven(text, h, dpr)        slanted seven-segment readout: the lit segments and
 *                              the ghost of every segment (the unlit LCD)
 *   bars(levels, cells, ...)   segmented columns of the day curve, one per slot
 *   numberText(v, cells)       a number in fixed cells, dashes when unknown
 *   clockParts(text)           "14:32" or " 2:32 PM" from a locale time string
 */

// Segments lit per digit. a top, b upper right, c lower right, d bottom,
// e lower left, f upper left, g middle.
var ON = {
    "0": "abcdef", "1": "bc", "2": "abged", "3": "abgcd", "4": "fgbc",
    "5": "afgcd", "6": "afgedc", "7": "abc", "8": "abcdefg", "9": "abcdfg",
    "-": "g", " ": ""
};

function sn(v, dpr) {
    return Math.round(v * dpr) / dpr;
}

/*! Cell geometry for a digit of height h (logical px). */
function metrics(h) {
    return {
        cell: h * 0.56,     // digit width
        gap: h * 0.16,      // space between digits
        colon: h * 0.22,    // width of a colon or a point
        thick: h * 0.13,    // segment thickness
        slant: 0.12         // horizontal lean per unit of height (the top leans right)
    };
}

/*! A closed polygon leaned by the slant and moved to x0, vertices on device px. */
function lean(points, m, h, x0, dpr) {
    var out = "";
    for (var i = 0; i < points.length; ++i) {
        var p = points[i];
        var x = sn(x0 + p[0] + (h - p[1]) * m.slant, dpr);
        out += (i === 0 ? "M" : "L") + x + " " + sn(p[1], dpr) + " ";
    }
    return out + "Z ";
}

/*! Horizontal segment from x1 to x2 at height yc, chamfered at both ends. */
function hbar(x1, x2, yc, k) {
    return [[x1, yc], [x1 + k, yc - k], [x2 - k, yc - k], [x2, yc], [x2 - k, yc + k], [x1 + k, yc + k]];
}

/*! Vertical segment from y1 to y2 at xc, chamfered at both ends. */
function vbar(xc, y1, y2, k) {
    return [[xc, y1], [xc + k, y1 + k], [xc + k, y2 - k], [xc, y2], [xc - k, y2 - k], [xc - k, y1 + k]];
}

/*! A seven-segment readout of `text` at digit height h.
 *  Returns { lit, ghost, width }: `lit` is the segments that are on, `ghost` every
 *  segment of every cell. ':' and '.' are always lit; unknown characters are blank. */
function seven(text, h, dpr) {
    var m = metrics(h);
    var t = m.thick, k = t / 2, g = h * 0.03;
    var w = m.cell, L = k, R = w - k, T = k, M = h / 2, B = h - k;
    var segs = {
        a: hbar(L + g, R - g, T, k),
        g: hbar(L + g, R - g, M, k),
        d: hbar(L + g, R - g, B, k),
        f: vbar(L, T + g, M - g, k),
        b: vbar(R, T + g, M - g, k),
        e: vbar(L, M + g, B - g, k),
        c: vbar(R, M + g, B - g, k)
    };
    var names = "abcdefg";
    var lit = "", ghost = "", x = 0;
    for (var i = 0; i < text.length; ++i) {
        var ch = text.charAt(i);
        if (ch === ":" || ch === ".") {
            var s = t * 0.9;
            var cx = x + (m.colon - s) / 2;
            var ys = ch === ":" ? [h * 0.28, h * 0.66] : [h - s];
            for (var j = 0; j < ys.length; ++j) {
                lit += lean([[cx, ys[j]], [cx + s, ys[j]], [cx + s, ys[j] + s], [cx, ys[j] + s]], m, h, 0, dpr);
            }
            x += m.colon;
            continue;
        }
        var on = ON.hasOwnProperty(ch) ? ON[ch] : "";
        for (var n = 0; n < names.length; ++n) {
            var name = names.charAt(n);
            var p = lean(segs[name], m, h, x, dpr);
            ghost += p;
            if (on.indexOf(name) >= 0) {
                lit += p;
            }
        }
        x += m.cell + m.gap;
    }
    return { lit: lit, ghost: ghost, width: x > 0 ? x - m.gap + h * m.slant : 0 };
}

/*! `levels` (0..100 each, -1 unknown) as columns of `cells` segments in a
 *  width x height box. The column at index `now` goes to `hot` (the backlight),
 *  the rest of the lit segments to `lit`; every segment is in `ghost`. */
function bars(levels, cells, width, height, dpr, now) {
    var n = levels.length;
    var pitch = width / Math.max(1, n);
    var bw = pitch * 0.6;
    var ch = height / cells;
    var gh = ch * 0.14;
    var ghost = "", lit = "", hot = "";
    for (var i = 0; i < n; ++i) {
        var x0 = sn(i * pitch + (pitch - bw) / 2, dpr);
        var x1 = sn(i * pitch + (pitch + bw) / 2, dpr);
        var level = levels[i] < 0 ? 0 : Math.min(100, levels[i]);
        var on = Math.round(level * cells / 100);
        for (var r = 0; r < cells; ++r) {
            var yb = sn(height - r * ch - gh, dpr);
            var yt = sn(height - (r + 1) * ch + gh, dpr);
            var seg = "M" + x0 + " " + yb + " H" + x1 + " V" + yt + " H" + x0 + " Z ";
            ghost += seg;
            if (r < on) {
                if (i === now) {
                    hot += seg;
                } else {
                    lit += seg;
                }
            }
        }
    }
    return { ghost: ghost, lit: lit, hot: hot };
}

/*! A number in `cells` digit cells, right aligned; dashes when v is not a number. */
function numberText(v, cells) {
    if (typeof v !== "number" || !isFinite(v)) {
        return "-".repeat(cells);
    }
    return String(Math.round(v)).padStart(cells, " ");
}

/*! Hours and minutes of a locale time string as digits, plus the rest ("PM"). */
function clockParts(text) {
    var s = String(text || "");
    var m = /(\d{1,2})\D(\d{2})/.exec(s);
    if (!m) {
        return { digits: "--:--", suffix: "" };
    }
    return { digits: m[1].padStart(2, " ") + ":" + m[2], suffix: s.replace(m[0], "").trim() };
}
