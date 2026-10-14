/*
 * Synthwave '84: the monoline neon glyph in the skin's own roles (cyan line work,
 * the pink sun, the yellow bolt), as on the popup's reading.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../../skins/synthwave84"

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

    SynthStyle {
        id: style

        theme: root.theme
    }

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: style.glyphPalette
        size: Math.round(root.size)
    }
}
