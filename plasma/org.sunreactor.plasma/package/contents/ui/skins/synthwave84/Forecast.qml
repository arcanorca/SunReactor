/*
 * Forecast, synthwave84: the next five intervals under a heading, in a neon card.
 * With the weather on the page, the same strip closes the Weather card
 * (ForecastStrip, drawn by Weather.qml) and this module stays away.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    /*! Drawn inside the Weather card instead, when the theme lists both. */
    readonly property bool hosted: root.theme.modules.indexOf("weather") >= 0
        && client.isConnected && client.weatherEnabled
    readonly property bool available: !hosted && strip.available

    spacing: style.rowGap

    Heading {
        visible: root.available
        style: root.style
        text: i18ndc(root.domain, "Synthwave heading of the forecast", "Forecast")
    }

    NeonCard {
        Layout.fillWidth: true
        visible: root.available
        style: root.style
        neon: root.style.pink

        ForecastStrip {
            id: strip

            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
