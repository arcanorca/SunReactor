.pragma library

/*
 * Two families of weather icons, for the two uses they have:
 *
 *   sprites    16x16 pixel art by generate_weather_icons.py, shipped with the
 *              package. Only the "pixel" weather icon style draws them.
 *   themeIcon  Breeze symbolic names. The widget's own icon (Plasmoid.icon,
 *              shown in the widget explorer and the tray fallback) uses these,
 *              so it stays a generic, recolourable icon.
 *
 * Paths resolve against this file, which lives in contents/ui/.
 */

function art(name) {
    return Qt.resolvedUrl("../icons/weather/" + name + ".svg");
}

/// Breeze symbolic icon for a daemon condition token, honouring day and night.
/// "" (no reading, not connected) is the unknown icon.
function themeIcon(condition, isNight) {
    switch (condition) {
    case "clear":
        return isNight ? "weather-clear-night-symbolic" : "weather-clear-symbolic";
    case "partly_cloudy":
        return isNight ? "weather-few-clouds-night-symbolic" : "weather-few-clouds-symbolic";
    case "cloudy":
        return "weather-clouds-symbolic";
    case "drizzle":
        return "weather-showers-scattered-symbolic";
    case "rain":
    case "heavy_rain":
        return "weather-showers-symbolic";
    case "thunderstorm":
        return "weather-storm-symbolic";
    case "snow":
        return "weather-snow-symbolic";
    case "mist":
    case "atmospheric":
    case "fog":
        return "weather-fog-symbolic";
    default:
        return "weather-none-available-symbolic";
    }
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
