/*
 * A small disclosure chevron, one stroke in one colour (Shapes). `up` flips it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    /*! Width of the chevron, logical px; it is half as tall. */
    property real size: 8
    property bool up: false
    property color color: "transparent"

    implicitWidth: size
    implicitHeight: size / 2
    width: size
    height: size / 2

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.color
            strokeWidth: 1.5
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin

            PathPolyline {
                path: root.up
                    ? [Qt.point(0, root.height), Qt.point(root.width / 2, 0), Qt.point(root.width, root.height)]
                    : [Qt.point(0, 0), Qt.point(root.width / 2, root.height), Qt.point(root.width, 0)]
            }
        }
    }
}
