/*
 * An LCD indicator flag, as the printed legends on a watch face: a small marker
 * and a caption. Lit, the flag is a solid plate of ink with the caption reversed
 * out of it; unlit, it is an outline that barely shows, like the ghost segments.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    required property CasioStyle style
    property string text: ""
    property bool lit: false
    property bool interactive: false
    property bool hovered: false
    /*! True when drawn on the dark resin watch case (e.g. LightCycle header)
        rather than the bright LCD glass. */
    property bool onCase: false

    readonly property real markSize: style.snap(style.sizeCap * 0.62)
    readonly property real hpad: style.snap(style.sizeCap * 0.55)
    readonly property real vpad: style.snap(style.sizeCap * 0.3)

    implicitWidth: Math.ceil(row.implicitWidth) + 2 * hpad
    implicitHeight: Math.ceil(Math.max(label.implicitHeight, markSize)) + 2 * vpad
    width: implicitWidth
    height: implicitHeight

    Accessible.role: interactive ? Accessible.Button : Accessible.StaticText
    Accessible.name: text + (!interactive && !lit ? " (off)" : "")

    Rectangle {
        anchors.fill: parent
        radius: 2
        antialiasing: true
        color: root.lit
            ? (root.onCase ? root.style.legend : root.style.ink)
            : (root.interactive ? root.style.theme.alpha(root.style.ink, root.hovered ? 0.18 : 0.09) : "transparent")
        border.width: root.lit ? 0 : (root.interactive ? root.style.hairline * 1.4 : root.style.hairline)
        border.color: root.onCase ? root.style.caseEdge
            : (root.interactive ? root.style.theme.alpha(root.style.ink, 0.55) : root.style.inkFaint)
    }

    Row {
        id: row

        anchors.centerIn: parent
        spacing: root.style.snap(root.style.sizeCap * 0.45)

        // A marker like the little triangles on a watch face.
        Shape {
            anchors.verticalCenter: parent.verticalCenter
            width: root.markSize
            height: root.markSize
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: -1
                fillColor: root.lit ? (root.onCase ? root.style.caseTop : root.style.glass)
                                    : (root.onCase ? root.style.legendMuted
                                       : (root.interactive ? root.style.ink : root.style.inkFaint))
                PathSvg {
                    path: "M0 0 H" + root.markSize + " L" + root.markSize / 2 + " " + root.markSize * 0.86 + " Z"
                }
            }
        }

        Text {
            id: label

            anchors.verticalCenter: parent.verticalCenter
            text: root.text
            textFormat: Text.PlainText
            font: root.style.cap(root.style.sizeCap)
            color: root.lit ? (root.onCase ? root.style.caseTop : root.style.glass)
                            : (root.onCase ? root.style.legendMuted
                               : (root.interactive ? root.style.ink : root.style.inkFaint))
        }
    }
}
