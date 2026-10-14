/*
 * Weather, PlayStation 3: the reading as a large light temperature beside its
 * line glyph, the condition under it, and what the reading does to the targets
 * in sub-text. The secondary readings are a vertical XMB list with thin
 * separators. Shown only when weather is enabled.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../components"

ColumnLayout {
    id: root

    // The PS3 side margin: the module's content keeps clear of the popup's edges.
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.leftMargin: root.style.sideMargin
    anchors.rightMargin: root.style.sideMargin

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XmbStyle style: XmbStyle {
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

    readonly property string effectText: {
        if (!client.hasWeatherReading) {
            return "";
        }
        if (client.weatherStale) {
            return i18nd(domain, "Last reading is out of date");
        }
        if (change >= 2) {
            return i18ndc(domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(domain, "Not changing brightness right now");
    }

    /*! The secondary readings, in the order of the list. */
    readonly property var details: {
        const d = client.weatherDetails;
        const out = [];
        if (d.feelsLikeC !== undefined) {
            out.push({ k: i18nd(domain, "Feels like"),
                       v: i18ndc(domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)) });
        }
        if (d.humidityPercent !== undefined) {
            out.push({ k: i18nd(domain, "Humidity"),
                       v: i18ndc(domain, "Percentage", "%1%", Math.round(d.humidityPercent)) });
        }
        if (d.windSpeedMps !== undefined) {
            out.push({ k: i18nd(domain, "Wind"),
                       v: i18ndc(domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)) });
        }
        if (d.airQualityIndex !== undefined) {
            out.push({ k: i18nd(domain, "Air quality"), v: String(Math.round(d.airQualityIndex)) });
        }
        return out;
    }

    WeatherText {
        id: weatherText
    }

    Layout.fillWidth: true
    spacing: style.gap * 0.5

    // The reading: the glyph, the temperature with the condition under it, and
    // what the reading does to the targets, in sub-text on the right.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        ThemedWeatherIcon {
            theme: root.theme
            size: root.style.snap(root.style.sizeHero * 1.0)
            condition: root.client.weatherCondition
            night: root.client.weatherIsNight
            Layout.alignment: Qt.AlignTop
            visible: root.client.hasWeatherReading
            WeatherGlyph {
                anchors.centerIn: parent
                size: root.style.snap(root.style.sizeHero * 1.0)
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
                glyphStyle: "line"
                palette: root.theme.glyphPalette
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                RowLayout {
                    visible: root.client.hasTemperature
                    spacing: root.style.gap * 0.5

                    Text {
                        text: String(Math.round(root.client.temperatureC))
                        textFormat: Text.PlainText
                        font: root.style.heroFont
                        color: root.style.ink
                        lineHeightMode: Text.FixedHeight
                        lineHeight: root.style.sizeHero
                    }

                    Text {
                        Layout.alignment: Qt.AlignBottom
                        Layout.bottomMargin: root.style.snap(root.style.sizeHero * 0.12)
                        text: "°C"
                        textFormat: Text.PlainText
                        font: root.style.unitFont
                        color: root.style.inkSoft
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                Text {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.maximumWidth: root.width * 0.5
                    visible: text.length > 0
                    text: root.effectText
                    textFormat: Text.PlainText
                    horizontalAlignment: Text.AlignRight
                    font: root.style.legendFont
                    color: root.style.inkSoft
                    elide: Text.ElideLeft
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.client.hasWeatherReading
                    ? (weatherText.conditionLabel(root.client.weatherCondition)
                       || i18nd(root.domain, "No weather reading"))
                    : root.stateNote
                textFormat: Text.PlainText
                font: root.style.labelFont
                color: root.client.hasWeatherReading ? root.style.ink : root.style.inkSoft
                elide: Text.ElideRight
            }
        }
    }

    // The secondary readings: a vertical XMB list.
    ColumnLayout {
        Layout.fillWidth: true
        visible: root.client.hasWeatherReading && root.details.length > 0
        spacing: 0

        Repeater {
            model: root.details

            delegate: XmbRow {
                required property var modelData

                style: root.style
                padding: root.style.snap(root.style.gap * 0.25)
                label: modelData.k
                value: modelData.v
            }
        }
    }
}
