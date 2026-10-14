/*
 * Wii (Forecast Channel): the solid glyph in the Wii's colours (a yellow sun,
 * Wii blue rain, slate clouds), as on the channel tile of the popup.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../../skins/wii"

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

    WiiStyle {
        id: style

        theme: root.theme
    }

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "solid"
        palette: style.glyphPalette
        size: Math.round(root.size)
    }
}
