/*
 * A small picture on the cell grid: rows of characters, each cell in a colour
 * role. Block elements (full, half, quadrant, eighth, shade) are rectangles on
 * the device pixel grid, the rest are glyphs at their cell. See TermPicture.js.
 *
 *     rows:  ["▄▄ ", " ▀▀"]   roles: ["aa ", "  a"]   colours: { a: accent }
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "TermPicture.js" as Picture

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    /*! One string per row; roles has the same shape (a space = not drawn). */
    property var rows: []
    property var roles: []
    /*! role -> colour. */
    property var colours: ({})

    readonly property int cols: Picture.cols(rows)
    readonly property var parts: Picture.prims(rows, roles, tm.cwDev, tm.chDev)

    implicitWidth: tm.cx(cols)
    implicitHeight: tm.cy(rows.length)
    width: implicitWidth
    height: implicitHeight

    function colourOf(role, shade) {
        const base = colours[role] !== undefined ? colours[role] : theme.fg;
        return shade >= 1 ? base : theme.mix(theme.bg, base, shade);
    }

    Repeater {
        model: root.parts.rects

        delegate: Rectangle {
            required property var modelData

            x: modelData.x / root.tm.dpr
            y: modelData.y / root.tm.dpr
            width: modelData.w / root.tm.dpr
            height: modelData.h / root.tm.dpr
            color: root.colourOf(modelData.role, modelData.shade)
            antialiasing: false
        }
    }

    Repeater {
        model: root.parts.texts

        delegate: TermText {
            required property var modelData

            tm: root.tm
            x: root.tm.cx(modelData.col)
            y: root.tm.cy(modelData.row)
            value: modelData.value
            color: root.colourOf(modelData.role, 1)
        }
    }
}
