.pragma library

/*
 * Text the hackergreen skin draws: ASCII weather glyphs, the static code-rain
 * texture behind the light cycle, and the bracketed status flags. Pure functions
 * of their arguments; nothing here reads the daemon or keeps state.
 */

/*! Four rows of nine cells per condition token (the names of WeatherGlyphs.js). */
var ART = {
    clear:               ["  \\ | /  ", "-- (O) --", "  / | \\  ", "         "],
    night_clear:         ["   ,--.  ", "  (  .-' ", "  (  `-. ", "   `--'  "],
    partly_cloudy:       ["  \\  /   ", "_ /\"\".-. ", "  \\_(  ).", "  /(__(_)"],
    night_partly_cloudy: ["  ,-.    ", " (  `.-. ", "  `-(  ).", "   (__(_)"],
    cloudy:              ["   .--.  ", ".-(    ).", "(__.__)_)", "         "],
    drizzle:             ["   .--.  ", ".-(    ).", "(__.__)_)", "  ' ' '  "],
    rain:                ["   .--.  ", ".-(    ).", "(__.__)_)", "  / / /  "],
    heavy_rain:          ["   .--.  ", ".-(    ).", "(__.__)_)", " / / / / "],
    thunderstorm:        ["   .--.  ", ".-(    ).", "(__.__)_)", "   /_/   "],
    snow:                ["   .--.  ", ".-(    ).", "(__.__)_)", "  * * *  "],
    mist:                [" ~~~~~~~ ", "  ~~~~~  ", " ~~~~~~~ ", "  ~~~~~  "],
    fog:                 [" ======= ", "  =====  ", " ======= ", "  =====  "],
    atmospheric:         [" ~ ~ ~ ~ ", "  ~ ~ ~  ", " ~ ~ ~ ~ ", "  ~ ~ ~  "],
    unknown:             ["         ", "         ", "    ?    ", "         "]
};

/*! The four art rows for a condition ("" or unknown tokens get the question mark). */
function art(condition, night) {
    const key = night && (condition === "clear" || condition === "partly_cloudy")
        ? "night_" + condition
        : condition;
    return ART.hasOwnProperty(key) ? ART[key] : ART.unknown;
}

/*! Three rows of seven cells per token: the forecast columns' compact sky. */
var ART_COMPACT = {
    clear:               [" \\ | / ", "-- O --", " / | \\ "],
    night_clear:         ["  ,-.  ", " (  .' ", "  `-'  "],
    partly_cloudy:       [" \\ .-. ", "-O(   )", " /(___)"],
    night_partly_cloudy: [" , .-. ", "( (   )", "`'(___)"],
    cloudy:              ["  .-.  ", ".-(  ).", "(__.__)"],
    drizzle:             ["  .-.  ", "(__.__)", " ' ' ' "],
    rain:                ["  .-.  ", "(__.__)", " / / / "],
    heavy_rain:          ["  .-.  ", "(__.__)", "/ / / /"],
    thunderstorm:        ["  .-.  ", "(__.__)", "  /_/  "],
    snow:                ["  .-.  ", "(__.__)", " * * * "],
    mist:                [" ~~~~~ ", "~~~~~~~", " ~~~~~ "],
    fog:                 [" ===== ", "=======", " ===== "],
    atmospheric:         [" ~ ~ ~ ", "~ ~ ~ ~", " ~ ~ ~ "],
    unknown:             ["       ", "   ?   ", "       "]
};

/*! The three compact rows for a condition (same routing as art()). */
function artCompact(condition, night) {
    const key = night && (condition === "clear" || condition === "partly_cloudy")
        ? "night_" + condition
        : condition;
    return ART_COMPACT.hasOwnProperty(key) ? ART_COMPACT[key] : ART_COMPACT.unknown;
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
