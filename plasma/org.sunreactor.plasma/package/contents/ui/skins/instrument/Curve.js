.pragma library

/*
 * Day-curve lookups shared by the instrument skins (client.dayCurve shape:
 * { monitors: [{ monitorId, minPct, maxPct, samples: [{ t, pct, pctNoWeather }],
 * milestones: [...] }], sun: {...} }).
 */

/*! Linear interpolation of a sample field at time t; -1 outside the samples. */
function valueAt(samples, t, field) {
    if (!samples || samples.length === 0 || t < samples[0].t || t > samples[samples.length - 1].t) {
        return -1;
    }
    for (var i = 1; i < samples.length; ++i) {
        var b = samples[i];
        if (b.t >= t) {
            var a = samples[i - 1];
            var f = b.t === a.t ? 0 : (t - a.t) / (b.t - a.t);
            return a[field] + (b[field] - a[field]) * f;
        }
    }
    return samples[samples.length - 1][field];
}

/*! The curve entry for a display id, or null. */
function curveFor(dayCurve, monitorId) {
    if (!dayCurve || !dayCurve.monitors) {
        return null;
    }
    for (var i = 0; i < dayCurve.monitors.length; ++i) {
        if (dayCurve.monitors[i].monitorId === monitorId) {
            return dayCurve.monitors[i];
        }
    }
    return null;
}

/*! Where the automatic curve of a display is at time t; -1 when unknown. */
function targetAt(dayCurve, monitorId, t) {
    var curve = curveFor(dayCurve, monitorId);
    return curve ? valueAt(curve.samples, t, "pct") : -1;
}

/*! Short caps channel name: "len-p24h-20" -> "LEN". */
function shortName(id) {
    var text = String(id);
    var head = text.split(/[-_ .]/)[0] || text;
    return head.slice(0, 4).toUpperCase();
}
