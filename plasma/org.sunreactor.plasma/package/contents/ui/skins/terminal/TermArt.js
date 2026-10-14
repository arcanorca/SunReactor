.pragma library

/*
 * Pictures for the weather panel, as { rows, roles } for TermPicture:
 *   - bigText(): the temperature in a 5-row block numeral font (0-9 . - ° C);
 *   - weatherArt(): a 7-row sky picture per condition, day and night;
 *   - weatherIcon(): a one-row symbol for a forecast column.
 * Roles: o sun/accent, h secondary (rain, moon), c cloud/fg, m muted (fog,
 * stars), y warning (lightning), w bright (snow), d dim (no reading).
 * Only glyphs JetBrains Mono covers are used, so no run falls back to another
 * font and every cell keeps the grid's advance.
 */

/*! 5-row numerals on full cells: "█" is ink, " " is paper. */
const DIGITS = {
    "0": ["███", "█ █", "█ █", "█ █", "███"],
    "1": ["██ ", " █ ", " █ ", " █ ", "███"],
    "2": ["███", "  █", "███", "█  ", "███"],
    "3": ["███", "  █", "███", "  █", "███"],
    "4": ["█ █", "█ █", "███", "  █", "  █"],
    "5": ["███", "█  ", "███", "  █", "███"],
    "6": ["███", "█  ", "███", "█ █", "███"],
    "7": ["███", "  █", "  █", "  █", "  █"],
    "8": ["███", "█ █", "███", "█ █", "███"],
    "9": ["███", "█ █", "███", "  █", "███"],
    ".": [" ", " ", " ", " ", "█"],
    "-": ["  ", "  ", "██", "  ", "  "],
    "°": ["▄▄", "▀▀", "  ", "  ", "  "],
    "C": ["███", "█  ", "█  ", "█  ", "███"],
};

/*! Rows of one numeral string, one glyph gap between glyphs. */
function bigText(text) {
    const rows = ["", "", "", "", ""];
    const roles = ["", "", "", "", ""];
    const chars = Array.from(String(text));
    chars.forEach((ch, i) => {
        const glyph = DIGITS[ch];
        if (!glyph) {
            return;
        }
        const width = Math.max(...glyph.map(row => Array.from(row).length));
        for (let r = 0; r < 5; ++r) {
            rows[r] += Array.from(glyph[r].padEnd(width, " ")).join("");
            roles[r] += Array.from(glyph[r].padEnd(width, " ")).map(c => c === " " ? " " : "o").join("");
        }
        if (i < chars.length - 1) {
            for (let r = 0; r < 5; ++r) {
                rows[r] += " ";
                roles[r] += " ";
            }
        }
    });
    return { rows: rows, roles: roles };
}

// Sky pictures, 7 rows, each up to 13 cells. Rows are padded to a common width
// when they are drawn, so a row may be shorter than its neighbours.
const SKY = {
    clear_day: {
        rows: [
            "      │      ",
            "  ╲   │   ╱  ",
            "    ╲ │ ╱    ",
            "───── ◉ ─────",
            "    ╱ │ ╲    ",
            "  ╱   │   ╲  ",
            "      │      ",
        ],
        top: "o", bottom: "o", split: 7,
    },
    clear_night: {
        rows: [
            "   ███    ",
            "  ███     ",
            " ███      ",
            " ███   ·  ",
            " ███      ",
            "  ███   · ",
            "   ███    ",
        ],
        top: "h", bottom: "h", split: 7,
    },
    partly_cloudy: {
        rows: [
            " ╲  │      ",
            "──  ◉      ",
            " ╱         ",
            "     ▗▄▄▄▖ ",
            "   ▗█████▙▖",
            "  ▐███████▌",
            "   ▝▀▀▀▀▀▀▘",
        ],
        top: "o", bottom: "c", split: 3,
    },
    cloudy: {
        rows: [
            "             ",
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            "▐███████████▌",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            "             ",
        ],
        top: "c", bottom: "c", split: 7,
    },
    drizzle: {
        rows: [
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            "  ·   ·   ·  ",
            "   ·   ·   · ",
            "  ·   ·   ·  ",
        ],
        top: "c", bottom: "h", split: 4,
    },
    rain: {
        rows: [
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            "  ╱   ╱   ╱  ",
            "   ╱   ╱   ╱ ",
            "  ╱   ╱   ╱  ",
        ],
        top: "c", bottom: "h", split: 4,
    },
    heavy_rain: {
        rows: [
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            " ╱╱ ╱╱ ╱╱ ╱╱ ",
            " ╱╱ ╱╱ ╱╱ ╱╱ ",
            " ╱╱ ╱╱ ╱╱ ╱╱ ",
        ],
        top: "c", bottom: "h", split: 4,
    },
    thunderstorm: {
        rows: [
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            "     ▄██     ",
            "    ██▀      ",
            "     ▀▀      ",
        ],
        top: "c", bottom: "y", split: 4,
    },
    snow: {
        rows: [
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            " ✶   ✶   ✶   ",
            "   ✶   ✶   ✶ ",
            " ✶   ✶   ✶   ",
        ],
        top: "c", bottom: "w", split: 4,
    },
    mist: {
        rows: [
            "             ",
            " ≋≋≋≋≋≋≋≋≋≋≋ ",
            "             ",
            "≋≋≋≋≋≋≋≋≋≋≋≋≋",
            "             ",
            " ≋≋≋≋≋≋≋≋≋≋≋ ",
            "             ",
        ],
        top: "m", bottom: "m", split: 7,
    },
    fog: {
        rows: [
            "             ",
            "≋≋≋≋≋≋≋≋≋≋≋≋≋",
            " ≋≋≋≋≋≋≋≋≋≋≋ ",
            "≋≋≋≋≋≋≋≋≋≋≋≋≋",
            " ≋≋≋≋≋≋≋≋≋≋≋ ",
            "≋≋≋≋≋≋≋≋≋≋≋≋≋",
            "             ",
        ],
        top: "m", bottom: "m", split: 7,
    },
    unknown: {
        rows: [
            "             ",
            "    ▗▄▄▄▖    ",
            "  ▗█████▙▖   ",
            " ▐█████████▌ ",
            " ▝▀▀▀▀▀▀▀▀▀▘ ",
            "             ",
            "             ",
        ],
        top: "d", bottom: "d", split: 7,
    },
};

