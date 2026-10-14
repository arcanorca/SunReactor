/*
 * A seven-segment readout: lit segments in `color` over the ghost of every
 * segment in the glass's ink at 8 %. `size` is the digit height in logical px.
 * Slanted, chamfered segments, snapped to device pixels; no animation.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "Casio.js" as Casio

Item {
    id: root

    required property CasioStyle style
    property string text: ""
    /*! Digit height, logical px. */
    property real size: style.digitSmall
    property color color: style.ink

    readonly property var glyph: Casio.seven(text, size, style.dpr)

    implicitWidth: Math.ceil(glyph.width)
    implicitHeight: Math.ceil(size)
    width: implicitWidth
    height: implicitHeight
    baselineOffset: size

    Accessible.role: Accessible.StaticText
    Accessible.name: text.trim()

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeWidth: -1
            fillColor: root.style.ghost
            PathSvg { path: root.glyph.ghost }
        }
        ShapePath {
            strokeWidth: -1
            fillColor: root.color
            PathSvg { path: root.glyph.lit }
        }
    }
}
