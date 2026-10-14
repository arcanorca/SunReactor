/*
 * Cyberpunk: the monoline glyph in the neon (secondary accent) on the surface,
 * the same line work as the popup's reading.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"

Item {
    id: root

    required property string condition
    required property bool night
    required property var theme
    property real size: 22
    property color surface: theme.bg
    property color ink: theme.fg

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: root.theme.glyphColours(root.surface, root.theme.secondaryAccent)
        size: Math.round(root.size)
    }
}
