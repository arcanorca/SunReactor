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

/*! The Game Boy DMG (theme `handheld`): the one theme whose palette is a
    four-shade monochrome LCD ramp. Its weather art is drawn in those shades only. */
function isMonoRamp(theme) {
    return !!theme && theme.available === true && theme.themeId === "handheld"
        && (theme.shades || []).length === 4;
}

/*! Atlas tone (tools/generate_weather_icons.py) -> shade step on the ramp, 0 darkest
    .. 3 lightest, by luminance: highlight and warm core light, body mid, shade dark,
    edges, water, outline and rays darkest. Tone 0 (transparent) is never looked up. */
var MONO_TONE = [0, 3, 2, 1, 0, 3, 0, 0, 0, 0];
/*! Where one tone stands for two things, the map differs per frame: the sun ring is
    dark (tone 1), the snow flakes are dark (tone 1 is also the cloud's top), and the
    thunderstorm bolt is dark where the sun's core is light (tone 5). */
var MONO_OVERRIDE = {
    "clear-day": { 1: 0 },
    "snow": { 1: 0 },
    "thunderstorm": { 5: 0 }
};

/*! PixelSprite colours for the atlas frame or symbol `name` on the ramp `shades`
    (darkest first): index i is the colour of tone i. */
function monoColors(name, shades) {
    const over = MONO_OVERRIDE[name] || {};
    const out = ["transparent"];
    for (let tone = 1; tone < MONO_TONE.length; ++tone) {
        const step = over[tone] !== undefined ? over[tone] : MONO_TONE[tone];
        out.push(shades[step]);
    }
    return out;
}

/*! monoColors for the weather frame index `frame` (see WEATHER). */
function monoWeatherColors(frame, shades) {
    return monoColors(WEATHER[frame] || "unknown", shades);
}