/*! Which sky picture a condition token draws. */
function skyKey(condition, night) {
    switch (condition) {
    case "clear":
        return night ? "clear_night" : "clear_day";
    case "partly_cloudy":
        return night ? "clear_night" : "partly_cloudy";
    case "cloudy":
        return "cloudy";
    case "drizzle":
        return "drizzle";
    case "rain":
        return "rain";
    case "heavy_rain":
        return "heavy_rain";
    case "thunderstorm":
        return "thunderstorm";
    case "snow":
        return "snow";
    case "mist":
        return "mist";
    case "fog":
    case "atmospheric":
        return "fog";
    default:
        return "unknown";
    }
}

/*! The 7-row sky picture for a condition: { rows, roles } padded to one width.
    Rows above `split` take the `top` role, the rest the `bottom` role. */
function weatherArt(condition, night) {
    const art = SKY[skyKey(condition, night)];
    const width = Math.max(...art.rows.map(row => Array.from(row).length));
    const rows = art.rows.map(row => Array.from(row).join("").padEnd(width, " "));
    const roles = rows.map((row, r) => {
        const role = r < art.split ? art.top : art.bottom;
        return Array.from(row).map(ch => ch === " " ? " " : role).join("");
    });
    return { rows: rows, roles: roles };
}

/*! A one-row symbol for a forecast column, at most 3 cells: { rows, roles }. */
function weatherIcon(condition, night) {
    let text = "·";
    let role = "d";
    switch (condition) {
    case "clear":
        text = night ? "◌" : "✶";
        role = night ? "h" : "o";
        break;
    case "partly_cloudy":
        text = "✶▗█▖";
        role = "oc";
        break;
    case "cloudy":
        text = "▗██▖";
        role = "c";
        break;
    case "drizzle":
        text = "·╱·";
        role = "h";
        break;
    case "rain":
        text = "╱╱╱";
        role = "h";
        break;
    case "heavy_rain":
        text = "▞▞▞";
        role = "h";
        break;
    case "thunderstorm":
        text = "▄▀▄";
        role = "y";
        break;
    case "snow":
        text = "✶·✶";
        role = "w";
        break;
    case "mist":
    case "fog":
    case "atmospheric":
        text = "≋≋≋";
        role = "m";
        break;
    default:
        break;
    }
    const chars = Array.from(text);
    // Multi-role icons: the first role for the leading glyph, the second after.
    const roles = role.length === 2
        ? role[0] + chars.slice(1).map(() => role[1]).join("")
        : chars.map(() => role).join("");
    return { rows: [text], roles: [roles] };
}

/*! Sparkline levels for `count` cells, interpolated between the samples,
    each sample at the centre of its share of the strip. Returns a string of
    ▁..█ (or ▄ when every sample is equal). */
function sparkline(values, count, centres) {
    if (values.length === 0 || count <= 0) {
        return "";
    }
    const lo = Math.min(...values);
    const hi = Math.max(...values);
    const blocks = "▁▂▃▄▅▆▇█";
    let out = "";
    for (let x = 0; x < count; ++x) {
        let v;
        if (values.length === 1 || x <= centres[0]) {
            v = values[0];
        } else if (x >= centres[centres.length - 1]) {
            v = values[values.length - 1];
        } else {
            let j = 0;
            while (j < centres.length - 2 && x > centres[j + 1]) {
                ++j;
            }
            const span = Math.max(1, centres[j + 1] - centres[j]);
            const f = (x - centres[j]) / span;
            v = values[j] + (values[j + 1] - values[j]) * f;
        }
        const level = hi > lo ? Math.round((v - lo) / (hi - lo) * 7) : 3;
        out += blocks[Math.max(0, Math.min(7, level))];
    }
    return out;
}
