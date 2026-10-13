/*
 * Weather, synthwave84: the weather page as one neon card under its heading. The
 * reading in one row (the cyan line glyph, the temperature in the readout face, the
 * condition), what the weather does to the displays, the secondary readings as a
 * 2 x 2 grid, then the forecast strip and the sun times, each under a hairline,
 * when the theme lists them. Shown only when weather is enabled.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../components" as Components

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.weatherEnabled
    /*! The forecast and the sun times join this card when the theme lists them. */
    readonly property bool forecastShown: root.theme.modules.indexOf("forecast") >= 0 && forecast.available
    readonly property bool sunShown: root.theme.modules.indexOf("suntimes") >= 0 && line.available

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
            out.push({ k: i18ndc(domain, "Short legend: relative humidity", "Humidity"),
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

    Components.WeatherText {
        id: weatherText
    }

    Heading {
        visible: root.available
        style: root.style
        text: i18ndc(root.domain, "Synthwave heading of the weather page", "Weather")
    }

    NeonCard {
        Layout.fillWidth: true
        visible: root.available
        style: root.style
        neon: root.style.cyan

        // The reading: the glyph, the temperature and the condition in one row.
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap2

            WeatherGlyph {
                Layout.alignment: Qt.AlignVCenter
                size: root.style.snap(root.style.unit * 2.0)
                condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
                night: root.client.hasWeatherReading && root.client.weatherIsNight
                glyphStyle: "line"
                palette: root.style.glyphPalette
            }
            RowLayout {
                Layout.alignment: Qt.AlignVCenter
                spacing: root.style.gap

                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: root.client.hasTemperature ? String(Math.round(root.client.temperatureC)) : "—"
                    textFormat: Text.PlainText
                    font: root.style.readoutFont
                    color: root.style.ink
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    visible: root.client.hasTemperature
                    text: "°C"
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: root.style.cyan
                }
            }
            Text {
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                Layout.alignment: Qt.AlignVCenter
                text: root.client.hasWeatherReading
                    ? (weatherText.conditionLabel(root.client.weatherCondition) || i18nd(root.domain, "No weather reading"))
                    : root.stateNote
                textFormat: Text.PlainText
                font: root.client.hasWeatherReading ? root.style.bodyStrongFont : root.style.bodyFont
                color: root.client.hasWeatherReading ? root.style.ink : root.style.muted
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
            Pill {
                visible: root.client.hasWeatherReading && root.client.weatherStale
                Layout.alignment: Qt.AlignVCenter
                style: root.style
                text: i18ndc(root.domain, "Tag: the weather reading is out of date", "Stale")
                tone: root.style.yellow
            }
        }

        // What the weather does to the displays, in words and a number.
        Text {
            Layout.fillWidth: true
            visible: root.client.hasWeatherReading
            text: {
                if (root.client.weatherStale) {
                    return i18nd(root.domain, "Last reading is out of date");
                }
                if (root.change >= 2) {
                    return i18ndc(root.domain, "Placeholder is a percentage", "Dimming displays by %1%", root.change);
                }
                if (root.change <= -2) {
                    return i18ndc(root.domain, "Placeholder is a percentage", "Brightening displays by %1%", -root.change);
                }
                return i18nd(root.domain, "Not changing brightness right now");
            }
            textFormat: Text.PlainText
            font: root.style.dataSmallFont
            color: root.client.weatherStale ? root.style.yellow
                : (root.change >= 2 ? root.style.yellow : root.style.cyan)
            elide: Text.ElideRight
        }

        // The secondary readings: two columns of caps label and value.
        GridLayout {
            Layout.fillWidth: true
            visible: root.extras.length > 0 && root.client.hasWeatherReading
            columns: 2
            columnSpacing: root.style.gap2 * 1.5
            rowSpacing: root.style.gap * 0.5

            Repeater {
                model: root.extras

                delegate: RowLayout {
                    id: extra

                    required property var modelData

                    Layout.fillWidth: true
                    spacing: root.style.gap

                    Text {
                        text: extra.modelData.k
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.cyan
                    }
                    Text {
                        Layout.fillWidth: true
                        text: extra.modelData.v
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.ink
                        elide: Text.ElideRight
                    }
                }
            }
        }

        Rectangle {
            visible: root.forecastShown
            Layout.fillWidth: true
            implicitHeight: root.style.hairline
            color: root.style.alpha(root.style.cyan, 0.45)
        }

        ForecastStrip {
            id: forecast

            visible: root.forecastShown
            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }

        Rectangle {
            visible: root.sunShown
            Layout.fillWidth: true
            implicitHeight: root.style.hairline
            color: root.style.alpha(root.style.cyan, 0.45)
        }

        SunLine {
            id: line

            visible: root.sunShown
            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
