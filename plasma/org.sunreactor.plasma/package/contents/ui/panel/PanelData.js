.pragma library

/*
 * Pure helpers for the panel item (no QML types, unit-tested in
 * tests/tst_shell.qml): which brightness to show, today's curve reduced to
 * one series, and colour contrast against the panel.
 */

var MODES = ["sky", "readout", "sparkline", "ring"];

function mode(name) {
    return MODES.indexOf(name) >= 0 ? name : "sky";
}

/*! Applied brightness of `displayId`, or the client's mean when the id is
    empty or no longer present. -1 = unknown (never invented). */
function displayPercent(monitors, displayId, globalPercent) {
    if (displayId) {
        for (var i = 0; i < monitors.length; ++i) {
            if (monitors[i].logicalId === displayId) {
                return monitors[i].percent >= 0 ? monitors[i].percent : -1;
            }
        }
    }
    return globalPercent >= 0 ? globalPercent : -1;
}

/*! client.dayCurve reduced to one series for `displayId` (or the mean of all
    displays): { t0, t1, lo, hi, points: [{t, pct}] }, or null when there is
    no usable curve. lo/hi are the min/max limits of the displays involved,
    so the curve fills the height whatever the limits are. */
function series(dayCurve, displayId) {
    if (!dayCurve || !dayCurve.monitors || dayCurve.monitors.length === 0) {
        return null;
    }
    var monitors = dayCurve.monitors;
    if (displayId) {
        var only = monitors.filter(function (m) { return m.monitorId === displayId; });
        if (only.length > 0) {
            monitors = only;
        }
    }
    var count = monitors[0].samples ? monitors[0].samples.length : 0;
    if (count < 2) {
        return null;
    }
    var points = [];
    for (var i = 0; i < count; ++i) {
        var sum = 0, n = 0, t = monitors[0].samples[i].t;
        for (var j = 0; j < monitors.length; ++j) {
            var s = monitors[j].samples && monitors[j].samples[i];
            if (s && s.t === t && typeof s.pct === "number") {
                sum += s.pct;
                ++n;
            }
        }
        if (n > 0) {
            points.push({ t: t, pct: sum / n });
        }
    }
    if (points.length < 2) {
        return null;
    }
    var lo = 100, hi = 0;
    for (var k = 0; k < monitors.length; ++k) {
        lo = Math.min(lo, monitors[k].minPct);
        hi = Math.max(hi, monitors[k].maxPct);
    }
    if (!(hi > lo)) {
        lo = 0;
        hi = 100;
    }
    return { t0: points[0].t, t1: points[points.length - 1].t, lo: lo, hi: hi, points: points };
}

/*! Linear interpolation of the series at time t (clamped to its ends). */
function valueAt(curve, t) {
    var p = curve.points;
    if (t <= p[0].t) {
        return p[0].pct;
    }
    for (var i = 1; i < p.length; ++i) {
        if (t <= p[i].t) {
            var f = (t - p[i - 1].t) / Math.max(1, p[i].t - p[i - 1].t);
            return p[i - 1].pct + (p[i].pct - p[i - 1].pct) * f;
        }
    }
    return p[p.length - 1].pct;
}

/*! 0..1 position of a value within the series' limit band. */
function level(curve, pct) {
    return Math.max(0, Math.min(1, (pct - curve.lo) / (curve.hi - curve.lo)));
}

/*! 0..1 position of t within the day. */
function dayFraction(curve, t) {
    return Math.max(0, Math.min(1, (t - curve.t0) / Math.max(1, curve.t1 - curve.t0)));
}

function luminance(c) {
    function lin(v) { return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); }
    return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
}

/*! WCAG contrast ratio of two colours (1..21). */
function contrast(a, b) {
    var la = luminance(a), lb = luminance(b);
    return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/*! `preferred` if it reads on `background` (contrast >= minimum), else
    `preferred` pulled towards `fallback` until it does, else `fallback`. */
function ink(preferred, background, fallback, minimum) {
    if (contrast(preferred, background) >= minimum) {
        return preferred;
    }
    for (var t = 0.25; t < 1; t += 0.25) {
        var c = Qt.rgba(preferred.r + (fallback.r - preferred.r) * t,
                        preferred.g + (fallback.g - preferred.g) * t,
                        preferred.b + (fallback.b - preferred.b) * t, 1);
        if (contrast(c, background) >= minimum) {
            return c;
        }
    }
    return fallback;
}

/*! The colour of `candidates` with the best contrast on `background`
    (pixel themes stay inside their palette). */
function bestOn(candidates, background) {
    var best = candidates[0], score = -1;
    for (var i = 0; i < candidates.length; ++i) {
        var s = contrast(candidates[i], background);
        if (s > score) {
            score = s;
            best = candidates[i];
        }
    }
    return best;
}

/*! Unicode block characters for a terminal-style sparkline: `columns`
    samples of the series, 8 heights. */
function blocks(curve, columns) {
    var glyphs = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    var out = "";
    for (var i = 0; i < columns; ++i) {
        var t = curve.t0 + (curve.t1 - curve.t0) * (i + 0.5) / columns;
        out += glyphs[Math.round(level(curve, valueAt(curve, t)) * 7)];
    }
    return out;
}
