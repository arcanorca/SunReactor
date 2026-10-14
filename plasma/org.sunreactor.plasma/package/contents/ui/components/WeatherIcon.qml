/*
 * One condition as an icon for the popup modules: the icon the user's "Weather
 * icons" setting picks (components/weathericons/ThemedWeatherIcon), which is the
 * theme's own icon in the default "Match the theme" style.
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

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    ThemedWeatherIcon {
        anchors.centerIn: parent
        theme: root.theme
        condition: root.condition
        night: root.night
        size: root.size
        surface: root.colours.bg
        ink: root.colours.fg
    }
}
