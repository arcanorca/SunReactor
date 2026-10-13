/*
 * Forecast, cyberpunk: the next five intervals in a titled panel of its own. With
 * the weather on the page, the same strip closes the Weather panel (ForecastStrip,
 * drawn by Weather.qml) and this module stays away.
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
    readonly property bool available: !hosted && strip.available

    spacing: 0

    CyberFrame {
        Layout.fillWidth: true
        visible: root.available
        style: root.style
        title: i18ndc(root.domain, "Panel title: the weather forecast", "Forecast")
        tag: strip.points.length > 0 ? i18ndc(root.domain, "Number of forecast columns", "%1 SLOTS", strip.points.length) : ""
        contentSpacing: 0

        ForecastStrip {
            id: strip

            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
