/*
 * macOS (Weather): the multicolour solid glyph, the theme's own roles (sun in the
 * accent, cloud in the ink, rain and bolt in the secondary and warning colours).
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
        glyphStyle: "solid"
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
