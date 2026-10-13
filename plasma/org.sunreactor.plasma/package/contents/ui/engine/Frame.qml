/*
 * Border styles for every renderer: none / line / bevel / inset / double.
 *
 *   pixel: true   inside a PixelSurface: widths are art pixels, corners square,
 *                 edges drawn as four rectangles (no anti-aliasing, so the
 *                 frame only contains its own colours);
 *   pixel: false  vector: widths in logical px (default one crisp device
 *                 pixel), `radius` from the theme for the line style.
 *
 * bevel = raised (light top/left, dark bottom/right), inset = sunken,
 * double = two lines with a gap, as in box-drawing ═ frames.
 * The frame draws only the border unless `fillColor` is set; place content
 * inside with anchors.margins: frame.inset.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property Theme theme
    property string style: theme.border
    property bool pixel: theme.isPixel
    property real lineWidth: pixel ? 1 : theme.lineWidth
    property color color: theme.borderInactive
    property color lightColor: theme.shades.length ? theme.shades[theme.shades.length - 1] : theme.fg
    property color darkColor: theme.shades.length ? theme.shades[0] : theme.bg
    property color fillColor: "transparent"
    property real radius: pixel ? 0 : theme.radius
    /*! A gradient for the fill, a skin's backdrop (Modules.BACKDROP); null keeps fillColor. */
    property var fillGradient: null

    /*! Distance from the outer edge to where content may start. */
    readonly property real inset: {
        switch (style) {
        case "none":
            return 0;
        case "double":
            return lineWidth * 3;
        case "bevel":
        case "inset":
            return lineWidth * 2;
        default:
            return lineWidth;
        }
    }

    readonly property bool squareEdges: pixel || radius <= 0 || style !== "line"
    readonly property color topLeft: style === "bevel" ? lightColor : (style === "inset" ? darkColor : color)
    readonly property color bottomRight: style === "bevel" ? darkColor : (style === "inset" ? lightColor : color)
    readonly property real edge: (style === "bevel" || style === "inset") ? lineWidth * 2 : lineWidth

    // Vector line with rounded corners.
    Rectangle {
        anchors.fill: parent
        visible: !root.squareEdges && root.style !== "none"
        radius: root.radius
        color: root.fillColor
        gradient: root.fillGradient
        border.width: root.lineWidth
        border.color: root.color
        antialiasing: true
    }

    // Square fill (pixel or square vector).
    Rectangle {
        anchors.fill: parent
        visible: root.squareEdges || root.style === "none"
        color: root.fillColor
        gradient: root.fillGradient
        antialiasing: false
    }

    // Four square edges; for double, a second ring inside.
    Repeater {
        model: !root.squareEdges || root.style === "none" ? 0 : (root.style === "double" ? 2 : 1)

        delegate: Item {
            id: ring

            required property int index
            readonly property real offset: index * root.lineWidth * 2

            anchors.fill: parent
            anchors.margins: offset

            Rectangle { // top
                x: 0; y: 0; width: ring.width; height: root.edge
                color: root.topLeft; antialiasing: false
            }
            Rectangle { // left
                x: 0; y: 0; width: root.edge; height: ring.height
                color: root.topLeft; antialiasing: false
            }
            Rectangle { // bottom
                x: 0; y: ring.height - root.edge; width: ring.width; height: root.edge
                color: root.bottomRight; antialiasing: false
            }
            Rectangle { // right
                x: ring.width - root.edge; y: 0; width: root.edge; height: ring.height
                color: root.bottomRight; antialiasing: false
            }
        }
    }
}
