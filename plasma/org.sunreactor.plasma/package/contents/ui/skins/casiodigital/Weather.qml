/*
 * Weather, casiodigital: the reading and what it does to the targets. The
 * temperature is in seven segments, the condition a printed caption, the icon a
 * solid glyph in the ink. The effect on the displays is an LCD flag (DIM, BOOST,
 * NO EFFECT) and its size in segments; the secondary readings are printed below.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Casio.js" as Casio

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CasioStyle style: CasioStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled
    /*! Percent points the weather takes off the targets (negative: adds). */
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

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    WeatherText {
        id: weatherText
    }

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.domain, "Weather")
        trailing: root.client.hasWeatherReading && root.client.weatherStale
            ? i18ndc(root.domain, "Tag: the weather reading is out of date", "STALE") : ""

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gapWide
            visible: root.client.hasWeatherReading

            WeatherGlyph {
                Layout.alignment: Qt.AlignVCenter
                size: root.style.digitHero
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
                glyphStyle: "solid"
                palette: root.style.glyphPalette
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                RowLayout {
                    spacing: root.style.gap

                    CasioDigits {
                        style: root.style
                        size: root.style.digitMedium
                        text: root.client.hasTemperature ? Casio.numberText(root.client.temperatureC, 3) : "---"
                    }
                    Text {
                        Layout.alignment: Qt.AlignBottom
                        text: "°C"
                        textFormat: Text.PlainText
                        font: root.style.cap(root.style.sizeBody)
                        color: root.style.inkMuted
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: weatherText.conditionLabel(root.client.weatherCondition)
                        || i18nd(root.domain, "No weather reading")
                    textFormat: Text.PlainText
                    font: root.style.cap(root.style.sizeCap * 1.2)
                    color: root.style.ink
                    elide: Text.ElideRight
                }
            }

            // The effect on the targets: a flag and its size in percent points, on one line.
            RowLayout {
                Layout.alignment: Qt.AlignTop
                spacing: root.style.gap
                CasioFlag {
                    Layout.alignment: Qt.AlignVCenter
                    style: root.style
                    text: root.change >= 2 ? i18ndc(root.domain, "Legend: weather is dimming the displays", "DIM")
                        : (root.change <= -2 ? i18ndc(root.domain, "Legend: weather is brightening the displays", "BOOST")
                                             : i18ndc(root.domain, "Legend: weather has no effect now", "NO EFFECT"))
                    lit: Math.abs(root.change) >= 2
                }
                RowLayout {
                    Layout.alignment: Qt.AlignVCenter
                    spacing: root.style.snap(root.style.gap * 0.5)
                    visible: Math.abs(root.change) >= 2
                    CasioDigits {
                        style: root.style
                        size: root.style.digitSmall
                        text: Casio.numberText(Math.abs(root.change), 3)
                    }
                    Text {
                        Layout.alignment: Qt.AlignBottom
                        text: "%"
                        textFormat: Text.PlainText
                        font: root.style.cap(root.style.sizeBody)
                        color: root.style.inkMuted
                    }
                }
            }
        }

        // Readings that do not earn a figure: printed, not hidden in a tooltip.
        Flow {
            Layout.fillWidth: true
            visible: root.extras.length > 0 && root.client.hasWeatherReading
            spacing: root.style.gapWide

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
                        font: root.style.cap(root.style.sizeCap)
                        color: root.style.inkMuted
                    }
                    Text {
                        id: extraValue

                        text: extra.modelData.v
                        textFormat: Text.PlainText
                        font: root.style.data(root.style.sizeBody)
                        color: root.style.ink
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: !root.client.hasWeatherReading
            text: root.stateNote
            textFormat: Text.PlainText
            font: root.style.body(root.style.sizeBody)
            color: root.style.ink
            wrapMode: Text.WordWrap
        }
    }
}
