/*
 * Weather, instrument family: the reading and what it does to the targets.
 *
 *   WEATHER ─────────────────────────────────────────────────────
 *   15 °C   Partly cloudy                      DIM ▮▮▮▮▮▮▮▮▮▯ −12 %
 *   FEELS 14°   RH 62 %   WIND 3.4 m/s   AQI 42
 *
 * The daemon's bounded multiplier is shown as a micro-bar (full = no change)
 * and in words-as-numbers; the secondary readings are printed, not hidden
 * in a tooltip. Shown only when weather is enabled.
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
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.weatherEnabled

    readonly property int change: Math.round((1.0 - client.weatherMultiplier) * 100)
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
            out.push({ k: i18ndc(domain, "Short legend: apparent temperature", "Feels"),
                       v: i18ndc(domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)) });
        }
        if (d.humidityPercent !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: relative humidity", "RH"),
                       v: i18ndc(domain, "Percentage", "%1%", Math.round(d.humidityPercent)) });
        }
        if (d.windSpeedMps !== undefined) {
            out.push({ k: i18ndc(domain, "Short legend: wind speed", "Wind"),
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

    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.domain, "Instrument legend over the weather reading", "Weather")

        Tag {
            anchors.verticalCenter: parent ? parent.verticalCenter : undefined
            visible: root.client.hasWeatherReading && root.client.weatherStale
            style: root.style
            tone: root.style.warning
            text: i18ndc(root.domain, "Tag: the weather reading is out of date", "Stale")
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        Row {
            Layout.alignment: Qt.AlignBaseline
            visible: root.client.hasTemperature
            spacing: root.style.snap(root.style.gap * 0.5)

            Digits {
                id: temperature

                style: root.style
                kind: "type"
                glowing: false
                size: Math.round(root.style.sizeMid * 0.74)
                font: root.style.midFont
                text: String(Math.round(root.client.temperatureC))
            }
            Text {
                anchors.baseline: temperature.baseline
                text: "°C"
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkMuted
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignBaseline
            text: root.client.hasWeatherReading
                ? (weatherText.conditionLabel(root.client.weatherCondition) || i18nd(root.domain, "No weather reading"))
                : root.stateNote
            textFormat: Text.PlainText
            font: root.style.bodyFont
            color: root.client.hasWeatherReading ? root.style.ink : root.style.inkMuted
            elide: Text.ElideRight
        }

        // Effect on the targets: micro-bar plus the change in percent.
        Row {
            Layout.alignment: Qt.AlignVCenter
            visible: root.client.hasWeatherReading && !root.client.weatherStale
            spacing: root.style.gap

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.change >= 2 ? i18ndc(root.domain, "Legend: weather is dimming the displays", "Dim")
                    : (root.change <= -2 ? i18ndc(root.domain, "Legend: weather is brightening the displays", "Boost")
                                         : i18ndc(root.domain, "Legend: weather has no effect now", "No effect"))
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkMuted
            }

            Item {
                id: micro

                anchors.verticalCenter: parent.verticalCenter
                visible: Math.abs(root.change) >= 2
                width: root.style.snap(root.style.unit * 2.4)
                height: root.style.snap(root.style.sizeLegend * 0.8)

                readonly property int cells: 10
                readonly property real pitch: width / cells
                readonly property int lit: Math.round(Math.max(0, Math.min(1, root.client.weatherMultiplier)) * cells)

                Repeater {
                    model: micro.cells
                    delegate: Rectangle {
                        required property int index
                        x: root.style.snap(index * micro.pitch)
                        width: Math.max(root.style.hairline, root.style.floorSnap(micro.pitch * 0.7))
                        height: micro.height
                        color: index < micro.lit ? root.style.ink : root.style.theme.alpha(root.style.warning, 0.5)
                    }
                }
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: Math.abs(root.change) >= 2
                text: root.change >= 2 ? i18ndc(root.domain, "Percent points removed", "−%1%", root.change)
                                       : i18ndc(root.domain, "Percent points added", "+%1%", -root.change)
                textFormat: Text.PlainText
                font: root.style.dataFont
                color: root.change >= 2 ? root.style.warning : root.style.ok
            }
        }
    }

    Flow {
        Layout.fillWidth: true
        visible: root.extras.length > 0 && root.client.hasWeatherReading
        spacing: root.style.gap2 * 1.5

        Repeater {
            model: root.extras

            delegate: Row {
                id: extra

                required property var modelData

                spacing: root.style.gap

                Text {
                    anchors.baseline: extraValue.baseline
                    text: extra.modelData.k
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkMuted
                }
                Text {
                    id: extraValue
                    text: extra.modelData.v
                    textFormat: Text.PlainText
                    font: root.style.dataSmallFont
                    color: root.style.ink
                }
            }
        }
    }
}
