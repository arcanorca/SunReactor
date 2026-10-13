.pragma library

/*
 * Geometry and colour rules for WeatherGlyph.qml.
 *
 * Glyphs are authored on a 24 x 24 design grid (y down). build() turns one
 * condition into a flat list of { d, c, w } elements in logical px:
 *   w > 0  a stroke of that width (line style)
 *   w == 0 a flat fill (solid style)
 * Nothing here touches QML types, so the geometry can be checked under node.
 *
 * Paths are kept as op lists ({ k: "L" | "A" }) until the last moment, so the
 * line style can hide what sits behind a cloud by trimming outlines against the
 * cloud's own shape, without any background-coloured fill.
 */

var GRID = 24;
var TAU = 2 * Math.PI;
var HALF_PI = Math.PI / 2;
var DEG = Math.PI / 180;

var LINE_WEIGHT = 1.75;      // nominal stroke, grid units
var MIN_BG_CONTRAST = 2.0;   // every role colour must reach this against palette.bg
var MIN_ROLE_CONTRAST = 1.5; // a secondary role falls back when this close to the cloud
var SHADE_DROP = 1.1;        // solid clouds: depth band height below the body

// Standard cloud on the grid, standalone frame: spans x 3..21, y 6..19. The two
// end circles share the flat base at y = 19; the two bumps sit on top.
var CLOUD = {
    left: [6.5, 15.5, 3.5],
    bump: [10.5, 11.0, 5.0],
    right: [16.0, 11.5, 4.5],
    end: [17.0, 15.0, 4.0]
};
// Crescent: the outer disc minus an offset inner disc.
var MOON = { outer: [11.5, 12.5, 7.5], inner: [15.5, 8.5, 6.2] };

// ---- device pixels -------------------------------------------------------------

/// Stroke width in whole device pixels for a glyph devSize device px across.
/// Never thinner than 2 px, so the panel size (16-22 px) keeps its weight.
function strokeDevicePx(devSize) {
    return Math.max(2, Math.round(LINE_WEIGHT * devSize / GRID));
}

// ---- colour ----------------------------------------------------------------------

function clamp255(v) {
    return Math.max(0, Math.min(255, Math.round(v)));
}

/// Accepts "#rgb", "#rrggbb", "#aarrggbb" strings and QML colour values.
/// Returns [r, g, b] in 0..255, or null when the value is not a colour.
function parseColor(c) {
    if (c === undefined || c === null) {
        return null;
    }
    if (typeof c === "string") {
        var s = c.trim();
        if (s.charAt(0) !== "#") {
            return null;
        }
        var h = s.slice(1);
        if (h.length === 3) {
            h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
        }
        if (h.length === 8) {
            h = h.slice(2);
        }
        if (h.length !== 6 || /[^0-9a-fA-F]/.test(h)) {
            return null;
        }
        return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
    }
    if (typeof c === "object" && c.r !== undefined && c.g !== undefined && c.b !== undefined) {
        return [c.r * 255, c.g * 255, c.b * 255];
    }
    return null;
}

function hex(rgb) {
    return "#" + rgb.map(function (v) {
        var s = clamp255(v).toString(16);
        return s.length < 2 ? "0" + s : s;
    }).join("");
}

function mix(a, b, t) {
    return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

function luminance(rgb) {
    function lin(v) {
        var x = v / 255;
        return x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
    }
    return 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2]);
}

