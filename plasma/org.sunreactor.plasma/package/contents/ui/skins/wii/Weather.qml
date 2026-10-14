/*
 * Weather, Wii style (the Forecast Channel's reading): a large friendly glyph,
 * the temperature in big rounded numerals, the condition in words and what the
 * weather does to the displays. The secondary readings sit in a 2 x 2 grid of
 * small tiles. Shown only when weather is enabled.
 *
 *   ┌────────────────────────────────────────────┐
 *   │  (sun)   15°C                              │
 *   │          Partly cloudy · Dims 4 %          │
 *   └────────────────────────────────────────────┘
 *   ┌──────────────────┐ ┌──────────────────┐
 *   │ Feels      14°   │ │ Humidity     62 % │
 *   └──────────────────┘ └──────────────────┘
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../../components/weathericons"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property WiiStyle style: WiiStyle {
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
    /*! What the weather does to the displays, in a few words; "" for no effect. */
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
            out.push({ k: i18ndc(domain, "Short legend: US EPA air quality index", "Air"),
                       v: String(Math.round(d.airQualityIndex)) });
        }
        return out;
    }

    spacing: style.rowGap

    WeatherText {
        id: weatherText
    }

    WiiTile {
        id: tile

        style: root.style
        pad: root.style.gap
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 2 * tile.pad

        RowLayout {
            id: inner

            anchors.fill: parent
            spacing: root.style.gap2

            WiiWeatherIcon {
                Layout.alignment: Qt.AlignVCenter
                theme: root.theme
                size: root.style.snap(root.style.unit * 2.3)
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
            }

            RowLayout {
                Layout.alignment: Qt.AlignVCenter
                spacing: 0

                Text {
                    Layout.alignment: Qt.AlignBaseline
                    visible: root.client.hasTemperature
                    text: String(Math.round(root.client.temperatureC))
                    textFormat: Text.PlainText
                    font: root.style.heroFont
                    color: root.style.accentInk
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    Layout.leftMargin: root.style.gap
                    visible: root.client.hasTemperature
                    text: "°C"
                    textFormat: Text.PlainText
                    font: root.style.titleFont
                    color: root.style.accentInk
                }
            }

            // The condition, and what the weather does to the displays.
            Text {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
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
                color: root.client.hasWeatherReading ? root.style.ink : root.style.inkSoft
                elide: Text.ElideRight
            }
        }
    }

    // The secondary readings, two by two.
    GridLayout {
        Layout.fillWidth: true
        visible: root.extras.length > 0 && root.client.hasWeatherReading
        columns: 2
        columnSpacing: root.style.gap
        rowSpacing: root.style.gap

        Repeater {
            model: root.extras

            delegate: WiiTile {
                id: detail

                required property var modelData

                style: root.style
                pad: root.style.gap
                Layout.fillWidth: true
                implicitHeight: line.implicitHeight + 2 * detail.pad

                RowLayout {
                    id: line

                    anchors.fill: parent
                    spacing: root.style.gap

                    Text {
                        text: detail.modelData.k
                        textFormat: Text.PlainText
                        font: root.style.captionFont
                        color: root.style.inkSoft
                    }
                    Item {
                        Layout.fillWidth: true
                    }
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
