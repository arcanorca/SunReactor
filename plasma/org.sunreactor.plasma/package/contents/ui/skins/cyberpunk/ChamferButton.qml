/*
 * A chamfered button: top-right and bottom-left corners cut. Active is a solid
 * yellow face with navy text. `call` (the way back) and hover take the yellow
 * outline; a keyboard-focused button gets a cyan inner outline. The owner sets
 * activeFocusOnTab and the keys; this item only draws and reports clicks.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "Geometry.js" as G

Item {
    id: root

    required property CyberStyle style
    property string text: ""
    property bool active: false
    property bool call: false
    property font font: style.labelFont
    /*! Shifts the label sideways, to make room for an icon at one side. */
    property real labelOffset: 0

    readonly property bool hovered: mouse.containsMouse
    readonly property real cut: style.snap(style.tabHeight * 0.5)

    signal clicked()

    implicitWidth: label.implicitWidth + style.pad * 2 + cut * 2
    implicitHeight: style.tabHeight + style.gap

    Shape {
        anchors.fill: parent
        ShapePath {
            strokeWidth: -1
            fillColor: root.active ? root.style.yellow
                : (root.hovered ? root.style.theme.alpha(root.style.yellow, 0.12) : "transparent")
            PathSvg { path: G.chamfer(root.width, root.height, root.cut, 0) }
        }
    }
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeColor: (root.active || root.call || root.hovered) ? root.style.yellow : root.style.rule
            strokeWidth: root.style.outline
            fillColor: "transparent"
            joinStyle: ShapePath.MiterJoin
            PathSvg { path: G.chamfer(root.width, root.height, root.cut, root.style.outline / 2) }
        }
    }
    Shape {
        anchors.fill: parent
        visible: root.activeFocus
        ShapePath {
            strokeColor: root.style.cyan
            strokeWidth: root.style.hairline
            fillColor: "transparent"
            PathSvg { path: G.chamfer(root.width, root.height, root.cut, root.style.outline + root.style.hairline * 3) }
        }
    }

    Text {
        id: label

        anchors.centerIn: parent
        anchors.horizontalCenterOffset: root.labelOffset
        text: root.text
        textFormat: Text.PlainText
        font: root.font
        color: root.active ? root.style.onYellow
            : ((root.hovered || root.call) ? root.style.ink : root.style.muted)
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
