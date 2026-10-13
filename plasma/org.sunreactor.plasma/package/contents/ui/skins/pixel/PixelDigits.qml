/*
 * A numeric readout in the hardware's own digits (inside a PixelSurface):
 *
 *   cells   Game Boy LCD: 5x7 glyph on a cell grid, unlit cells in the ghost
 *           shade - the LCD matrix shows through
 *   seg     Casio: seven segments with ghost segments
 *   dots    Nothing: dot matrix
 *   block   solid 5x7 glyph scaled up (Mac, Amiga, neon)
 *   bitmap  the theme's bitmap font at 2x (Commodore 64)
 *
 * `size` 1 is the normal readout, 2 the hero. Characters a face cannot show
 * (a translated unit, say) fall back to the 5x7 face at the same scale.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"

Item {
    id: root

    required property Theme theme
    required property PixelStyle style
    property string text: ""
    property color color: style.ink
    property color ghostColor: style.ghost
    /*! Unlit segments (seg) / unlit matrix dots (dots, cells). Seven-segment
        ghosts read as stray 8s in short numbers, so only the clock opts in. */
    property bool showGhost: kind !== "seg"
    /*! 1 = readout, 2 = hero. */
    property int size: 1
    /*! Override the style's digit kind. */
    property string kind: style.digits

    readonly property bool useBitmap: kind === "bitmap"
    readonly property bool segOk: kind === "seg" && glyphs.covers(text, "seg")
    readonly property int artWidth: useBitmap ? bitmap.artWidth : glyphs.artWidth
    readonly property int artHeight: useBitmap ? bitmap.artHeight : glyphs.artHeight
    readonly property int capHeight: useBitmap ? 8 * size : glyphs.capHeight

    width: artWidth
    height: artHeight
    implicitWidth: artWidth
    implicitHeight: artHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: text

    PixelGlyphs {
        id: glyphs

        visible: !root.useBitmap
        text: root.useBitmap ? "" : root.text
        color: root.color
        ghostColor: root.showGhost && (root.segOk || root.kind === "dots" || root.kind === "cells")
            ? root.ghostColor : "transparent"
        face: root.segOk ? "seg" : "small"
        // seg: 2 x (L + 3T) tall; hero L5 T2 = 16 rows, readout L3 T1 = 9 rows.
        segmentLength: root.size >= 2 ? 6 : 3
        segmentWidth: root.size >= 2 ? 2 : 1
        letterSpacing: root.segOk ? 1 : 1
        pitch: {
            switch (root.kind) {
            case "cells":
            case "dots":
                return root.size >= 2 ? 3 : 2;
            case "seg":
                return root.size >= 2 ? 2 : 1;
            default:
                return root.size >= 2 ? 2 : 1;
            }
        }
        dot: root.kind === "cells" || root.kind === "dots" ? (root.size >= 2 ? 2 : 1) : 0
        round: false
    }

    BitmapText {
        id: bitmap

        visible: root.useBitmap
        theme: root.theme
        inSurface: true
        text: root.useBitmap ? root.text : ""
        color: root.color
        magnification: root.size
    }
}
