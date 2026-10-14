/*
 * The generic weather icon (the registry's fallback): WeatherGlyph in the theme's
 * glyph style (line, or solid for pixel themes), in the colours of the surface.
 * Used by themes with no icon file and when the user chose a style in settings.
 * On the Game Boy DMG (handheld) the glyph is drawn in the theme's four shades only.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../../skins/pixel/Sprites.js" as Sprites

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
        // The DMG's ramp, whatever the surface: one of its four shades per role.
        palette: Sprites.isMonoRamp(root.theme)
            ? ({ ramp: root.theme.shades.map(c => root.theme.hexOf(c)) })
            : root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
