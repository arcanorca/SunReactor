/*
 * Ordered dither across the theme's shade ramp: the palette quantiser for
 * pixel themes. `level` 0..1 picks a position between the darkest and the
 * lightest shade; with `levelTo` >= 0 the level runs across the item (a ramp).
 * Pattern = the theme's dither token (none / bayer2 / bayer4) unless set.
 * Only ramp colours are ever written, so it passes the conformance check by
 * construction. Fills its own size; inSurface as for BitmapText.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

PixelDither {
    id: root

    required property Theme theme
    property bool inSurface: false
    /*! Ramp to dither across; defaults to the theme shades, dark to light. */
    property var ramp: theme.shades

    colors: ramp
    pattern: theme.dither
    artScale: inSurface ? 1 : theme.grid.unit
}
