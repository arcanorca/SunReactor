/*
 * Xbox 360 glossy metallic-glass weather icons, backed by pre-rendered HiDPI
 * assets in skins/xbox360/assets/weather/. Below 16 logical px it falls back
 * to the clean WeatherGlyph so panel strips stay crisp.
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

    readonly property bool iconMode: size >= 16
    readonly property bool isDark: theme.xboxDarkMode || theme.luminance(surface) < 0.35
    readonly property string assetName: {
        switch (condition) {
        case "clear": return night ? "clear_night" : "clear_day";
        case "partly_cloudy": return night ? "partly_cloudy_night" : "partly_cloudy_day";
        case "cloudy": return "cloudy";
        case "drizzle": return "drizzle";
        case "rain": return "rain";
        case "heavy_rain": return "heavy_rain";
        case "thunderstorm": return "thunderstorm";
        case "snow": return "snow";
        case "mist": return "mist";
        case "atmospheric": return "atmospheric";
        case "fog": return "fog";
        default: return "unknown";
        }
    }

    Image {
        visible: root.iconMode
        anchors.fill: parent
        source: Qt.resolvedUrl("../../skins/xbox360/assets/" + (root.isDark ? "dark/weather/" : "weather/") + root.assetName + ".png")
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.iconMode
        condition: root.condition
        night: root.night
        glyphStyle: "solid"
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
