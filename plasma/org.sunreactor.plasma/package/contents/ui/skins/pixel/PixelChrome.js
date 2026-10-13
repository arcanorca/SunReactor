.pragma library

/*
 * Window chrome of the pixel styles as lists of art-pixel rectangles
 * {x, y, w, h, c}. PixelPanel draws them with one Repeater of Rectangles, so
 * a frame costs a handful of nodes and every edge sits on the art grid.
 *
 * Each chrome also defines its padding (where content may start) and where
 * its title goes. `t` carries the colours (all theme tokens, see PixelStyle).
 */

function push(list, x, y, w, h, c) {
    if (w > 0 && h > 0) {
        list.push({ x: x, y: y, w: w, h: h, c: String(c) });
    }
}

/*! 1 px outline; cut 0 = square, 1 = corner pixel missing, 2 = rounded. */
function ring(list, x, y, w, h, c, cut) {
    if (cut >= 2) {
        push(list, x + 2, y, w - 4, 1, c);
        push(list, x + 2, y + h - 1, w - 4, 1, c);
        push(list, x, y + 2, 1, h - 4, c);
        push(list, x + w - 1, y + 2, 1, h - 4, c);
        push(list, x + 1, y + 1, 1, 1, c);
        push(list, x + w - 2, y + 1, 1, 1, c);
        push(list, x + 1, y + h - 2, 1, 1, c);
        push(list, x + w - 2, y + h - 2, 1, 1, c);
        return;
    }
    const k = cut >= 1 ? 1 : 0;
    push(list, x + k, y, w - 2 * k, 1, c);
    push(list, x + k, y + h - 1, w - 2 * k, 1, c);
    push(list, x, y + 1, 1, h - 2, c);
    push(list, x + w - 1, y + 1, 1, h - 2, c);
}

/*! Padding {l, t, r, b} and title placement {tx, ty} per chrome. */
function metrics(chrome, hasTitle, titleH) {
    switch (chrome) {
    case "textbox":
        return { l: 6, t: hasTitle ? titleH + 6 : 6, r: 6, b: 4, tx: 7, ty: 3 };
    case "petscii":
        return { l: 5, t: hasTitle ? titleH + 6 : 5, r: 5, b: 4, tx: 5, ty: 0 };
    case "window":
        return { l: 5, t: 14, r: 6, b: 4, tx: 0, ty: 3 };
    case "lcd":
        return { l: 5, t: hasTitle ? Math.ceil(titleH / 2) + 5 : 5, r: 5, b: 5, tx: 0, ty: 0 };
    case "workbench":
        return { l: 5, t: 14, r: 5, b: 4, tx: 14, ty: 2 };
    case "card":
        return { l: 6, t: hasTitle ? titleH + 9 : 6, r: 6, b: 6, tx: 6, ty: 5 };
    default: // chamfer
        return { l: 5, t: hasTitle ? Math.ceil(titleH / 2) + 5 : 5, r: 5, b: 5, tx: 8, ty: 0 };
    }
}

/*!
 * The rectangles of one panel. W x H is the whole panel in art px; title is
 * {w, h, x} of the laid-out title (w 0 = none); t = colours:
 * { paper, ink, frame, hot, inverse, light, dark, desk, lightPaper }.
 */
