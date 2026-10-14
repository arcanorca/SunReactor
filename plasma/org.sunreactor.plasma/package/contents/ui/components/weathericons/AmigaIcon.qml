/*
 * Amiga (Workbench): the 16 px sky sprite of the pixel atlas, on the Workbench pens
 * (the skin's own sprite colours), at a whole number of device pixels per art pixel.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma
import "../../skins/amiga"
import "../../skins/pixel/Sprites.js" as Sprites

Item {
    id: root

    required property string condition
    required property bool night
    required property var theme
    property real size: 22
    property color surface: theme.bg
    property color ink: theme.fg

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! Device px per art px: the largest whole multiple of 16 that fits, at least 1. */
    readonly property int artPx: Math.max(1, Math.floor(size * dpr / 16 + 0.001))

    AmigaStyle {
        id: style

        theme: root.theme
    }

    PixelSprite {
        x: Math.round((root.width * root.dpr - 16 * root.artPx) / 2) / root.dpr
        y: Math.round((root.height * root.dpr - 16 * root.artPx) / 2) / root.dpr
        source: Sprites.weatherAtlas()
        frame: Sprites.conditionFrame(root.condition, root.night)
        colors: style.spriteColors
        artScale: root.artPx / root.dpr
    }
}
