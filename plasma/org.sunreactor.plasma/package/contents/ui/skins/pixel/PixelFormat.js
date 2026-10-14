.pragma library

/*
 * Compact, locale-aware readouts for the pixel skins, where every glyph costs
 * six art pixels: "14:30" or "2:30PM" (no space before the meridiem), and the
 * pieces separately for seven-segment clocks that show the meridiem as an
 * indicator next to the digits.
 */

function twelveHour() {
    // Locale.ShortFormat == 1; the enum is not reachable from a library script.
    const fmt = Qt.locale().timeFormat(1);
    return /a/i.test(fmt.replace(/'[^']*'/g, ""));
}

/*! {hm: "2:30", meridiem: "PM"} or {hm: "14:30", meridiem: ""}; empty if unknown. */
function timeParts(epochSeconds) {
    if (!epochSeconds || epochSeconds <= 0) {
        return { hm: "", meridiem: "" };
    }
    const date = new Date(epochSeconds * 1000);
    if (twelveHour()) {
        const h = date.getHours();
        const hm = ((h + 11) % 12 + 1) + ":" + String(date.getMinutes()).padStart(2, "0");
        return { hm: hm, meridiem: h < 12 ? Qt.locale().amText : Qt.locale().pmText };
    }
    return { hm: Qt.formatTime(date, "HH:mm"), meridiem: "" };
}

/*! "14:30" / "2:30PM". */
function time(epochSeconds) {
    const p = timeParts(epochSeconds);
    return p.hm + p.meridiem;
}

/*! Hour label for an axis: "06" / "6A"; h is 0..24. */
function hour(h) {
    if (twelveHour()) {
        const hh = h % 24;
        if (hh === 0) {
            return "12A";
        }
        if (hh === 12) {
            return "12P";
        }
        return (hh % 12) + (hh < 12 ? "A" : "P");
    }
    return String(h).padStart(2, "0");
}

/*! Short display tag: "MI", "LEN" - the first word of the id, at most 4 letters. */
function shortName(logicalId) {
    const head = String(logicalId).split(/[-_ .]/)[0] || String(logicalId);
    return head.slice(0, 4).toUpperCase();
}
