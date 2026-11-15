.pragma library

/*
 * Plot helpers for the PS3 skin's day curve (the curve lookups are in
 * modules/DayCurve.js). Pure functions: they build SVG path strings and
 * positions; the QML draws them.
 */

/*! Samples mapped into a w x h plot: time over [t0, t0 + span], pct over 0..100
    with 100 at the top. */
function points(samples, t0, span, w, h) {
    var out = [];
    for (var i = 0; i < samples.length; ++i) {
        var s = samples[i];
        var p = Math.max(0, Math.min(100, s.pct));
        out.push({ x: (s.t - t0) / span * w, y: h * (1 - p / 100) });
    }
    return out;
}

/*! A smooth SVG path through the points: Catmull-Rom converted to cubic Beziers. */
function smoothPath(pts) {
    if (pts.length < 2) {
        return "";
    }
    var d = "M " + pts[0].x.toFixed(2) + " " + pts[0].y.toFixed(2);
    for (var i = 0; i < pts.length - 1; ++i) {
        var p0 = pts[Math.max(0, i - 1)];
        var p1 = pts[i];
        var p2 = pts[i + 1];
        var p3 = pts[Math.min(pts.length - 1, i + 2)];
        var c1x = p1.x + (p2.x - p0.x) / 6;
        var c1y = p1.y + (p2.y - p0.y) / 6;
        var c2x = p2.x - (p3.x - p1.x) / 6;
        var c2y = p2.y - (p3.y - p1.y) / 6;
        d += " C " + c1x.toFixed(2) + " " + c1y.toFixed(2) + " " + c2x.toFixed(2) + " " + c2y.toFixed(2)
            + " " + p2.x.toFixed(2) + " " + p2.y.toFixed(2);
    }
    return d;
}

/*! The smooth curve closed down to the baseline at h: the area under it. */
function areaPath(pts, h) {
    if (pts.length < 2) {
        return "";
    }
    var last = pts[pts.length - 1];
    var first = pts[0];
    return smoothPath(pts) + " L " + last.x.toFixed(2) + " " + h.toFixed(2)
        + " L " + first.x.toFixed(2) + " " + h.toFixed(2) + " Z";
}

/*! Local-time hour marks every six hours inside [t0, t0 + span], as { x, label }
    in a plot w wide. */
function hourTicks(t0, span, w) {
    var out = [];
    var start = new Date(t0 * 1000);
    start.setMinutes(0, 0, 0);
    var first = start.getTime() / 1000;
    for (var k = 0; k < 60; ++k) {
        var t = first + k * 3600;
        if (t > t0 + span) {
            break;
        }
        if (t < t0) {
            continue;
        }
        var local = new Date(t * 1000);
        if (local.getHours() % 6 === 0) {
            out.push({ x: (t - t0) / span * w, label: ("0" + local.getHours()).slice(-2) });
        }
    }
    return out;
}
