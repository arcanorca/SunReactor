/*
 * Commodore 64: the PETSCII sun, moon, cloud or rain sprite of the popup (three
 * graphics-character rows), in the accent on the surface. Each row is its own
 * PixelGlyphs, as the popup draws them, so one art pixel is `artPx` device px and
 * the rows stay on the art grid. Below the size for three rows it draws the first
 * two; below that, the generic line glyph.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma
import "../../engine"
import "../../panel/PanelData.js" as PanelData
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
    /*! Three rows are 24 art px tall, two rows 16; a cell is 6 art px across and 8 down. */
    readonly property int pxThree: Math.floor(size * dpr / 24 + 0.001)
    readonly property int pxTwo: Math.floor(size * dpr / 16 + 0.001)
    readonly property bool threeRows: pxThree >= 1
    readonly property int artPx: threeRows ? pxThree : pxTwo
    /*! The block must fit the box: a two-row sprite five cells wide does not fit a small panel. */
    readonly property bool drawn: artPx >= 1 && artCols * 6 * artPx <= width * dpr
    readonly property var rows: Petscii.sprite(condition, night).slice(0, threeRows ? 3 : 2)
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
            color: PanelData.ink(root.theme.accent, root.surface, root.ink, 3)
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
