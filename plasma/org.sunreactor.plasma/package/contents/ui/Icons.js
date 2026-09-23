.pragma library

/*
 * Weather sprites, drawn as 16x16 pixel art by generate_weather_icons.py and
 * shipped with the package. They share the palette and the shading of the
 * terminal cockpit, so both surfaces look like the same product.
 *
 * Paths resolve against this file, which lives in contents/ui/.
 */

function art(name) {
    return Qt.resolvedUrl("../icons/weather/" + name + ".svg");
}

/// Sun or moon, for when there is no weather reading to show.
function skyArt(isNight) {
    return art(isNight ? "clear-night" : "clear-day");
}

/// Maps a daemon condition token to a sprite, honouring day and night.
function conditionArt(condition, isNight) {
    switch (condition) {
    case "clear":
        return skyArt(isNight);
    case "partly_cloudy":
        return art(isNight ? "partly-cloudy-night" : "partly-cloudy-day");
    case "cloudy":
        return art("cloudy");
    case "drizzle":
        return art("drizzle");
    case "rain":
        return art("rain");
    case "heavy_rain":
        return art("heavy-rain");
    case "thunderstorm":
        return art("thunderstorm");
    case "snow":
        return art("snow");
    case "mist":
    case "atmospheric":
        return art("mist");
    case "fog":
        return art("fog");
    default:
        return art("unknown");
    }
}
