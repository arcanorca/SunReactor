.pragma library

/*
 * Cell-string helpers for the terminal skins. Every string drawn on the grid
 * is measured in cells (one code point = one cell; the skins only use
 * single-width glyphs), so columns line up exactly like the TUI's
 * pad_to / pad_left / truncate (src/tui/ui/kit.rs).
 */

/*! Width of `text` in cells. */
function len(text) {
    return Array.from(String(text)).length;
}

/*! `text` cut to `cells`, ending in "…" when it had to be shortened. */
function fit(text, cells) {
    const chars = Array.from(String(text));
    if (cells <= 0) {
        return "";
    }
    if (chars.length <= cells) {
        return chars.join("");
    }
    return chars.slice(0, Math.max(0, cells - 1)).join("") + "…";
}

/*! The first of `forms` (longest first) that fits `cells`; the last one when
    none does. Fixed labels are shortened this way, never cut with "…". */
function firstFit(forms, cells) {
    for (const form of forms) {
        if (len(form) <= cells) {
            return form;
        }
    }
    return forms[forms.length - 1];
}

/*! `text` with its " · " segments dropped from the end (the least important,
    the sentence's order is importance) until it fits `cells`; the first
    segment always stays. */
function shedSegments(text, cells) {
    const parts = String(text).split(" · ");
    while (parts.length > 1 && len(parts.join(" · ")) > cells) {
        parts.pop();
    }
    return parts.join(" · ");
}

/*! Runs (TermRuns: { t, ... }) with the runs of the highest `shed` rank dropped
    first, a whole rank at a time, until they fit `cells`. Runs without a rank
    are kept: a label that must not go has none. */
function shedRuns(runs, cells) {
    const rankOf = run => run.shed || 0;
    const width = list => list.reduce((n, run) => n + len(run.t), 0);
    let out = runs.slice();
    let rank = Math.max(0, ...out.map(rankOf));
    while (width(out) > cells && rank > 0) {
        out = out.filter(run => rankOf(run) !== rank);
        rank = Math.max(0, ...out.map(rankOf));
    }
    return out;
}

/*! Left-aligned in exactly `cells` (cut or space-padded). */
function padEnd(text, cells) {
    const cut = fit(text, cells);
    return cut + " ".repeat(Math.max(0, cells - len(cut)));
}

/*! Right-aligned in exactly `cells` (cut or space-padded). */
function padStart(text, cells) {
    const cut = fit(text, cells);
    return " ".repeat(Math.max(0, cells - len(cut))) + cut;
}

/*! `count` copies of one glyph. */
function repeat(glyph, count) {
    return count > 0 ? String(glyph).repeat(count) : "";
}

/*! Short label for a display, from its logical id: the TUI's status line
    uses the first word of the display name ("mi-monitor" -> "Mi"). */
function shortName(logicalId) {
    const head = String(logicalId).split(/[-_ .]/)[0] || String(logicalId);
    return head.charAt(0).toUpperCase() + head.slice(1, 8);
}

/*! "11h 25m" for a span in seconds, "—" when unknown. */
function duration(seconds) {
    if (!(seconds > 0)) {
        return "—";
    }
    const minutes = Math.round(seconds / 60);
    return Math.floor(minutes / 60) + "h " + String(minutes % 60).padStart(2, "0") + "m";
}

function smoothstep(t) {
    const x = Math.max(0, Math.min(1, t));
    return x * x * (3 - 2 * x);
}

/*!
 * Phase bounds in seconds since local midnight, like the TUI's cycle_bounds:
 * [0, dawn, sunrise, solar noon, sunset, dusk, day length]. Missing times
 * (polar edges) collapse onto their neighbours so the phases stay ordered.
 */
function phaseBounds(sun, t0, t1) {
    const day = Math.max(1, t1 - t0);
    const at = (value, fallback) => value > 0 ? Math.max(0, Math.min(day, value - t0)) : fallback;
    const rise = at(sun && sun.sunrise, day * 0.3);
    const set = at(sun && sun.sunset, day * 0.75);
    const noon = at(sun && sun.solarNoon, (rise + set) / 2);
    const dawn = Math.min(rise, at(sun && sun.dawn, rise));
    const dusk = Math.max(set, at(sun && sun.dusk, set));
    return [0, dawn, rise, noon, set, dusk, day];
}

/*! Index of the phase containing `s` (seconds since midnight). */
function phaseAt(s, bounds) {
    for (let i = 0; i < 6; ++i) {
        if (s >= bounds[i] && s < bounds[i + 1]) {
            return i;
        }
    }
    return 5;
}

/*!
 * How much sky light there is at `s`, 0 (night) .. 1 (high sun), from the
 * sun times alone: the TUI derives it from the solar elevation, which the
 * widget does not have per minute. Twilight ramps to the horizon value
 * (0.375, i.e. elevation 0° on the TUI's -6°..10° scale), the day follows a
 * sine arch to noon.
 */
function skyLight(s, bounds) {
    const dawn = bounds[1], rise = bounds[2], set = bounds[4], dusk = bounds[5];
    let light = 0;
    if (s <= dawn || s >= dusk) {
        light = 0;
    } else if (s < rise) {
        light = 0.375 * (s - dawn) / Math.max(1, rise - dawn);
    } else if (s > set) {
        light = 0.375 * (dusk - s) / Math.max(1, dusk - set);
    } else {
        light = 0.375 + 0.625 * Math.sin(Math.PI * (s - rise) / Math.max(1, set - rise)) * 1.6;
    }
    return smoothstep(light);
}
