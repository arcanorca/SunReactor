/*
 * Pixel themes (handheld, classic Macintosh, the generic pixel family): the 16 px
 * sky sprite, drawn at a whole number of device pixels per art pixel (PixelSkySprite
 * decodes the sprite at its art size, so it stays crisp at fractional scales).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma
import "../../Icons.js" as Icons

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

    PixelSkySprite {
        x: Math.round((root.width * root.dpr - 16 * root.artPx) / 2) / root.dpr
        y: Math.round((root.height * root.dpr - 16 * root.artPx) / 2) / root.dpr
        source: Icons.conditionArt(root.condition, root.night)
        artSize: 16
        artScale: root.artPx / root.dpr
    }
}
