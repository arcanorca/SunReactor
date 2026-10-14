.pragma library

/*
 * Geometry of ModernWeatherIcon.qml: one condition as flat shapes on the 24 x 24
 * design grid (y down), scaled to logical px by `unit`. Every shape is a polygon:
 * circles and arcs are sampled finely (64 steps a turn is well under a device
 * pixel at 64 px), so each layer is one SVG path string. Overlapping parts are
 * merged by the nonzero fill rule, which needs every sub-path wound the same way;
 * wound() enforces that.
 *
 * Nothing here touches QML types, so the file can be checked under node.
 *
 * scene() returns one entry per layer, in px: { d, box: [x0, y0, x1, y1] } (the
 * box is the layer's bounds, for its gradient). Empty layers have d "".
 */

var GRID = 24;
var TAU = 2 * Math.PI;
var DEG = Math.PI / 180;
var CIRCLE_STEPS = 64;
var RAY_R = 0.9;      // sun rays: half of their width, grid units
var SNOW_R = 0.55;    // snowflake arms
var MOON_OUTER = [11.5, 12.5, 7.5];
var MOON_INNER = [15.5, 8.5, 6.2];

/// Relative luminance (WCAG) of a colour with r, g, b in 0..1.
function luminance(c) {
    var lin = function (v) { return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
}

function clamp1(v) {
    return Math.max(-1, Math.min(1, v));
}

// ---- polygons (grid units) --------------------------------------------------------

/// Points of a full circle, counter-clockwise on paper (increasing angle, y down: clockwise on screen).
function circle(cx, cy, r) {
    return arc(cx, cy, r, 0, TAU, CIRCLE_STEPS).slice(0, CIRCLE_STEPS);
}

/// Points on an arc from angle a0 to a1 (radians, inclusive ends), `steps` segments.
function arc(cx, cy, r, a0, a1, steps) {
    var pts = [];
    for (var k = 0; k <= steps; ++k) {
        var a = a0 + (a1 - a0) * k / steps;
        pts.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
    }
    return pts;
}

/// Twice the signed area of a polygon (positive for the winding `wound` produces).
function area2(pts) {
    var s = 0;
    for (var i = 0; i < pts.length; ++i) {
        var p = pts[i], q = pts[(i + 1) % pts.length];
        s += p[0] * q[1] - q[0] * p[1];
    }
    return s;
}

/// The polygon with positive area, so that every sub-path of one path winds alike.
function wound(pts) {
    return area2(pts) >= 0 ? pts : pts.slice().reverse();
}

function rect(x0, y0, x1, y1) {
    return [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
}

/// A segment with round ends: radius r around the segment p0 - p1.
function capsule(x0, y0, x1, y1, r) {
    var th = Math.atan2(y1 - y0, x1 - x0);
    var cap1 = arc(x1, y1, r, th - Math.PI / 2, th + Math.PI / 2, 12);
    var cap0 = arc(x0, y0, r, th + Math.PI / 2, th + 3 * Math.PI / 2, 12);
    return cap1.concat(cap0);
}

/// A four-pointed sparkle with tips up, down, left and right.
function sparkle(cx, cy, R) {
    var pts = [];
    for (var k = 0; k < 8; ++k) {
        var a = -Math.PI / 2 + k * Math.PI / 4;
        var r = k % 2 === 0 ? R : R * 0.24;
        pts.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
    }
    return pts;
}

/// A raindrop: a point at (x, y0) and a round bulb of radius w, `len` long overall.
/// The convex hull of the point and the bulb: tangent lines, round bottom.
function drop(x, y0, len, w) {
    var cy = y0 + len - w;
    var L = len - w;
    var alpha = Math.acos(Math.min(1, w / L));
    var psi = -Math.PI / 2;
    return [[x, y0]].concat(arc(x, cy, w, psi + alpha, psi + TAU - alpha, 32));
}

/// The annular sector from angle a0 to a1 at radius r, half-width hw.
function ring(cx, cy, r, hw, a0, a1) {
    return arc(cx, cy, r + hw, a0, a1, 32).concat(arc(cx, cy, r - hw, a1, a0, 32));
}

/// The disc `o` minus the disc `i` (each [cx, cy, r]): a crescent.
function crescent(o, i) {
    var ox = o[0], oy = o[1], R = o[2], ix = i[0], iy = i[1], r = i[2];
    var dx = ix - ox, dy = iy - oy, d = Math.hypot(dx, dy);
    // Direction from the inner centre to the outer one: the crescent's axis.
    var gamma = Math.atan2(-dy, -dx);
    // Where the two circles meet, measured from the axis on each circle.
    var hO = Math.PI - Math.acos(clamp1((R * R + d * d - r * r) / (2 * R * d)));
    var phi = Math.acos(clamp1((r * r + d * d - R * R) / (2 * r * d)));
    return arc(ox, oy, R, gamma - hO, gamma + hO, 48)
        .concat(arc(ix, iy, r, gamma + phi, gamma - phi, 48));
}

/// The standard cloud on the grid spans x 3..21, y 6..19. The frame maps it to
/// (cx, top) at scale s: the cloud's centre column and its top edge.
function cloudFrame(s, cx, top) {
    return { ox: cx - 12 * s, oy: top - 6 * s, s: s };
}

/// The cloud as the union of four discs and the base block, wound alike.
function cloudPolys(F) {
    var s = F.s;
    var at = function (x, y) { return [F.ox + s * x, F.oy + s * y]; };
    var discs = [[7.0, 15.0, 4.0], [10.5, 11.0, 5.0], [16.0, 11.5, 4.5], [17.0, 15.0, 4.0]];
    var polys = discs.map(function (c) {
        var p = at(c[0], c[1]);
        return wound(circle(p[0], p[1], c[2] * s));
    });
    var a = at(7.0, 15.0), b = at(17.0, 19.0);
    polys.push(wound(rect(a[0], a[1], b[0], b[1])));
    return polys;
}

/// A sun: a disc with rays, each ray a capsule from rIn to rOut at the given angles (degrees).
function sunPolys(cx, cy, r, rIn, rOut, degrees) {
    var polys = [wound(circle(cx, cy, r))];
    degrees.forEach(function (deg) {
        var a = deg * DEG, c = Math.cos(a), s = Math.sin(a);
        polys.push(wound(capsule(cx + rIn * c, cy + rIn * s, cx + rOut * c, cy + rOut * s, RAY_R)));
    });
    return polys;
}

/// A short line with round ends from (x0, y0) to (x1, y1), half-width hw.
function bar(x0, y0, x1, y1, hw) {
    return wound(capsule(x0, y0, x1, y1, hw));
}

/// Three arms through the centre (cx, cy), each half R long: a snowflake.
function snowflake(cx, cy, R) {
    return [90, 30, 150].map(function (deg) {
        var a = deg * DEG, dx = R * Math.cos(a), dy = R * Math.sin(a);
        return wound(capsule(cx - dx, cy - dy, cx + dx, cy + dy, SNOW_R));
    });
}

/// A question mark: hook, stem and dot, for the unknown condition.
function question(cx, cy) {
    return [
        wound(ring(cx, cy - 2.6, 2.0, 0.7, Math.PI, 2 * Math.PI)),
        wound(capsule(cx, cy - 0.6, cx, cy + 0.5, 0.7)),
        wound(circle(cx, cy + 3.4, 0.9)),
    ];
}

// ---- output (px) --------------------------------------------------------------------

function fmt(v) {
    return (Math.round(v * 100) / 100).toString();
}

/// One SVG path string for a list of polygons, in px.
function toD(polys, unit) {
    var parts = [];
    polys.forEach(function (pts) {
        if (pts.length < 3) {
            return;
        }
        var seg = pts.map(function (p, i) {
            return (i === 0 ? "M " : "L ") + fmt(p[0] * unit) + " " + fmt(p[1] * unit);
        });
        parts.push(seg.join(" ") + " Z");
    });
    return parts.join(" ");
}

/// Bounds of a list of polygons, in px: [x0, y0, x1, y1].
function boxOf(polys, unit) {
    var x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    polys.forEach(function (pts) {
        pts.forEach(function (p) {
            x0 = Math.min(x0, p[0]); y0 = Math.min(y0, p[1]);
            x1 = Math.max(x1, p[0]); y1 = Math.max(y1, p[1]);
        });
    });
    if (x0 === Infinity) {
        return [0, 0, 0, 0];
    }
    return [x0 * unit, y0 * unit, x1 * unit, y1 * unit];
}

function layer(polys, unit) {
    return { d: toD(polys, unit), box: boxOf(polys, unit) };
}

function empty() {
    return { d: "", box: [0, 0, 0, 0] };
}

/// Moon polygons for a frame that maps the standard moon (see MOON_*) to a scale and offset.
function moonPolys(F) {
    var at = function (c) { return [F.ox + F.s * c[0], F.oy + F.s * c[1], F.s * c[2]]; };
    return [wound(crescent(at(MOON_OUTER), at(MOON_INNER)))];
}

/*
 * The whole picture for one condition.
 *
 *   condition  daemon token ("clear", "partly_cloudy", ..., "" unknown)
 *   night      night variants: a crescent and stars instead of the sun
 *   unit       logical px per grid unit (side / 24)
 *
 * Returns { glow, sun, moon, stars, back, front, drops, bolt, snow, mist, question }.
 * `glow` and `sun` also carry the centre and radius (px) of their gradients;
 * `glow` is a halo, drawn first; `back` is the cloud behind `front`.
 */
function scene(condition, night, unit) {
    var U = unit;
    var out = {
        glow: { d: "", box: [0, 0, 0, 0], cx: 0, cy: 0, r: 0 },
        sun: { d: "", box: [0, 0, 0, 0], cx: 0, cy: 0, r: 0 },
        moon: empty(),
        stars: empty(),
        back: empty(),
        front: empty(),
        drops: empty(),
        bolt: empty(),
        snow: empty(),
        mist: empty(),
        question: empty(),
    };
    var isNight = !!night;
    var cond = String(condition || "");
    var EIGHT = [0, 45, 90, 135, 180, 225, 270, 315];

    function setSun(cx, cy, r, rIn, rOut, degrees) {
        var polys = sunPolys(cx, cy, r, rIn, rOut, degrees);
        out.sun = { d: toD(polys, U), box: boxOf(polys, U), cx: cx * U, cy: cy * U, r: r * U };
        out.glow = { d: toD([circle(cx, cy, 10.5)], U), box: boxOf([circle(cx, cy, 10.5)], U),
                     cx: cx * U, cy: cy * U, r: 10.5 * U };
    }
    function setMoon(F, starPolys, cx, cy) {
        var polys = moonPolys(F);
        out.moon = layer(polys, U);
        var g = { cx: cx * U, cy: cy * U, r: 8.0 * U };
        out.glow = { d: toD([circle(cx, cy, 8.0)], U), box: boxOf([circle(cx, cy, 8.0)], U),
                     cx: g.cx, cy: g.cy, r: g.r };
        out.stars = layer(starPolys, U);
    }
    function setCloud(F, part) {
        out[part] = layer(cloudPolys(F), U);
    }
    function setDrops(xs, y0, len, w) {
        out.drops = layer(xs.map(function (x) { return wound(drop(x, y0, len, w)); }), U);
    }

    switch (cond) {
    case "clear":
        if (isNight) {
            setMoon({ ox: 0, oy: 0, s: 1 }, [
                wound(sparkle(4.8, 4.8, 2.0)), wound(sparkle(19.4, 3.9, 1.5)), wound(sparkle(20.4, 17.6, 1.2)),
            ], 11.5, 12.5);
        } else {
            setSun(12, 12, 4.6, 6.6, 9.3, EIGHT);
        }
        break;

    case "partly_cloudy": {
        var pc = cloudFrame(0.84, 10.2, 10.2);
        if (isNight) {
            var mF = { ox: 15.2 - 0.72 * 11.5, oy: 8.2 - 0.72 * 12.5, s: 0.72 };
            setMoon(mF, [wound(sparkle(4.6, 4.8, 1.8)), wound(sparkle(10.6, 2.6, 1.3))], 15.2, 8.2);
        } else {
            setSun(15.8, 8.6, 3.7, 5.6, 7.6, [-150, -110, -70, -30, 10]);
        }
        setCloud(pc, "front");
        break;
    }

    case "cloudy":
        setCloud(cloudFrame(0.72, 15.6, 2.4), "back");
        setCloud(cloudFrame(0.9, 10.0, 7.2), "front");
        break;

    case "drizzle":
        setCloud(cloudFrame(0.84, 12, 3.6), "front");
        setDrops([7.5, 12, 16.5], 17.0, 2.0, 0.55);
        break;

    case "rain":
        setCloud(cloudFrame(0.84, 12, 2.6), "front");
        setDrops([7.5, 12, 16.5], 16.0, 3.4, 0.7);
        break;

    case "heavy_rain":
        setCloud(cloudFrame(0.84, 12, 1.9), "front");
        out.drops = layer([6.5, 12, 17.5].map(function (x) { return wound(drop(x, 14.4, 4.2, 0.75)); })
            .concat([9.25, 14.75].map(function (x) { return wound(drop(x, 17.2, 4.2, 0.75)); })), U);
        break;

    case "thunderstorm": {
        setCloud(cloudFrame(0.84, 12, 1.6), "front");
        var bolt = [[13.9, 13.0], [8.7, 18.8], [12.1, 18.8], [10.0, 23.0], [15.6, 16.8], [12.2, 16.8], [14.4, 13.0]];
        out.bolt = layer([wound(bolt)], U);
        out.drops = layer([wound(drop(5.4, 15.6, 3.0, 0.7)), wound(drop(18.6, 15.6, 3.0, 0.7))], U);
        break;
    }

    case "snow":
        setCloud(cloudFrame(0.84, 12, 2.4), "front");
        out.snow = layer(snowflake(6.5, 18.6, 1.8).concat(snowflake(12, 18.6, 1.8), snowflake(17.5, 18.6, 1.8)), U);
        break;

    case "mist":
    case "atmospheric":
        out.mist = layer([wound(ring(15.5, 8.4, 4.2, 0.6, -160 * DEG, 140 * DEG)),
                          bar(3.5, 12.4, 20.5, 12.4, 0.7),
                          bar(6.5, 16.4, 17.5, 16.4, 0.7),
                          bar(3.5, 20.4, 13.5, 20.4, 0.7)], U);
        break;

    case "fog":
        setCloud(cloudFrame(0.84, 12, 2.2), "front");
        out.mist = layer([bar(4.0, 16.0, 20.0, 16.0, 0.7), bar(7.0, 20.0, 17.0, 20.0, 0.7)], U);
        break;

    default:
        setCloud(cloudFrame(0.92, 12, 4.8), "front");
        out.question = layer(question(12, 9.1), U);
        break;
    }
    return out;
}
