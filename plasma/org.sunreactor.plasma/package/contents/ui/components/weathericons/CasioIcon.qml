/*
 * Casio digital (LCD): the monoline glyph in the LCD ink on a glass tile, as on
 * the watch display. Overlapping sun, moon and cloud shapes trim cleanly instead
 * of fusing into one silhouette. The glass and ink come from CasioStyle.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../../skins/casiodigital"

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

    CasioStyle {
        id: style

        theme: root.theme
    }

    Rectangle {
        anchors.fill: parent
        radius: Math.round(root.size * 0.12)
        color: style.glass
        border.width: style.hairline
        border.color: style.caseEdge
        antialiasing: true
    }

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: style.glyphPalette
        size: Math.round(root.size * 0.78)
    }
}
