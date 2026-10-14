/*
 * A band of hazard stripes (yellow on the navy base). Used for small decorative
 * strips only: the Automation header, and the manual / paused alert.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "Geometry.js" as G

Item {
    id: root

    required property CyberStyle style
    property color stripe: style.yellow
    property color base: style.theme.bg

    implicitHeight: style.snap(Math.max(4, Math.round(style.unit * 0.55)))
    clip: true

    Rectangle {
        anchors.fill: parent
        color: root.base
    }
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeWidth: -1
            fillColor: root.stripe
            PathSvg { path: G.hazard(root.width, root.height) }
        }
    }
}
