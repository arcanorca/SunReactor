.pragma library

/*
 * Turns a small picture (rows of characters, each row paired with a row of
 * colour roles) into drawing primitives on the cell grid, in device px.
 *
 * Block elements are never drawn as glyphs: the font's full and half blocks do
 * not fill a cell height exactly, so stacked rows show seams. Quadrants,
 * eighth-height blocks and full cells become rectangles aligned to the device
 * pixel grid; shades (░ ▒ ▓) become a tint of the role colour. Every other
 * character stays a glyph, drawn at its cell.
 *
 * Roles: a single character per cell, chosen by the caller (a space or "" in
 * the role row means the cell is not drawn).
 */

/*! Quadrant masks: bit 0 top-left, 1 top-right, 2 bottom-left, 3 bottom-right. */
const QUADRANTS = {
    "▘": 1, "▝": 2, "▖": 4, "▗": 8,
    "▀": 3, "▄": 12, "▌": 5, "▐": 10,
    "▚": 9, "▞": 6,
    "▙": 13, "▟": 14, "▛": 7, "▜": 11,
    "█": 15,
};

/*! Lower eighth blocks: ▁ (1/8 high) .. ▇ (7/8), and █ as 8/8. */
const EIGHTHS = { "▁": 1, "▂": 2, "▃": 3, "▄": 4, "▅": 5, "▆": 6, "▇": 7, "█": 8 };

/*! Shade blocks: the share of the role colour. */
const SHADES = { "░": 0.3, "▒": 0.55, "▓": 0.8 };

/*! Width in cells of the widest row. */
function cols(rows) {
    let widest = 0;
    for (const row of rows) {
        widest = Math.max(widest, Array.from(row).length);
    }
    return widest;
}

/*!
 * Primitives for a picture: `rects` as { x, y, w, h, role, shade } in device
 * px (shade 1 = solid), and `texts` as { col, row, value, role }.
 */
function prims(rows, roles, cwDev, chDev) {
    const rects = [];
    const texts = [];
    // Quadrant sizes: the odd device pixel goes to the right and bottom halves.
    const halfW = Math.floor(cwDev / 2);
    const halfH = Math.floor(chDev / 2);
    const rightW = cwDev - halfW;
    const bottomH = chDev - halfH;

    for (let r = 0; r < rows.length; ++r) {
        const line = Array.from(rows[r]);
        const roleLine = Array.from(roles[r] || "");
        for (let c = 0; c < line.length; ++c) {
            const ch = line[c];
            const role = roleLine[c] || "";
            if (ch === " " || role === "" || role === " ") {
                continue;
            }
            const x0 = c * cwDev;
            const y0 = r * chDev;
            if (ch === "█") {
                rects.push({ x: x0, y: y0, w: cwDev, h: chDev, role: role, shade: 1 });
            } else if (QUADRANTS[ch] !== undefined) {
                const mask = QUADRANTS[ch];
                if (mask & 1) rects.push({ x: x0, y: y0, w: halfW, h: halfH, role: role, shade: 1 });
                if (mask & 2) rects.push({ x: x0 + halfW, y: y0, w: rightW, h: halfH, role: role, shade: 1 });
                if (mask & 4) rects.push({ x: x0, y: y0 + halfH, w: halfW, h: bottomH, role: role, shade: 1 });
                if (mask & 8) rects.push({ x: x0 + halfW, y: y0 + halfH, w: rightW, h: bottomH, role: role, shade: 1 });
            } else if (EIGHTHS[ch] !== undefined) {
                const h = Math.round(EIGHTHS[ch] * chDev / 8);
                rects.push({ x: x0, y: y0 + chDev - h, w: cwDev, h: h, role: role, shade: 1 });
            } else if (SHADES[ch] !== undefined) {
                rects.push({ x: x0, y: y0, w: cwDev, h: chDev, role: role, shade: SHADES[ch] });
            } else {
                texts.push({ col: c, row: r, value: ch, role: role });
            }
        }
    }
    return { rects: rects, texts: texts };
}
