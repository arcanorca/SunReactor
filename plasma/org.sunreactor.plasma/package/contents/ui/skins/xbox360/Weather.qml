/*
 * Xbox 360 Weather: metallic blade hero card with Xbox360Icon, bold Chakra
 * Petch temperature, condition/effect summary, and 2x2 detail tiles.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../../components/weathericons"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

    readonly property int change: Math.round((1.0 - client.weatherMultiplier) * 100)
    readonly property string stateNote: client.hasWeatherReading ? "" : weatherText.stateNote(client.weatherState)
    readonly property string effectText: {
        if (client.weatherStale) {
            return i18ndc(domain, "Tag: the weather reading is out of date", "Out of date");
        }
        if (change >= 2) {
            return i18ndc(domain, "Short: the weather dims the displays by a percentage", "Dims %1%", change);
        }
        if (change <= -2) {
            return i18ndc(domain, "Short: the weather brightens the displays by a percentage", "Brightens %1%", -change);
        }
        return "";
    }
    readonly property var extras: {
        const d = client.weatherDetails;
        const out = [];
        if (d.feelsLikeC !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: apparent temperature", "Feels"),
                       v: i18ndc(domain, "Temperature in degrees, compact", "%1°", Format.tempRounded(d.feelsLikeC, client.isFahrenheit)) });
        }
        if (d.humidityPercent !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: relative humidity", "Humidity"),
                       v: i18ndc(domain, "Percentage", "%1%", Math.round(d.humidityPercent)) });
        }
        if (d.windSpeedMps !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: wind speed", "Wind"),
                       v: i18ndc(domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)) });
        }
        if (d.airQualityIndex !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: US EPA air quality index", "Air"),
                       v: String(Math.round(d.airQualityIndex)) });
        }
        return out;
    }

    spacing: style.rowGap

    WeatherText {
        id: weatherText
    }

    XboxCard {
        id: card

        style: root.style
        pad: root.style.cardPad
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 2 * card.pad

        RowLayout {
            id: inner

            anchors.fill: parent
            spacing: root.style.gap2

            ThemedWeatherIcon {
                theme: root.theme
                size: root.style.snap(root.style.unit * 2.4)
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
                Layout.alignment: Qt.AlignVCenter
                Xbox360Icon {
                    anchors.centerIn: parent
                    theme: root.theme
                    size: root.style.snap(root.style.unit * 2.4)
                    condition: root.client.weatherCondition
                    night: root.client.weatherIsNight
                }
            }

            RowLayout {
                Layout.alignment: Qt.AlignVCenter
                spacing: 0

                Text {
                    Layout.alignment: Qt.AlignBaseline
                    visible: root.client.hasTemperature
                    text: String(Format.tempRounded(root.client.temperatureC, root.client.isFahrenheit))
                    textFormat: Text.PlainText
                    font: root.style.heroFont
                    color: root.style.greenBright
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    Layout.leftMargin: root.style.gap * 0.5
                    visible: root.client.hasTemperature
                    text: Format.tempUnit(root.client.isFahrenheit)
                    textFormat: Text.PlainText
                    font: root.style.titleFont
                    color: root.style.green
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 1

                Text {
                    Layout.fillWidth: true
                    visible: root.client.locationCity.length > 0
                    text: root.client.locationCity
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.weatherBlue
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: {
                        if (!root.client.hasWeatherReading) {
                            return root.stateNote;
                        }
                        const condition = weatherText.conditionLabel(root.client.weatherCondition)
                            || i18nd(root.domain, "No weather reading");
                        return root.effectText.length > 0 ? condition + " · " + root.effectText : condition;
                    }
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    font: root.style.bodyBoldFont
                    color: root.client.hasWeatherReading ? root.style.ink : root.style.muted
                    elide: Text.ElideRight
                }
            }
        }
    }

    GridLayout {
        Layout.fillWidth: true
        visible: root.extras.length > 0 && root.client.hasWeatherReading
        columns: 2
        columnSpacing: root.style.gap
        rowSpacing: root.style.gap

        Repeater {
            model: root.extras

            delegate: XboxCard {
                id: detail

                required property var modelData

                style: root.style
                pad: root.style.cardPad
                Layout.fillWidth: true
                implicitHeight: Math.max(root.style.snap(root.style.unit * 1.85), line.implicitHeight + 2 * detail.pad)

                RowLayout {
                    id: line

                    anchors.fill: parent
                    spacing: root.style.gap

                    Text {
                        text: detail.modelData.k
                        textFormat: Text.PlainText
                        font: root.style.captionFont
                        color: root.style.muted
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: detail.modelData.v
                        textFormat: Text.PlainText
                        font: root.style.bodyBoldFont
                        color: root.style.ink
                    }
                }
            }
        }
    }
}
