/*
 * Instrument family (vector themes with no icon of their own: braun, nixie, VFD,
 * oscilloscope, thinkpad, ...): the monoline WeatherGlyph in the theme's colours
 * on the surface, like the instrument panels' own line work.
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
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
