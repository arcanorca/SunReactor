.pragma library

/*
 * Frame indices of the pixel atlases written by tools/generate_weather_icons.py
 * (ATLAS_ORDER / UI8_ORDER there). Paths resolve against this file
 * (contents/ui/skins/pixel/).
 */

var WEATHER = ["clear-day", "clear-night", "partly-cloudy-day", "partly-cloudy-night", "cloudy",
               "drizzle", "rain", "heavy-rain", "thunderstorm", "snow", "mist", "fog", "unknown"];
var UI8 = ["sun", "moon", "display", "pause", "clock", "hand", "drop", "rise", "set", "noon", "lock"];

function weatherAtlas() {
    return Qt.resolvedUrl("../../../icons/pixel/weather16.png");
}

function symbolAtlas() {
    return Qt.resolvedUrl("../../../icons/pixel/ui8.png");
}

function weatherFrame(name) {
    const i = WEATHER.indexOf(name);
    return i >= 0 ? i : WEATHER.indexOf("unknown");
}

function symbol(name) {
    return Math.max(0, UI8.indexOf(name));
}

/*! Daemon condition token -> weather frame, honouring night. */
function conditionFrame(condition, isNight) {
    switch (condition) {
    case "clear":
        return weatherFrame(isNight ? "clear-night" : "clear-day");
    case "partly_cloudy":
        return weatherFrame(isNight ? "partly-cloudy-night" : "partly-cloudy-day");
    case "cloudy":
        return weatherFrame("cloudy");
    case "drizzle":
        return weatherFrame("drizzle");
    case "rain":
        return weatherFrame("rain");
    case "heavy_rain":
        return weatherFrame("heavy-rain");
    case "thunderstorm":
        return weatherFrame("thunderstorm");
    case "snow":
        return weatherFrame("snow");
    case "mist":
    case "atmospheric":
        return weatherFrame("mist");
    case "fog":
        return weatherFrame("fog");
    default:
        return weatherFrame("unknown");
    }
}
