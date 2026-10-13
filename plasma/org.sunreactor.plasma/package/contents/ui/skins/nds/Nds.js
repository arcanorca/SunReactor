.pragma library

/*
 * Geometry of the Nintendo DS skin, in art pixels (DESIGN.md section 4): the
 * dark bezel of each screen, the hinge between the two screens, the light-blue
 * menu background with its bubbles, the battery lamp and the stylus cursor.
 * Every function appends {x, y, w, h, c} rectangles on the art grid to `out`
 * and returns it. Colours are passed in as theme tokens, so nothing here is a
 * literal colour.
 */

/*! Dark bezel around every screen. */
var BEZEL = 2;
/*! The hinge strip under the top screen. */
var HINGE = 9;
/*! The top screen on the brightness page, lid and hinge together. */
var TOP_H = 87;
/*! The top screen on the weather page. */
var WEATHER_TOP_H = 59;
/*! The status strip above the menu, and the folder tab row. */
var TELEMETRY_H = 9;
var TABS_H = 15;

/*! The stylus tip: an X is a lit pixel of the cursor. */
var CURSOR = ["X....", "XX...", "XXX..", "XXXX.", "XXXXX"];

function push(out, x, y, w, h, c) {
    if (w > 0 && h > 0) {
        out.push({ x: x, y: y, w: w, h: h, c: String(c) });
    }
    return out;
}

/*! Filled disc of radius r centred on (cx, cy): one rectangle per row. */
function disc(out, cx, cy, r, c) {
    for (let dy = -r; dy <= r; ++dy) {
        const half = Math.floor(Math.sqrt(r * r - dy * dy) + 0.5);
        push(out, cx - half, cy + dy, 2 * half + 1, 1, c);
    }
    return out;
}

/*! The menu's bubbles over an area: discs on a staggered diagonal lattice. */
function bubbles(out, x0, y0, x1, y1, c) {
    let row = 0;
    for (let y = y0 + 3; y < y1 - 2; y += 10, ++row) {
        for (let x = x0 + ((row * 7) % 19); x < x1 - 2; x += 19) {
            disc(out, x, y, (row + x) % 3 === 0 ? 2 : 1, c);
        }
    }
    return out;
}

/*! A tile: filled, with a 1 px edge whose corners are left to the fill (rounded). */
function box(out, x, y, w, h, fill, edge) {
    push(out, x, y, w, h, fill);
    push(out, x + 1, y, w - 2, 1, edge);
    push(out, x + 1, y + h - 1, w - 2, 1, edge);
    push(out, x, y + 1, 1, h - 2, edge);
    push(out, x + w - 1, y + 1, 1, h - 2, edge);
    return out;
}

/*! The hinge strip at y (HINGE rows): a plastic bar, a speaker grille, the power
    LED on the left and the wireless LED on the right. col: plastic, dot, power, wifi. */
function hinge(out, W, y, col) {
    const cx = Math.floor(W / 2);
    push(out, 1, y + 2, W - 2, HINGE - 4, col.plastic);
    push(out, 3, y + 1, W - 6, HINGE - 2, col.plastic);
    for (let i = 0; i < 6; ++i) {
        push(out, cx - 5 + 2 * i, y + 3, 1, 1, col.dot);
        push(out, cx - 5 + 2 * i, y + 5, 1, 1, col.dot);
    }
    disc(out, 6, y + 4, 1, col.power);
    disc(out, W - 7, y + 4, 1, col.wifi);
    return out;
}

/*! A lid of W x H: bubbles, then the dark bezel, then the hinge when asked.
    col: bubble, bezel, plastic, dot, power, wifi. */
function lid(out, W, H, hingeOn, col) {
    bubbles(out, BEZEL, BEZEL, W - BEZEL, H - BEZEL, col.bubble);
    push(out, 0, 0, W, BEZEL, col.bezel);
    push(out, 0, H - BEZEL, W, BEZEL, col.bezel);
    push(out, 0, BEZEL, BEZEL, H - 2 * BEZEL, col.bezel);
    push(out, W - BEZEL, BEZEL, BEZEL, H - 2 * BEZEL, col.bezel);
    if (hingeOn) {
        hinge(out, W, H, col);
    }
    return out;
}

/*! Battery lamp, 10 x 5 with a nub: `lit` of three cells in col.lit, the rest
    in col.faint. col: frame, lit, faint. */
function battery(out, x, y, lit, col) {
    push(out, x, y, 10, 1, col.frame);
    push(out, x, y + 4, 10, 1, col.frame);
    push(out, x, y, 1, 5, col.frame);
    push(out, x + 9, y, 1, 5, col.frame);
    push(out, x + 10, y + 1, 1, 3, col.frame);
    for (let i = 0; i < 3; ++i) {
        push(out, x + 1 + 3 * i, y + 1, 2, 3, i < lit ? col.lit : col.faint);
    }
    return out;
}

/*! The stylus-tip cursor with its tip at (x, y). */
function cursor(out, x, y, c) {
    for (let r = 0; r < CURSOR.length; ++r) {
        for (let k = 0; k < CURSOR[r].length; ++k) {
            if (CURSOR[r][k] === "X") {
                push(out, x + k, y + r, 1, 1, c);
            }
        }
    }
    return out;
}
