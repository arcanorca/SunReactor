/*
 * One condition as an icon for the popup modules: the theme's own weather icon
 * (components/weathericons, by theme id or renderer), or, when the user chose a
 * weather icon style in settings, the vector glyph in that style or the legacy
 * 16 px sprite for "pixel". Modules size it; the glyph and the themed icons snap
 * to device pixels themselves, and the sprite is a Kirigami icon at whole-pixel size.
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
import "weathericons"

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

    /*! The user picked a weather icon style: the generic glyph or sprite. */
    readonly property bool styleChosen: ["line", "solid", "pixel"].indexOf(theme.overrides.weatherIconStyle) >= 0
    readonly property bool pixelArt: styleChosen && theme.glyphStyle === "pixel"
    readonly property bool themed: theme.available && !styleChosen

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    ThemedWeatherIcon {
        anchors.centerIn: parent
        visible: root.themed
        theme: root.theme
        condition: root.condition
        night: root.night
        size: root.size
        surface: root.colours.bg
        ink: root.colours.fg
    }

    Kirigami.Icon {
        anchors.fill: parent
        visible: root.pixelArt
        roundToIconSize: false
        source: Icons.conditionArt(root.condition, root.night)
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.pixelArt && !root.themed
        condition: root.condition
        night: root.night
        glyphStyle: root.theme.glyphStyle
        palette: root.colours
        size: Math.round(root.size)
    }
}
