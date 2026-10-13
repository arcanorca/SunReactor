/*
 * A theme's palette in four squares: background, foreground, accent,
 * secondary accent (a list_themes entry or a normalised theme palette).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Row {
    id: root

    /*! [bg, fg, accent, secondaryAccent] as colour strings. */
    property var colors: []
    property real size: Kirigami.Units.iconSizes.small * 0.75
    property color outline: Qt.rgba(0.5, 0.5, 0.5, 0.55)

    spacing: 0
    Accessible.ignored: true

    Repeater {
        model: root.colors
        delegate: Rectangle {
            required property var modelData
            width: root.size
            height: root.size
            color: modelData
            border.width: 1
            border.color: root.outline
        }
    }
}
