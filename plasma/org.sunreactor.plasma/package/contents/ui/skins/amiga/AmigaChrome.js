.pragma library

/*
 * Workbench chrome as lists of art-pixel rectangles {x, y, w, h, c}, drawn by a
 * Repeater of Rectangles (antialiasing off) inside a PixelSurface. Every edge
 * sits on the art grid; colours come from AmigaStyle, passed in by the caller.
 *
 *   window   2 px bevel (white top/left, black bottom/right), grey title bar
 *            with a close gadget at left and a depth gadget at right, the title
 *            text after the close gadget, a black rule, a white body
 *   gadget   raised Workbench gadget: black outline, grey face, white top/left
 *            highlight, black bottom/right shadow; lit = orange face, pressed =
 *            black face (inverse)
 *   trough   proportional-gadget track: recessed (black top/left, white
 *            bottom/right), grey inside, optional tick marks below
 *   knob     the proportional knob: orange, black outline, white highlight
 */

function push(list, x, y, w, h, c) {
    if (w > 0 && h > 0) {
        list.push({ x: x, y: y, w: w, h: h, c: String(c) });
    }
}

/*! Title bar height in art px (rows 2 .. 2 + TITLE_H - 1 inside the bevel). */
var TITLE_H = 8;
/*! Close and depth gadgets: 7 x 7 art px, centred in the title bar. */
var GADGET = 7;
/*! Content padding (art px) from the window's outer edge: bevel plus margin. */
var PAD = { l: 4, r: 4, b: 3, top: 13 };
/*! A plain window (no title bar): the bevel and a small margin only. */
var PLAIN = { l: 4, r: 4, b: 4, top: 4 };

/*! Window chrome, W x H art px; the title text is a child item on the bar. */
function windowRects(W, H, t) {
    const out = [];
    // Bevel: the window's frame, white light from the top left, black shadow below.
    push(out, 0, 0, W, 2, t.body);
    push(out, 0, 0, 2, H, t.body);
    push(out, 0, H - 2, W, 2, t.ink);
    push(out, W - 2, 0, 2, H, t.ink);
    // Title bar and its rule.
    push(out, 2, 2, W - 4, TITLE_H, t.face);
    push(out, 2, 2 + TITLE_H, W - 4, 1, t.ink);
    // Body.
    push(out, 2, 3 + TITLE_H, W - 4, H - 5 - TITLE_H, t.body);
    // Close gadget, left.
    const gy = 2 + Math.floor((TITLE_H - GADGET) / 2);
    raised(out, 4, gy, GADGET, GADGET, false, t);
    // Depth gadget, right: a back square and a front square.
    const dx = W - 4 - GADGET;
    raised(out, dx, gy, GADGET, GADGET, false, t);
    push(out, dx + 2, gy + 2, 3, 3, t.ink);
    push(out, dx + 1, gy + 1, 1, 1, t.face);
    return out;
}

/*! Plain window: a 2 px bevel around a white body, no title bar. */
function plainRects(W, H, t) {
    const out = [];
    push(out, 0, 0, W, 2, t.body);
    push(out, 0, 0, 2, H, t.body);
    push(out, 0, H - 2, W, 2, t.ink);
    push(out, W - 2, 0, 2, H, t.ink);
    push(out, 2, 2, W - 4, H - 4, t.body);
    return out;
}

/*! Raised gadget face of W x H at (x, y): outline, face, highlight, shadow. */
function raised(list, x, y, W, H, lit, t) {
    push(list, x, y, W, H, t.ink);
    const face = lit ? t.hot : t.face;
    push(list, x + 1, y + 1, W - 2, H - 2, face);
    if (!lit) {
        push(list, x + 1, y + 1, W - 2, 1, t.body);
        push(list, x + 1, y + 1, 1, H - 2, t.body);
        push(list, x + 1, y + H - 2, W - 2, 1, t.ink);
        push(list, x + W - 2, y + 1, 1, H - 2, t.ink);
    }
}

/*! A gadget button: lit = orange, inverse = black (pressed). */
function gadgetRects(W, H, state, t) {
    const out = [];
    const lit = state === "lit";
    const inverse = state === "pressed";
    if (inverse) {
        push(out, 0, 0, W, H, t.ink);
    } else {
        raised(out, 0, 0, W, H, lit, t);
    }
    return out;
}

/*! The proportional trough: W x 7 art px, recessed. */
function troughRects(W, t) {
    const out = [];
    push(out, 0, 0, W, 7, t.face);
    push(out, 0, 0, W, 1, t.ink);
    push(out, 0, 0, 1, 7, t.ink);
    push(out, 0, 6, W, 1, t.body);
    push(out, W - 1, 0, 1, 7, t.body);
    return out;
}

/*! The knob at x: 9 x 5 art px inside a trough (rows 1..5). */
function knobRects(x, t) {
    const out = [];
    push(out, x, 1, 9, 5, t.ink);
    push(out, x + 1, 2, 7, 3, t.hot);
    push(out, x + 1, 2, 7, 1, t.body);
    push(out, x + 1, 2, 1, 3, t.body);
    return out;
}
