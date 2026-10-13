/*
 * One weather condition as an icon for the popup modules: the vector glyph
 * (WeatherGlyph) in the theme's weather icon style, or the legacy 16 px sprite
 * when that style is "pixel". Modules size it; the glyph snaps to device pixels
 * itself, and the sprite is a Kirigami icon at whole-pixel size.
 *
 *   WeatherIcon { theme: root.theme; condition: "rain"; night: false; size: 22 }
 *
 * `colours` defaults to the theme's glyph roles on its own background; a surface
 * of another colour passes its own (see Theme.glyphColours).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../Icons.js" as Icons

Item {
    id: root

    required property Theme theme
    /*! Daemon condition token; "" is the unknown icon. */
    property string condition: ""
    property bool night: false
    /*! Logical px, square. */
    property real size: Kirigami.Units.iconSizes.medium
    /*! WeatherGlyph colour roles; see Theme.glyphColours. */
    property var colours: theme.glyphPalette

    readonly property bool pixelArt: theme.glyphStyle === "pixel"

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    Kirigami.Icon {
        anchors.fill: parent
        visible: root.pixelArt
        roundToIconSize: false
        source: Icons.conditionArt(root.condition, root.night)
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.pixelArt
        condition: root.condition
        night: root.night
        glyphStyle: root.theme.glyphStyle
        palette: root.colours
        size: Math.round(root.size)
    }
}
