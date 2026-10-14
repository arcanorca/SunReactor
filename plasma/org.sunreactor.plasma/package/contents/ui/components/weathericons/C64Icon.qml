/*
 * Commodore 64: the PETSCII weather picture, a three-by-two block of graphics
 * characters (Petscii.sprite), in the theme's light blue on a plate of the theme's
 * blue, like the C64 screen. Each row is its own PixelGlyphs, as the popup draws them,
 * so one art pixel is `artPx` device px and the rows stay on the art grid. The
 * picture is drawn from 22 px; below that, and where its block does not fit the box,
 * the generic line glyph. The plate and ink are the theme's own, so surface and ink
 * are unused.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma
import "../../engine"
import "../../skins/commodore64/Petscii.js" as Petscii

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
    /*! The picture is 18 art px across and 16 down (a cell is 6 x 8); the art px is the
     *  largest whole device-pixel count that keeps the width inside the box. */
    readonly property int artPx: Math.floor(size * dpr / 18 + 0.001)
    /*! Crisp from 22 px: below that the line glyph stands in. The block must also fit the box. */
    readonly property real minSize: 22
    readonly property bool drawn: size >= minSize && artPx >= 1 && artCols * 6 * artPx <= width * dpr
    readonly property var rows: Petscii.sprite(condition, night)
    readonly property int artCols: {
        let n = 1;
        for (let i = 0; i < rows.length; ++i) {
            n = Math.max(n, Petscii.cells(rows[i]));
        }
        return n;
    }
    /*! The block's left and top edge in device px, centred on the device grid. */
    readonly property int originX: Math.round((width * dpr - artCols * 6 * artPx) / 2)
    readonly property int originY: Math.round((height * dpr - rows.length * 8 * artPx) / 2)

    /*! The C64 screen the picture sits on: the theme's blue, on whole device px. */
    Rectangle {
        visible: root.drawn
        width: Math.round(root.width * root.dpr) / root.dpr
        height: Math.round(root.height * root.dpr) / root.dpr
        color: root.theme.bg
    }

    Repeater {
        model: root.rows

        delegate: PixelGlyphs {
            id: line

            required property int index
            required property string modelData

            visible: root.drawn
            x: root.originX / root.dpr
            y: (root.originY + index * 8 * root.artPx) / root.dpr
            text: Petscii.fold(modelData)
            face: "petscii"
            capitals: true
            color: root.theme.fg
            artScale: root.artPx / root.dpr
        }
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.drawn
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
