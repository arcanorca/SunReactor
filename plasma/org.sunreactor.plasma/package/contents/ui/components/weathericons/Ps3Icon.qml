/*
 * PlayStation 3 (XMB): a thin monoline glyph in one colour, the theme's ink on
 * the surface, with no accent roles: the XMB icons are white line work.
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

    readonly property var mono: ({
        bg: theme.hexOf(surface),
        fg: theme.hexOf(ink),
        text_muted: theme.hexOf(ink),
        accent: theme.hexOf(ink),
        secondary_accent: theme.hexOf(ink),
        warning: theme.hexOf(ink)
    })

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: root.mono
        size: Math.round(root.size)
    }
}
