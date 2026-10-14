.pragma library

/*
 * Path strings for the cyberpunk skin. Every outline, hatch, cell row and tick
 * ruler is one path built here, so a skin draws each with one Shape instead of
 * one item per cell or line. Coordinates are logical px inside the drawing item.
 */

/*! "M x y L ... Z" through [x, y] points. */
function poly(points) {
    let d = "";
    for (let i = 0; i < points.length; ++i) {
        d += (i === 0 ? "M" : "L") + points[i][0] + " " + points[i][1] + " ";
    }
    return d + "Z ";
}

/*! A rectangle as a closed path. */
function rect(x, y, w, h) {
    return poly([[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
}

/*
 * A w x h box with its top-right and bottom-left corners cut by `cut`. `inset`
 * moves the edge inwards by that many px (half a stroke), so a stroke centred on
 * the path lies inside the box. The diagonal moves by inset * sqrt(2).
 */
function chamfer(w, h, cut, inset) {
    const k = inset * Math.SQRT2;
    return poly([
        [inset, inset],
        [w - cut - k + inset, inset],
        [w - inset, cut + k - inset],
        [w - inset, h - inset],
        [cut + k - inset, h - inset],
        [inset, h - cut - k + inset]
    ]);
}

/*
 * A title tab hanging from the top edge, `w` wide and `cut` tall. Its right edge
 * is the panel's top-right cut, so placed at the right edge it continues the cut.
 */
function tab(w, cut) {
    return poly([[0, 0], [w - cut, 0], [w, cut], [0, cut]]);
}

/*
 * Diagonal hatch inside the chamfered box: lines x - y = m, m stepping by `step`,
 * each clipped to the box and to both cut corners analytically (no clip item).
 */
function hatch(w, h, cut, step) {
    let d = "";
    for (let m = -(h - cut); m <= w - cut; m += step) {
        const x1 = Math.max(0, m);
        const x2 = Math.min(w, h + m);
        if (x2 > x1) {
            d += "M" + x1 + " " + (x1 - m) + " L" + x2 + " " + (x2 - m) + " ";
        }
    }
    return d;
}

/*
 * Hazard stripes filling a w x h box: 45 degree parallelograms, the stripe as
 * wide as the box is tall. The base colour shows between them. Clip the item.
 */
function hazard(w, h) {
    // Before layout the box is 0 high: no stripes, and no zero step in the loop.
    if (!(h > 0)) {
        return "";
    }
    let d = "";
    for (let a = -h; a < w + h; a += 2 * h) {
        d += poly([[a, 0], [a + h, 0], [a, h], [a - h, h]]);
    }
    return d;
}

/*
 * Slanted cells (parallelograms; the top edge is `skew` px to the right of the
 * bottom edge) for indices first..last-1 on a row of `pitch` px.
 */
function cells(first, last, pitch, cw, ch, skew) {
    let d = "";
    for (let i = first; i < last; ++i) {
        const x = i * pitch;
        d += poly([[x + skew, 0], [x + skew + cw, 0], [x + cw, ch], [x, ch]]);
    }
    return d;
}

/*! A downward-pointing triangle: top edge at y0, tip at y0 + height, centred on cx. */
function triangleDown(cx, y0, half, height) {
    return poly([[cx - half, y0], [cx + half, y0], [cx, y0 + height]]);
}

/*
 * A tick ruler from y = 0 down: n + 1 ticks spread over w, every `every`-th one
 * long (`longLen`), the others `shortLen`. every <= 0 makes them all short.
 */
function ticks(w, n, every, shortLen, longLen) {
    let d = "";
    for (let i = 0; i <= n; ++i) {
        const x = i * w / n;
        const len = (every > 0 && i % every === 0) ? longLen : shortLen;
        d += "M" + x + " 0 L" + x + " " + len + " ";
    }
    return d;
}

/*! Four corner brackets (L shapes, `len` long) on a w x h box inset by `i`. */
function corners(w, h, len, i) {
    const l = i;
    const r = w - i;
    const t = i;
    const b = h - i;
    return "M" + l + " " + (t + len) + " L" + l + " " + t + " L" + (l + len) + " " + t + " "
        + "M" + (r - len) + " " + t + " L" + r + " " + t + " L" + r + " " + (t + len) + " "
        + "M" + r + " " + (b - len) + " L" + r + " " + b + " L" + (r - len) + " " + b + " "
        + "M" + (l + len) + " " + b + " L" + l + " " + b + " L" + l + " " + (b - len) + " ";
}
