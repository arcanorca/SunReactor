/*
 * A vibrancy panel: a rounded rect of translucent light over the background, one
 * device pixel of highlight along its top edge and one device pixel of outer
 * edge. Its children are stacked in a column inside the padding; the panel is
 * exactly as tall as that column.
 *
 *   Surface {
 *       style: root.style
 *       RowLayout { ... }
 *   }
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property MacStyle style
    property real radius: style.radiusCard
    property real padding: style.padCard
    /*! Stacking gap between the children. */
    property alias spacing: column.spacing
    /*! Children go into the column. */
    default property alias content: column.data

    implicitWidth: column.implicitWidth + 2 * root.padding
    implicitHeight: column.implicitHeight + 2 * root.padding

    Rectangle {
        anchors.fill: parent
        radius: root.radius
        antialiasing: true
        color: root.style.material
        border.width: root.style.hairline
        border.color: root.style.materialEdge
    }

    // The highlight stops short of the corners, as a real top edge does.
    Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: root.radius
        anchors.rightMargin: root.radius
        height: root.style.hairline
        color: root.style.materialTop
    }

    ColumnLayout {
        id: column

        anchors.fill: parent
        anchors.margins: root.padding
        spacing: root.style.gap
    }
}
