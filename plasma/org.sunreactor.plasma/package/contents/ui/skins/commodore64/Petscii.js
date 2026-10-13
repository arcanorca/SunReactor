.pragma library

/*
 * Text and symbol helpers for the Commodore 64 skin. The "petscii" face
 * (plugin/glyphtables.cpp) draws exactly the characters in ALLOWED once they
 * are upper-cased: the C64 screen capitals, digits, the symbols the skin uses
 * and the graphics (box lines, half / eighth / full blocks, shade blocks,
 * bullets and triangles). fold() maps anything else onto them, so every label
 * has a glyph before layout and its width is known in cells.
 */

const ALLOWED = /[^A-Z0-9 !"#$%&'()*+,\-.\/:;<=>?@\[\\\]_°·…─│┌┐└┘▌▐▀▄█▒░▁▂▃▄▅▆▇●♥◤◥◢◣►◄]/g;

/*! Upper case, accents folded away (Turkish, German), punctuation mapped, the
 *  rest replaced by "?". One character is one cell, so the result is safe to measure. */
function fold(text) {
    let s = String(text === undefined || text === null ? "" : text).toUpperCase();
    s = s.replace(/[\u2013\u2014]/g, "-").replace(/[\u2018\u2019]/g, "'").replace(/[\u201C\u201D]/g, "\"");
    s = s.replace(/\u2192/g, ">").replace(/\u2190/g, "<").replace(/\u2191/g, "^").replace(/\u2193/g, "v").replace(/[\u2022\u2219\u22C5]/g, "\u00B7");
    s = s.normalize("NFD").replace(/[\u0300-\u036f]/g, "");
    return s.replace(ALLOWED, "?");
}

/*! Shortens to n cells with a visible "." when it does not fit. */
function fit(text, n) {
    const s = String(text);
    if (n <= 0) {
        return "";
    }
    if (s.length <= n) {
        return s;
    }
    return s.slice(0, Math.max(0, n - 1)) + ".";
}

/*! Eighths of a cell filled from the bottom: 0 blank .. 8 full block. */
const BLOCKS = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

function blocks(eighths) {
    return BLOCKS[Math.max(0, Math.min(8, Math.round(eighths)))];
}

/*! Linear interpolation of samples {t, pct, noWeather} at time t; key picks the field. */
function valueAt(samples, t, key) {
    const n = samples.length;
    if (n === 0) {
        return 0;
    }
    if (t <= samples[0].t) {
        return samples[0][key];
    }
    for (let i = 1; i < n; ++i) {
        const a = samples[i - 1];
        const b = samples[i];
        if (t <= b.t) {
            const span = b.t - a.t;
            return span > 0 ? a[key] + (b[key] - a[key]) * (t - a.t) / span : b[key];
        }
    }
    return samples[n - 1][key];
}

/*! Sun, moon, cloud and rain sprites: rows of graphics characters, at most 5 cells wide, 3 rows. */
const SUN = ["\\│/", "─●─", "/│\\"];
const MOON = [" ▄▄", "▐█ ", " ▀▀"];
const CLOUD = [" ▄▄▄ ", "▐███▌", "▀▀▀▀▀"];

function sprite(condition, night) {
    switch (condition) {
    case "clear":
        return night ? MOON : SUN;
    case "partly_cloudy":
        return night ? ["▄▄ ", "▐██▌", "▀▀▀▀"] : ["\\│  ", "─●▄▄", "/▐██▌"];
    case "cloudy":
        return CLOUD;
    case "drizzle":
    case "rain":
    case "heavy_rain":
        return ["▐███▌", "▀▀▀▀▀", "/ / /"];
    case "thunderstorm":
        return ["▐███▌", "▀▀▀▀▀", " /!/ "];
    case "snow":
        return ["* * *", " *** ", "* * *"];
    case "mist":
    case "fog":
    case "atmospheric":
        return ["=====", " === ", "====="];
    default:
        return ["  ?  ", "     ", "     "];
    }
}

/*! One-cell icon for a forecast slot. */
function icon(condition, night) {
    switch (condition) {
    case "clear":
        return night ? "·" : "●";
    case "partly_cloudy":
        return "◢";
    case "cloudy":
        return "▒";
    case "drizzle":
    case "rain":
    case "heavy_rain":
        return "/";
    case "thunderstorm":
        return "!";
    case "snow":
        return "*";
    case "mist":
    case "fog":
    case "atmospheric":
        return "=";
    default:
        return "?";
    }
}
