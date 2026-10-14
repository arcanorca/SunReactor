.pragma library

/*
 * Text the hackergreen skin draws: ASCII weather glyphs, the static code-rain
 * texture behind the light cycle, and the bracketed status flags. Pure functions
 * of their arguments; nothing here reads the daemon or keeps state.
 */

/*! Three rows of seven cells per condition token (the names of WeatherGlyphs.js). */
var ART = {
    clear: [" \\ | / ", "-- O --", " / | \\ "],
    night_clear: ["  .-.  ", " (   ) ", "  `-'  "],
    partly_cloudy: [" \\ |   ", " -(   )", "  (___)"],
    cloudy: ["  .-.  ", " (   ) ", "(_____)"],
    drizzle: ["  .-.  ", " (   ) ", " . . . "],
    rain: ["  .-.  ", " (   ) ", " / / / "],
    heavy_rain: ["  .-.  ", " (   ) ", "/ / / /"],
    thunderstorm: ["  .-.  ", " (   ) ", "  /\\/  "],
    snow: ["  .-.  ", " (   ) ", " * * * "],
    mist: [" ~~~~~ ", "~~~~~~~", " ~~~~~ "],
    fog: ["=======", " ===== ", "======="],
    atmospheric: [" ~ ~ ~ ", "~ ~ ~ ~", " ~ ~ ~ "],
    unknown: ["       ", "   ?   ", "       "]
};

/*! The three art rows for a condition ("" or unknown tokens get the question mark). */
function art(condition, night) {
    const key = condition === "clear" && night ? "night_clear" : condition;
    return ART.hasOwnProperty(key) ? ART[key] : ART.unknown;
}

/*! Glyphs of the code texture: digits and symbols of the bundled mono font. */
var GLYPHS = "0123456789+-*/=<>|:;.~^%&#$";

/*! A deterministic value in [0, 1) for two integers (a fixed hash, no state). */
function unit(a, b) {
    let h = (Math.imul(a, 374761393) + Math.imul(b, 668265263)) | 0;
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/*!
 * `rows` strings of `cols` cells: falling streams of code, frozen. Each column
 * carries at most one trail (its head row and length are fixed by the column),
 * with gaps, so the texture reads as rain without any motion. The result depends
 * only on the size, so it is computed once per size.
 */
function rain(cols, rows) {
    const out = [];
    for (let r = 0; r < rows; ++r) {
        let line = "";
        for (let c = 0; c < cols; ++c) {
            let glyph = " ";
            if (unit(c, 9) < 0.6) {
                const head = Math.floor(unit(c, 3) * rows);
                const length = 1 + Math.floor(unit(c, 5) * Math.max(1, rows - 1));
                const above = (head - r + rows) % rows;
                if (above <= length && unit(c * 131 + r, 17) < 0.8) {
                    glyph = GLYPHS.charAt(Math.floor(unit(c, r + 101) * GLYPHS.length));
                }
            }
            line += glyph;
        }
        out.push(line);
    }
    return out;
}

/*! "[ LABEL ]" in the flag colour of `tone` ("ok", "warn", "err" or "dim"). */
function flag(theme, tone, label) {
    const colour = tone === "err" ? theme.error
        : tone === "warn" ? theme.warning
        : tone === "dim" ? theme.textMuted
        : theme.success;
    return { t: "[ " + label + " ]", c: colour, b: true };
}