function rects(chrome, W, H, title, t) {
    const out = [];
    const hasTitle = title.w > 0;
    // The surface is opaque: whatever the chrome leaves open shows the desk,
    // never what happens to be behind the popup.
    push(out, 0, 0, W, H, t.desk);
    switch (chrome) {
    case "textbox": {
        // Game Boy menu box: paper, double rule with a rounded outer corner.
        push(out, 0, 0, W, H, t.paper);
        ring(out, 0, 0, W, H, t.ink, 1);
        ring(out, 2, 2, W - 4, H - 4, t.ink, 0);
        if (hasTitle) {
            // Reverse-video tab sitting on the inner rule.
            push(out, title.x - 2, 2, title.w + 4, title.h + 3, t.ink);
        }
        break;
    }
    case "petscii": {
        // Rounded PETSCII box; the title is reverse video on the top rule.
        const top = hasTitle ? Math.floor(title.h / 2) : 0;
        ring(out, 0, top, W, H - top, t.frame, 2);
        if (hasTitle) {
            push(out, title.x - 1, 0, title.w + 2, title.h + 1, t.ink);
        }
        break;
    }
    case "window": {
        // System 1 document window: 1 px frame, drop shadow, striped title bar.
        const w = W - 1;
        const h = H - 1;
        push(out, 0, 0, w, h, t.paper);
        ring(out, 0, 0, w, h, t.ink, 0);
        push(out, 1, h, w, 1, t.ink);          // shadow, bottom
        push(out, w, 1, 1, h, t.ink);          // shadow, right
        push(out, 1, 12, w - 2, 1, t.ink);     // title bar rule
        for (let y = 3; y <= 9; y += 2) {
            push(out, 2, y, w - 4, 1, t.ink);
        }
        // Close box with a paper margin.
        push(out, 7, 2, 11, 9, t.paper);
        ring(out, 8, 2, 9, 9, t.ink, 0);
        if (hasTitle) {
            push(out, title.x - 4, 2, title.w + 8, 9, t.paper);
        }
        break;
    }
    case "lcd": {
        // Watch LCD window; the caption is printed on the bezel line.
        const top = hasTitle ? Math.floor(title.h / 2) : 0;
        ring(out, 0, top, W, H - top, t.frame, 2);
        if (hasTitle) {
            push(out, title.x - 3, top, title.w + 6, 1, t.desk);
        }
        break;
    }
    case "workbench": {
        // Workbench window: blue body, white/black bevel, white title bar
        // with the close gadget left and the depth gadget right.
        push(out, 0, 0, W, H, t.paper);
        push(out, 0, 0, W, 1, t.light);
        push(out, 0, 0, 1, H, t.light);
        push(out, 0, H - 1, W, 1, t.dark);
        push(out, W - 1, 0, 1, H, t.dark);
        push(out, 1, 1, W - 2, 11, t.light);       // title bar
        push(out, 1, 12, W - 2, 1, t.dark);
        // Close gadget: a box with a dot.
        ring(out, 3, 3, 7, 7, t.dark, 0);
        push(out, 5, 5, 3, 3, t.dark);
        // Depth gadget: back box hollow, front box filled.
        ring(out, W - 16, 3, 8, 6, t.dark, 0);
        push(out, W - 12, 5, 8, 6, t.dark);
        push(out, W - 11, 6, 6, 4, t.paper);
        break;
    }
    case "card": {
        // Nothing widget card: quiet outline, rounded.
        ring(out, 0, 0, W, H, t.frame, 2);
        if (hasTitle) {
            push(out, title.x - 6, title.y + 2, 3, 3, t.hot);
        }
        break;
    }
    default: {
        // Neon: chamfered outline, hot tick on the top edge, title on the line.
        const top = hasTitle ? Math.floor(title.h / 2) : 0;
        const c = t.frame;
        push(out, 3, top, W - 3, 1, c);                 // top (after chamfer)
        push(out, 0, top + 3, 1, H - top - 3, c);       // left
        push(out, 0, H - 1, W - 3, 1, c);               // bottom
        push(out, W - 1, top, 1, H - top - 3, c);       // right
        for (let i = 0; i < 3; ++i) {
            push(out, 2 - i, top + i, 1, 1, c);         // chamfer top-left
            push(out, W - 3 + i, H - 1 - (2 - i), 1, 1, c); // chamfer bottom-right
        }
        push(out, 3, top - 1 < 0 ? top : top - 1, 3, 2, t.hot);
        if (hasTitle) {
            push(out, title.x - 2, top, title.w + 4, 1, t.desk);
        }
        break;
    }
    }
    return out;
}
