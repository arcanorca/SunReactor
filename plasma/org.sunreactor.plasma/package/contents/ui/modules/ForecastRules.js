.pragma library

/*
 * Forecast rules shared by every Forecast module and skin. Pure functions of
 * their arguments, so a binding that calls one tracks what it passes in.
 */

/*! The first `slots` forecast points after `nowEpochS`, in the daemon's order
    (client.forecast: [{ epochS, ... }]). */
function upcoming(forecast, nowEpochS, slots) {
    var out = [];
    for (var i = 0; i < forecast.length; ++i) {
        if (forecast[i].epochS > nowEpochS) {
            out.push(forecast[i]);
        }
        if (out.length === slots) {
            break;
        }
    }
    return out;
}
