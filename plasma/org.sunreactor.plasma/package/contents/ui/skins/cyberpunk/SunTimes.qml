/*
 * Sun times, cyberpunk: the sunrise, noon and sunset readouts on one line in a
 * titled panel of its own, the day's length in the tag. With the weather on the
 * page, the same line closes the Weather panel (SunLine, drawn by Weather.qml)
 * and this module stays away.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    /*! Drawn inside the Weather panel instead, when the theme lists both. */
    readonly property bool hosted: root.theme.modules.indexOf("weather") >= 0
        && client.isConnected && client.weatherEnabled
    readonly property bool available: !hosted && line.available

    spacing: 0

    CyberFrame {
        Layout.fillWidth: true
        visible: root.available
        style: root.style
        title: i18ndc(root.domain, "Panel title: sunrise and sunset", "Sun times")
        tag: line.dayLength > 0
            ? i18ndc(root.domain, "Length of daylight, hours and minutes, compact", "DAY %1H %2M",
                     Math.floor(line.dayLength / 3600), String(Math.round(line.dayLength % 3600 / 60)).padStart(2, "0"))
            : ""
        contentSpacing: 0

        SunLine {
            id: line

            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