/// WCAG contrast ratio, 1 (identical) to 21.
function contrast(a, b) {
    var la = luminance(a), lb = luminance(b);
    return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/// Blends c toward target until it reaches MIN_BG_CONTRAST on bg. Returns null
/// when even the full target is not enough.
function blendUntilLegible(c, bg, target) {
    if (contrast(c, bg) >= MIN_BG_CONTRAST) {
        return c;
    }
    for (var i = 1; i <= 20; ++i) {
        var m = mix(c, target, i / 20);
        if (contrast(m, bg) >= MIN_BG_CONTRAST) {
            return m;
        }
    }
    return null;
}

/// Contrast guard: a role colour that sits too close to the background is
/// pulled toward fg, and toward the bg's inverse when fg cannot do it either.
function guardRole(c, bg, fg) {
    var m = blendUntilLegible(c, bg, fg);
    if (m) {
        return m;
    }
    var inverse = luminance(bg) > 0.18 ? [0, 0, 0] : [255, 255, 255];
    return blendUntilLegible(c, bg, inverse) || inverse;
}

var FALLBACK = {
    bg: "#15171c",
    fg: "#e6e6e6",
    text_muted: "#8c8c8c",
    accent: "#ff9f43",
    secondary_accent: "#54a0ff",
    warning: "#feca57"
};

/// Colour roles for one glyph. Missing palette keys fall back to FALLBACK.
function resolvePalette(palette) {
    var p = palette || {};
    function pick(key) {
        return parseColor(p[key]) || parseColor(FALLBACK[key]);
    }
    var bg = pick("bg");
    var fg = guardRole(pick("fg"), bg, pick("fg"));
    var muted = guardRole(pick("text_muted"), bg, fg);
    var rawAccent = pick("accent");
    var rawRain = pick("secondary_accent");
    var rawBolt = pick("warning");
    // Rain and the bolt keep their own colour unless it is too close to the cloud.
    var rain = contrast(rawRain, fg) >= MIN_ROLE_CONTRAST ? rawRain : rawAccent;
    var bolt = contrast(rawBolt, fg) >= MIN_ROLE_CONTRAST ? rawBolt : rawAccent;
    var sun = guardRole(rawAccent, bg, fg);
    return {
        bg: bg,
        fg: fg,
        muted: muted,
        sun: sun,
        rain: guardRole(rain, bg, fg),
        bolt: guardRole(bolt, bg, fg),
        // Solid clouds: depth band and the back cloud of a pair.
        shade: mix(fg, muted, 0.5),
        // Faint hint (sun behind mist): the sun colour pulled toward the background.
        faint: mix(sun, bg, 0.5)
    };
}

// ---- primitives ------------------------------------------------------------------
// A path is { ops: [...], closed: bool }. An op is either
//   { k: "L", x0, y0, x1, y1 }                    a straight line
//   { k: "A", cx, cy, r, a0, sw }                 an arc from angle a0 sweeping sw
// Angles are radians, measured with y down, so +HALF_PI points at the bottom.

function norm(a) {
    var t = a % TAU;
    return t < 0 ? t + TAU : t;
}

/// Sweep from a0 to a1 that passes through angle mid (the arc facing outward).
function sweepThrough(a0, a1, mid) {
    var ccw = norm(a1 - a0);
    return norm(mid - a0) < ccw ? ccw : ccw - TAU;
}

function angleOn(p, c) {
    return Math.atan2(p[1] - c[1], p[0] - c[0]);
}

function arcOp(c, a0, a1, mid) {
    return { k: "A", cx: c[0], cy: c[1], r: c[2], a0: a0, sw: sweepThrough(a0, a1, mid) };
}

function lineOp(x0, y0, x1, y1) {
    return { k: "L", x0: x0, y0: y0, x1: x1, y1: y1 };
}

function linePath(x0, y0, x1, y1) {
    return { ops: [lineOp(x0, y0, x1, y1)], closed: false };
}

function circlePath(c) {
    return { ops: [{ k: "A", cx: c[0], cy: c[1], r: c[2], a0: 0, sw: TAU }], closed: true };
}

function polygonPath(points) {
    var ops = [];
    for (var i = 0; i < points.length; ++i) {
        var a = points[i], b = points[(i + 1) % points.length];
        ops.push(lineOp(a[0], a[1], b[0], b[1]));
    }
    return { ops: ops, closed: true };
}

/// Two intersection points of circles a and b, each [x, y, r].
function circleHits(a, b) {
    var dx = b[0] - a[0], dy = b[1] - a[1], d = Math.hypot(dx, dy);
    var x = (a[2] * a[2] - b[2] * b[2] + d * d) / (2 * d);
    var h = Math.sqrt(Math.max(0, a[2] * a[2] - x * x));
    var px = a[0] + x * dx / d, py = a[1] + x * dy / d;
    return [[px - h * dy / d, py + h * dx / d], [px + h * dy / d, py - h * dx / d]];
}

function dist2(a, b) {
    var dx = a[0] - b[0], dy = a[1] - b[1];
    return dx * dx + dy * dy;
}

/// The intersection of a and b that lies farther from `centre`.
function outerHit(a, b, centre) {
    var hits = circleHits(a, b);
    return dist2(hits[0], centre) >= dist2(hits[1], centre) ? hits[0] : hits[1];
}

function pointOnOp(op, t) {
    if (op.k === "L") {
        return [op.x0 + (op.x1 - op.x0) * t, op.y0 + (op.y1 - op.y0) * t];
    }
    var a = op.a0 + op.sw * t;
    return [op.cx + op.r * Math.cos(a), op.cy + op.r * Math.sin(a)];
}

function opStart(op) {
    return pointOnOp(op, 0);
}

function opEnd(op) {
    return pointOnOp(op, 1);
}

function subOp(op, t0, t1) {
    if (op.k === "L") {
        var a = pointOnOp(op, t0), b = pointOnOp(op, t1);
        return lineOp(a[0], a[1], b[0], b[1]);
    }
    return { k: "A", cx: op.cx, cy: op.cy, r: op.r, a0: op.a0 + op.sw * t0, sw: op.sw * (t1 - t0) };
}

// ---- occlusion -------------------------------------------------------------------

/// Predicate over grid points: inside any circle {x, y, r} or rectangle {x0, y0, x1, y1}.
function coverFn(shapes) {
    return function (x, y) {
        for (var i = 0; i < shapes.length; ++i) {
            var s = shapes[i];
            if (s.r !== undefined) {
                var dx = x - s.x, dy = y - s.y;
                if (dx * dx + dy * dy < s.r * s.r) {
                    return true;
                }
            } else if (x >= s.x0 && x <= s.x1 && y >= s.y0 && y <= s.y1) {
                return true;
            }
        }
        return false;
    };
}

/// Parameter intervals [t0, t1] of op (0..1) that the predicate does not cover.
/// Scans 96 samples, then bisects each change to ~6e-8.
function keepIntervals(op, covered) {
    var N = 96;
    var kept = [];
    var prevT = 0;
    var prevC = covered(pointOnOp(op, 0));
    var startT = prevC ? null : 0;
    for (var i = 1; i <= N; ++i) {
        var t = i / N;
        var c = covered(pointOnOp(op, t));
        if (c !== prevC) {
            var lo = prevT, hi = t;
            for (var k = 0; k < 24; ++k) {
                var mid = (lo + hi) / 2;
                if (covered(pointOnOp(op, mid)) === prevC) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            var b = (lo + hi) / 2;
            if (prevC) {
                startT = b;
            } else {
                kept.push([startT, b]);
            }
            prevC = c;
        }
        prevT = t;
    }
    if (!prevC) {
        kept.push([startT, 1]);
    }
    return kept.filter(function (iv) {
        return iv[1] - iv[0] > 1e-4;
    });
}

/// The part of a path that is not hidden by the predicate. A path that is
/// untouched keeps its closed flag.
function trimPath(path, covered) {
    var ops = [];
    var whole = true;
    path.ops.forEach(function (op) {
        var iv = keepIntervals(op, covered);
        if (iv.length !== 1 || iv[0][0] > 1e-9 || iv[0][1] < 1 - 1e-9) {
            whole = false;
        }
        iv.forEach(function (t) {
            ops.push(subOp(op, t[0], t[1]));
        });
    });
    return { ops: ops, closed: path.closed && whole };
}

// ---- solid shapes ----------------------------------------------------------------

/// A line of width w with round ends, as a closed fill.
function capsulePath(x0, y0, x1, y1, w) {
    var h = w / 2;
    var dx = x1 - x0, dy = y1 - y0, len = Math.hypot(dx, dy);
    if (len < 1e-6) {
        return circlePath([x0, y0, h]);
    }
    var th = Math.atan2(dy, dx);
    var nx = -Math.sin(th) * h, ny = Math.cos(th) * h;
    var ops = [
        lineOp(x0 + nx, y0 + ny, x1 + nx, y1 + ny),
        { k: "A", cx: x1, cy: y1, r: h, a0: th + HALF_PI, sw: -Math.PI },
        lineOp(x1 - nx, y1 - ny, x0 - nx, y0 - ny),
        { k: "A", cx: x0, cy: y0, r: h, a0: th - HALF_PI, sw: -Math.PI }
    ];
    return { ops: ops, closed: true };
}

/// An annular sector from angle a0 sweeping sw, between radii ri and ro.
function ringPath(cx, cy, ro, ri, a0, sw) {
    var a1 = a0 + sw;
    var ops = [
        { k: "A", cx: cx, cy: cy, r: ro, a0: a0, sw: sw },
        lineOp(cx + ro * Math.cos(a1), cy + ro * Math.sin(a1), cx + ri * Math.cos(a1), cy + ri * Math.sin(a1)),
        { k: "A", cx: cx, cy: cy, r: ri, a0: a1, sw: -sw },
        lineOp(cx + ri * Math.cos(a0), cy + ri * Math.sin(a0), cx + ro * Math.cos(a0), cy + ro * Math.sin(a0))
    ];
    return { ops: ops, closed: true };
}

// ---- emission --------------------------------------------------------------------

/// SVG path data in logical px. `unit` converts grid units to logical px.
function toD(paths, unit) {
    var parts = [];
    function q(v) {
        return Math.round(v * unit * 1000) / 1000;
    }
    function pt(p) {
        return q(p[0]) + " " + q(p[1]);
    }
    var cur = null;
    paths.forEach(function (path) {
        path.ops.forEach(function (op) {
            var s = opStart(op), e = opEnd(op);
            if (cur === null || Math.abs(cur[0] - s[0]) > 1e-6 || Math.abs(cur[1] - s[1]) > 1e-6) {
                parts.push("M" + pt(s));
            }
            if (op.k === "L") {
                parts.push("L" + pt(e));
            } else {
                // Cubic approximation, at most a quarter turn per segment.
                var n = Math.max(1, Math.ceil(Math.abs(op.sw) / HALF_PI - 1e-9));
                var step = op.sw / n, kap = 4 / 3 * Math.tan(step / 4), r = op.r;
                for (var i = 0; i < n; ++i) {
                    var a = op.a0 + i * step, b = a + step;
                    var sa = Math.sin(a), ca = Math.cos(a), sb = Math.sin(b), cb = Math.cos(b);
                    var p0 = [op.cx + r * ca, op.cy + r * sa];
                    var p3 = [op.cx + r * cb, op.cy + r * sb];
                    var p1 = [p0[0] - kap * r * sa, p0[1] + kap * r * ca];
                    var p2 = [p3[0] + kap * r * sb, p3[1] - kap * r * cb];
                    parts.push("C" + pt(p1) + " " + pt(p2) + " " + pt(p3));
                }
            }
            cur = e;
        });
        if (path.closed && path.ops.length) {
            parts.push("Z");
            cur = null;
        }
    });
    return parts.join(" ");
}

// ---- cloud and moon geometry -----------------------------------------------------

/// Cloud outline placed by frame F = { ox, oy, s }. Returns the closed outline
/// path and the shapes (circles and base rectangle) that cover its body.
function cloudGeom(F, dy) {
    function T(c) {
        return [F.ox + F.s * c[0], F.oy + F.s * c[1] + dy, F.s * c[2]];
    }
    var L = T(CLOUD.left), M = T(CLOUD.bump), S = T(CLOUD.right), R = T(CLOUD.end);
    var base = L[1] + L[2];
    var centre = [(L[0] + M[0] + S[0] + R[0]) / 4, (L[1] + M[1] + S[1] + R[1]) / 4];
    function dir(c) {
        return Math.atan2(c[1] - centre[1], c[0] - centre[0]);
    }
    var pSR = outerHit(S, R, centre);
    var pMS = outerHit(M, S, centre);
    var pLM = outerHit(L, M, centre);
    var ops = [
        lineOp(L[0], base, R[0], base),
        arcOp(R, HALF_PI, angleOn(pSR, R), dir(R)),
        arcOp(S, angleOn(pSR, S), angleOn(pMS, S), dir(S)),
        arcOp(M, angleOn(pMS, M), angleOn(pLM, M), dir(M)),
        arcOp(L, angleOn(pLM, L), HALF_PI, dir(L))
    ];
    return {
        path: { ops: ops, closed: true },
        shapes: [
            { x: L[0], y: L[1], r: L[2] },
            { x: M[0], y: M[1], r: M[2] },
            { x: S[0], y: S[1], r: S[2] },
            { x: R[0], y: R[1], r: R[2] },
            { x0: L[0], y0: M[1], x1: R[0], y1: base }
        ]
    };
}

/// Crescent outline placed by frame F: outer arc, then the inner arc back.
function moonPath(F) {
    var o = [F.ox + F.s * MOON.outer[0], F.oy + F.s * MOON.outer[1], F.s * MOON.outer[2]];
    var i = [F.ox + F.s * MOON.inner[0], F.oy + F.s * MOON.inner[1], F.s * MOON.inner[2]];
    var hits = circleHits(o, i);
    var away = Math.atan2(o[1] - i[1], o[0] - i[0]);
    return {
        ops: [
            arcOp(o, angleOn(hits[0], o), angleOn(hits[1], o), away),
            arcOp(i, angleOn(hits[1], i), angleOn(hits[0], i), away)
        ],
        closed: true
    };
}

// ---- conditions ------------------------------------------------------------------

/// Builds the elements for one glyph. `unit` is logical px per grid unit,
/// `strokePx` the device-snapped stroke width in logical px.
function build(condition, night, style, unit, strokePx, palette) {
    var P = resolvePalette(palette);
    var solid = style === "solid";
    var isNight = !!night;
    var wBar = strokePx / unit;   // solid bars and hooks, grid units
    var out = [];
    var WHOLE = { cx: 12, cy: 12 };

    function push(paths, color, w) {
        var d = toD(paths, unit);
        if (d.length) {
            out.push({ d: d, c: hex(color), w: w });
        }
    }
    function trim(paths, covers) {
        if (solid || !covers || !covers.length) {
            return paths;
        }
        var pred = coverFn(covers);
        return paths.map(function (p) {
            return trimPath(p, pred);
        });
    }
    // Outline: a stroke in line style, a flat fill in solid style.
    function body(paths, color, covers) {
        if (solid) {
            push(paths, color, 0);
        } else {
            push(trim(paths, covers), color, strokePx);
        }
    }
    // A short line: a stroke in line style, a capsule fill in solid style.
    function bar(x0, y0, x1, y1, color, covers) {
        if (solid) {
            push([capsulePath(x0, y0, x1, y1, wBar)], color, 0);
        } else {
            push(trim([linePath(x0, y0, x1, y1)], covers), color, strokePx);
        }
    }
    // An arc stroke, or an annular sector in solid style.
    function arcMark(cx, cy, r, a0, sw, color) {
        if (solid) {
            push([ringPath(cx, cy, r + wBar / 2, r - wBar / 2, a0, sw)], color, 0);
        } else {
            push([{ ops: [{ k: "A", cx: cx, cy: cy, r: r, a0: a0, sw: sw }], closed: false }], color, strokePx);
        }
    }
    function dot(cx, cy, r, color) {
        push([circlePath([cx, cy, r])], color, 0);
    }
    function star(cx, cy, R, color) {
        var pts = [];
        for (var k = 0; k < 8; ++k) {
            var a = -HALF_PI + k * Math.PI / 4;
            var rr = k % 2 === 0 ? R : R * 0.28;
            pts.push([cx + rr * Math.cos(a), cy + rr * Math.sin(a)]);
        }
        push([polygonPath(pts)], color, 0);
    }
    function sun(cx, cy, r, rIn, rOut, degrees, color, covers) {
        body([circlePath([cx, cy, r])], color, covers);
        degrees.forEach(function (deg) {
            var a = deg * DEG, c = Math.cos(a), s = Math.sin(a);
            bar(cx + rIn * c, cy + rIn * s, cx + rOut * c, cy + rOut * s, color, covers);
        });
    }
    function moon(F, color, covers) {
        body([moonPath(F)], color, covers);
    }
    function cloudShapes(F) {
        return cloudGeom(F, 0).shapes;
    }
    // Cloud body. Solid style draws the depth band first, then the body on top.
    function cloud(F, color, covers, bandColor) {
        if (solid && bandColor) {
            body([cloudGeom(F, SHADE_DROP).path], bandColor);
        }
        body([cloudGeom(F, 0).path], color, covers);
    }
    function drop(x, y, len, slant, color) {
        bar(x + slant / 2, y, x - slant / 2, y + len, color, null);
    }
    function asterisk(cx, cy, R, color) {
        [90, 30, 150].forEach(function (deg) {
            var a = deg * DEG, dx = R * Math.cos(a), dy = R * Math.sin(a);
            bar(cx - dx, cy - dy, cx + dx, cy + dy, color, null);
        });
    }
    // A question mark: hook, stem and dot. Solid style knocks it out of the cloud.
    function question(cx, cy, color) {
        arcMark(cx, cy, 2.0, Math.PI, 1.5 * Math.PI, color);
        bar(cx, cy + 2.0, cx, cy + 3.3, color, null);
        dot(cx, cy + 5.6, 1.0, color);
    }

    var rain = P.rain;
    var bolt = P.bolt;
    var fullFrame = { ox: 0, oy: 0, s: 1 };

    switch (String(condition || "")) {
    case "clear":
        if (isNight) {
            moon(fullFrame, P.fg, null);
            if (solid) {
                star(4.8, 4.8, 2.0, P.muted);
                star(19.4, 3.9, 1.5, P.muted);
                star(20.4, 17.6, 1.2, P.muted);
            }
        } else {
            sun(WHOLE.cx, WHOLE.cy, 4.2, 6.9, 9.4, [0, 45, 90, 135, 180, 225, 270, 315], P.sun, null);
        }
        break;

    case "partly_cloudy": {
        var pcCloud = cloudFrame(0.84, 10.2, 10.2);
        var pcShapes = cloudShapes(pcCloud);
        if (isNight) {
            moon({ ox: 15.2 - 0.72 * 11.5, oy: 8.2 - 0.72 * 12.5, s: 0.72 }, P.fg, pcShapes);
            if (solid) {
                star(4.6, 4.8, 1.8, P.muted);
                star(10.6, 2.6, 1.3, P.muted);
            }
        } else {
            sun(15.8, 8.6, 3.7, 5.6, 7.6, [-135, -90, -45, 0], P.sun, pcShapes);
        }
        cloud(pcCloud, P.fg, null, P.shade);
        break;
    }

    case "cloudy": {
        var front = cloudFrame(0.9, 10.0, 7.2);
        var back = cloudFrame(0.72, 15.6, 2.4);
        cloud(back, solid ? P.shade : P.muted, cloudShapes(front), null);
        cloud(front, P.fg, null, P.shade);
        break;
    }

    case "drizzle": {
        cloud(cloudFrame(0.84, 12, 3.6), P.fg, null, P.shade);
        [7.5, 12, 16.5].forEach(function (x) {
            drop(x, 17.0, 2.0, 0.6, rain);
        });
        break;
    }

    case "rain": {
        cloud(cloudFrame(0.84, 12, 2.6), P.fg, null, P.shade);
        [7.5, 12, 16.5].forEach(function (x) {
            drop(x, 16.0, 3.4, 0.9, rain);
        });
        break;
    }

    case "heavy_rain": {
        cloud(cloudFrame(0.84, 12, 1.9), P.fg, null, P.shade);
        [6.5, 12, 17.5].forEach(function (x) {
            drop(x, 14.4, 4.2, 0.8, rain);
        });
        [9.25, 14.75].forEach(function (x) {
            drop(x, 17.2, 4.2, 0.8, rain);
        });
        break;
    }

    case "thunderstorm": {
        cloud(cloudFrame(0.84, 12, 1.6), P.fg, null, P.shade);
        body([polygonPath([[13.6, 13.8], [9.0, 19.0], [12.2, 19.0], [10.2, 22.6], [15.2, 17.0], [11.9, 17.0]])], bolt, null);
        drop(5.4, 15.6, 3.0, 0.9, rain);
        drop(18.6, 15.6, 3.0, 0.9, rain);
        break;
    }

    case "snow": {
        cloud(cloudFrame(0.84, 12, 2.4), P.fg, null, P.shade);
        asterisk(6.5, 18.6, 1.8, P.fg);
        asterisk(12, 18.6, 1.8, P.fg);
        asterisk(17.5, 18.6, 1.8, P.fg);
        break;
    }

    case "mist":
    case "atmospheric":
        arcMark(15.5, 8.4, 4.2, -160 * DEG, 140 * DEG, P.faint);
        bar(3.5, 12.4, 20.5, 12.4, P.muted, null);
        bar(6.5, 16.4, 17.5, 16.4, P.muted, null);
        bar(3.5, 20.4, 13.5, 20.4, P.muted, null);
        break;

    case "fog":
        cloud(cloudFrame(0.84, 12, 2.2), P.fg, null, P.shade);
        bar(4.0, 16.0, 20.0, 16.0, P.muted, null);
        bar(7.0, 20.0, 17.0, 20.0, P.muted, null);
        break;

    default:
        cloud(cloudFrame(0.92, 12, 4.8), P.fg, null, P.shade);
        question(12, 9.1, solid ? P.bg : P.fg);
        break;
    }
    return out;
}

/// Frame that puts the standard cloud (3..21 x 6..19) centred on cx with its top at top.
function cloudFrame(s, cx, top) {
    return { ox: cx - 12 * s, oy: top - 6 * s, s: s };
}
