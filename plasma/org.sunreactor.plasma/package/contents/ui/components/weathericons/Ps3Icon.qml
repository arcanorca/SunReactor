/*
 * PlayStation 3 (XMB) crystal-glass weather icons, backed by etched-crystal
 * assets in skins/ps3/assets/weather/. Falls back to a clean white monoline
 * WeatherGlyph below 16 logical px.
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

    readonly property var mono: ({
        bg: theme.hexOf(surface),
        fg: theme.hexOf(ink),
        text_muted: theme.hexOf(ink),
        accent: theme.hexOf(ink),
        secondary_accent: theme.hexOf(ink),
        warning: theme.hexOf(ink)
    })

    Image {
        visible: root.iconMode
        anchors.fill: parent
        source: Qt.resolvedUrl("../../skins/ps3/assets/weather/" + root.assetName + ".png")
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.iconMode
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: root.mono
        size: Math.round(root.size)
    }
}
