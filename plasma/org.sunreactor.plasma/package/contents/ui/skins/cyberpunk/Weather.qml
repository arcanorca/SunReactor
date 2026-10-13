/*
 * Weather, cyberpunk: the reading as a line glyph in cyan and yellow, the
 * temperature, the sky, and what the weather does to the targets as a cell bar
 * with the change in percent. Secondary readings follow in brackets.
 *
 *   [glyph]  15°C  PARTLY CLOUDY         DIM ▰▰▰▰▰▰▰▰▰▱  [−12%]
 *            FEELS [14°]  RH [62%]  WIND [3.4 m/s]  AQI [42]
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.weatherEnabled

    readonly property int change: Math.round((1.0 - client.weatherMultiplier) * 100)
    /*! Line glyph: the sky in cyan, the sun and moon in yellow. */
    readonly property var glyphColours: theme.glyphColours(theme.bg, theme.secondaryAccent)
    readonly property string stateNote: {
        if (client.hasWeatherReading) {
            return "";
        }
        switch (client.weatherState) {
        case "no_api_key":
            return i18nd(domain, "No API key is configured");
        case "loading":
            return i18nd(domain, "Looking up the current conditions…");
        case "unauthorized":
            return i18nd(domain, "The weather provider rejected the API key");
        case "rate_limited":
            return i18nd(domain, "The weather provider is rate limiting requests");
        case "network_error":
            return i18nd(domain, "The weather provider could not be reached");
        default:
            return i18nd(domain, "No reading yet");
        }
    }
    readonly property var extras: {
        const d = client.weatherDetails;
        const out = [];
        if (d.feelsLikeC !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: apparent temperature", "FEELS"),
                       v: i18ndc(domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)) });
        }
        if (d.humidityPercent !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: relative humidity", "RH"),
                       v: i18ndc(domain, "Percentage", "%1%", Math.round(d.humidityPercent)) });
        }
        if (d.windSpeedMps !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: wind speed", "WIND"),
                       v: i18ndc(domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)) });
        }
        if (d.airQualityIndex !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: US EPA air quality index", "AQI"),
                       v: String(Math.round(d.airQualityIndex)) });
        }
        return out;
    }

    spacing: style.rowGap

    WeatherText {
        id: weatherText
    }

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        title: i18ndc(root.domain, "Panel title: the weather reading", "Weather")

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap2

            WeatherGlyph {
                Layout.alignment: Qt.AlignVCenter
                visible: root.client.hasWeatherReading
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
                glyphStyle: "line"
                palette: root.glyphColours
                size: Math.round(root.style.unit * 2.2)
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: root.style.gap

                Row {
                    spacing: root.style.gap

                    Text {
                        id: temperature

                        visible: root.client.hasTemperature
                        text: String(Math.round(root.client.temperatureC))
                        textFormat: Text.PlainText
                        font: root.style.midFont
                        color: root.style.yellow
                    }
                    Text {
                        visible: root.client.hasTemperature
                        anchors.baseline: temperature.baseline
                        text: "°C"
                        textFormat: Text.PlainText
                        font: root.style.bodyFont
                        color: root.style.muted
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: root.client.hasWeatherReading
                        ? (weatherText.conditionLabel(root.client.weatherCondition) || i18nd(root.domain, "No weather reading"))
                        : root.stateNote
                    textFormat: Text.PlainText
                    font: root.client.hasWeatherReading ? root.style.labelFont : root.style.bodyFont
                    color: root.client.hasWeatherReading ? root.style.ink : root.style.muted
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }
            }

            // The stale flag, the effect on the targets: a cell bar and the change in percent.
            ColumnLayout {
                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: root.client.hasWeatherReading
                spacing: root.style.gap

                Text {
                    Layout.alignment: Qt.AlignRight
                    visible: root.client.weatherStale
                    text: i18ndc(root.domain, "Tag: the weather reading is out of date", "STALE")
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: root.style.red
                }
                Text {
                    Layout.alignment: Qt.AlignRight
                    visible: !root.client.weatherStale
                    text: root.change >= 2 ? i18ndc(root.domain, "Legend: weather is dimming the displays", "DIM")
                        : (root.change <= -2 ? i18ndc(root.domain, "Legend: weather is brightening the displays", "BOOST")
                                             : i18ndc(root.domain, "Legend: weather has no effect now", "NO EFFECT"))
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: root.style.muted
                }
                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    visible: !root.client.weatherStale
                    spacing: root.style.gap

                    CellBar {
                        Layout.preferredWidth: root.style.snap(root.style.unit * 5.5)
                        Layout.preferredHeight: root.style.snap(root.style.unit * 1.0)
                        style: root.style
                        cells: 10
                        value: Math.round(Math.max(0, Math.min(1, root.client.weatherMultiplier)) * 100)
                        litColor: root.style.cyan
                    }
                    Bracket {
                        style: root.style
                        value: root.change >= 2 ? "−" + root.change + "%"
                            : (root.change <= -2 ? "+" + (-root.change) + "%" : "0%")
                        font: root.style.techFont
                        color: root.style.cyan
                    }
                }
            }
        }

        Flow {
            Layout.fillWidth: true
            visible: root.extras.length > 0 && root.client.hasWeatherReading
            spacing: root.style.gap

            Repeater {
                model: root.extras

                delegate: Row {
                    id: extra

                    required property var modelData

                    spacing: root.style.gap

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: extra.modelData.k
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.muted
                    }
                    Bracket {
                        anchors.verticalCenter: parent.verticalCenter
                        style: root.style
                        value: extra.modelData.v
                        font: root.style.techFont
                        color: root.style.cyan
                        bracketColor: root.style.muted
                    }
                }
            }
        }
    }
}
