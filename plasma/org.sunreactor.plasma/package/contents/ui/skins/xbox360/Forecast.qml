/*
 * Xbox 360 Forecast: up to 5 upcoming slots as metallic blade mini-tiles.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components/weathericons"
import "../../Format.js" as Format
import "../../modules/ForecastRules.js" as ForecastRules

RowLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }
    readonly property int maxSlots: 5
    readonly property var points: ForecastRules.upcoming(client.forecast, client.nowEpochS, maxSlots)
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled && points.length > 0

    spacing: style.gap

    Accessible.role: Accessible.List
    Accessible.name: i18ndc(domain, "Title of the forecast panel", "Forecast")

    Repeater {
        model: root.points

        delegate: XboxCard {
            id: slot

            required property var modelData

            style: root.style
            pad: root.style.snap(root.style.cardPad * 0.9)
            Layout.fillWidth: true
            implicitHeight: col.implicitHeight + 2 * slot.pad

            Accessible.role: Accessible.ListItem
            Accessible.name: Format.timeText(modelData.epochS) + " "
                + i18ndc(root.domain, "Temperature in degrees, compact", "%1°", Format.tempRounded(modelData.temperatureC, root.client.isFahrenheit))

            ColumnLayout {
                id: col

                anchors.fill: parent
                spacing: root.style.gap * 0.5

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: Format.timeText(slot.modelData.epochS)
                    textFormat: Text.PlainText
                    font: root.style.captionFont
                    color: root.style.muted
                }
                ThemedWeatherIcon {
                    Layout.alignment: Qt.AlignHCenter
                    theme: root.theme
                    size: root.style.snap(root.style.unit * 1.45)
                    condition: slot.modelData.condition
                    night: slot.modelData.isNight
                    Xbox360Icon {
                        anchors.centerIn: parent
                        theme: root.theme
                        size: root.style.snap(root.style.unit * 1.45)
                        condition: slot.modelData.condition
                        night: slot.modelData.isNight
                    }
                }
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°", Format.tempRounded(slot.modelData.temperatureC, root.client.isFahrenheit))
                    textFormat: Text.PlainText
                    font: root.style.bodyBoldFont
                    color: root.style.ink
                }
            }
        }
    }
}
