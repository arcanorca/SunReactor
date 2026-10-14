/*
 * A touch button of the console: a rounded panel with a thick outline and a soft
 * top-to-bottom fill. `kind` picks the face: "panel" (white to pale blue, the
 * default), "ds" (filled DS blue, the chosen state) or "quiet" (greyed, for a
 * control that cannot be changed). Children are placed on the panel.
 */

pragma ComponentBehavior: Bound

import QtQuick

Rectangle {
    id: root

    required property NdsStyle style
    property string kind: "panel"
    property color outlineColor: style.blue
    property int outlineWidth: style.outline

    /*! Children sit on the panel. */
    default property alias content: stage.data

    radius: style.radius
    border.width: outlineWidth
    border.color: outlineColor
    gradient: Gradient {
        GradientStop {
            position: 0
            color: root.kind === "ds" ? Qt.lighter(root.style.blue, 1.12)
                : (root.kind === "quiet" ? root.style.quietTop : root.style.panelTop)
        }
        GradientStop {
            position: 1
            color: root.kind === "ds" ? root.style.blue
                : (root.kind === "quiet" ? root.style.quietBottom : root.style.panelBottom)
        }
    }

    Item {
        id: stage

        anchors.fill: parent
    }
}
