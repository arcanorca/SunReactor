/*
 * One DS screen: a PixelSurface holding the lid (the light-blue menu background
 * with its bubbles, the dark bezel) and, for the top screen, the hinge strip
 * under it. `lidHeight` is the screen's own height in art px; the hinge adds
 * Nds.HINGE below it. Children are laid out in art px inside the lid, as in any
 * PixelSurface, so a module puts its content here.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "Nds.js" as Nds

PixelSurface {
    id: root

    required property Theme theme
    /*! The top screen: the hinge strip sits under the lid. */
    property bool hinge: false
    /*! The screen's own height in art px. */
    property int lidHeight: 0

    artHeight: lidHeight + (hinge ? Nds.HINGE : 0)

    readonly property var colours: ({
        bubble: theme.bg, bezel: theme.fg, plastic: theme.borderInactive,
        dot: theme.fg, power: theme.success, wifi: theme.accent
    })

    // The menu background: an ordered dither of the accent into the paper, so the
    // lid reads as the DS menu's light blue (palette colours only).
    PixelDither {
        width: root.artWidth
        height: root.lidHeight
        colors: [root.theme.accent, root.theme.bg]
        level: 0.8
        pattern: "bayer4"
    }

    Repeater {
        model: Nds.lid([], root.artWidth, root.lidHeight, root.hinge, root.colours)

        delegate: Rectangle {
            required property var modelData

            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.c
            antialiasing: false
        }
    }
}
