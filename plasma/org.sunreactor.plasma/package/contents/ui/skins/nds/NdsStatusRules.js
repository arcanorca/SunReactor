.pragma library

/*
 * Nintendo DS status bar telemetry rules:
 *  1. Ascending signal bars (0..4 when connected, -1 offline) measure how close
 *     active displays are to their configured max_pct ceiling.
 *  2. Solar battery level (0..3 when connected, -1 offline) follows the local
 *     sun clock: charging upward from sunrise to sunset, discharging overnight,
 *     and reaching 0 shortly before sunrise.
 */

/*! Mean proximity of enabled displays to their configured max brightness in [0, 1], or -1 if unknown. */
function brightnessMaxProximity(monitors, globalPercent) {
    var sum = 0;
    var count = 0;
    if (monitors && monitors.length > 0) {
        for (var i = 0; i < monitors.length; ++i) {
            var m = monitors[i];
            if (!m || m.enabled === false || m.percent === undefined || Number(m.percent) < 0) {
                continue;
            }
            var pct = Number(m.percent);
            var minP = m.minPct !== undefined && Number(m.minPct) >= 0 ? Number(m.minPct) : 0;
            var maxP = m.maxPct !== undefined && Number(m.maxPct) > 0 ? Number(m.maxPct) : 100;
            var ratio = maxP <= minP
                ? (pct >= maxP ? 1 : 0)
                : Math.max(0, Math.min(1, (pct - minP) / (maxP - minP)));
            sum += ratio;
            ++count;
        }
    }
    if (count > 0) {
        return sum / count;
    }
    if (globalPercent !== undefined && Number(globalPercent) >= 0) {
        return Math.max(0, Math.min(1, Number(globalPercent) / 100));
    }
    return -1;
}

/*! 0..4 lit ascending bars when connected, or -1 while disconnected. */
function signalBars(connected, monitors, globalPercent) {
    if (!connected) {
        return -1;
    }
    var ratio = brightnessMaxProximity(monitors, globalPercent);
    if (ratio < 0) {
        return 0;
    }
    if (ratio >= 0.88) {
        return 4;
    }
    if (ratio >= 0.62) {
        return 3;
    }
    if (ratio >= 0.36) {
        return 2;
    }
    if (ratio >= 0.10) {
        return 1;
    }
    return 0;
}

/*! Continuous solar battery charge in [0, 1]: rises sunrise->sunset, falls sunset->next sunrise, -1 unknown. */
function solarBatteryCharge(nowEpochS, sunriseEpochS, sunsetEpochS) {
    if (nowEpochS <= 0 || sunriseEpochS <= 0 || sunsetEpochS <= sunriseEpochS) {
        return -1;
    }
    var dayLen = sunsetEpochS - sunriseEpochS;
    if (nowEpochS >= sunriseEpochS && nowEpochS < sunsetEpochS) {
        return Math.max(0, Math.min(1, (nowEpochS - sunriseEpochS) / Math.max(1, dayLen)));
    }
    var nightLen = Math.max(1, 86400 - dayLen);
    var rem = nowEpochS >= sunsetEpochS
        ? 1 - (nowEpochS - sunsetEpochS) / nightLen
        : (sunriseEpochS - nowEpochS) / nightLen;
    return Math.max(0, Math.min(1, rem));
}

/*! Discrete battery segment level: -1 offline, 0..3 synchronized to the solar clock. */
function solarBatteryLevel(connected, nowEpochS, sunriseEpochS, sunsetEpochS, isDaylight) {
    if (!connected || nowEpochS <= 0) {
        return -1;
    }
    if (sunriseEpochS <= 0 || sunsetEpochS <= sunriseEpochS) {
        return isDaylight ? 3 : 1;
    }
    var charge = solarBatteryCharge(nowEpochS, sunriseEpochS, sunsetEpochS);
    if (nowEpochS >= sunriseEpochS && nowEpochS < sunsetEpochS) {
        if (charge >= 0.67) {
            return 3;
        }
        if (charge >= 0.34) {
            return 2;
        }
        return 1;
    }
    if (charge > 0.68) {
        return 3;
    }
    if (charge > 0.36) {
        return 2;
    }
    if (charge >= 0.10) {
        return 1;
    }
    return 0;
}
