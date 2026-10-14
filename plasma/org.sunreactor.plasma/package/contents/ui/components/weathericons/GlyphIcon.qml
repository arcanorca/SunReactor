/*
 * The generic weather icon (the registry's fallback): WeatherGlyph in the theme's
 * glyph style (line, or solid for pixel themes), in the colours of the surface.
 * Used by themes with no icon file and when the user chose a style in settings.
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
        glyphStyle: root.theme.glyphStyle
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
