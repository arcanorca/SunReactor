/*
 * Weather, Windows XP: a welcome pane. The temperature in a large figure with the
 * sky beside it, the brightness effect of the weather as a short green progress
 * bar, and the readings in a details list under a blue Details band. When the
 * forecast and the sun times are listed, the window hosts their tables too, so
 * the whole weather page is one card. Shown only when weather is enabled.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

WinWindow {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    style: ui
    width: parent ? parent.width : 0

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled
    title: i18ndc(domain, "Window title of the weather reading", "Weather")

    /*! The forecast and the sun status bar live in this card when they are listed. */
    readonly property bool hostsForecast: theme.modules.indexOf("forecast") >= 0
    readonly property bool hostsSun: theme.modules.indexOf("suntimes") >= 0

    WeatherText {
        id: weatherText
    }

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
            out.push({ k: i18ndc(domain, "Weather details: apparent temperature", "Feels like"),
                       v: i18ndc(domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)) });
        }
        if (d.humidityPercent !== undefined) {
            out.push({ k: i18ndc(domain, "Weather details: relative humidity", "Humidity"),
                       v: i18ndc(domain, "Percentage", "%1%", Math.round(d.humidityPercent)) });
        }
        if (d.windSpeedMps !== undefined) {
            out.push({ k: i18ndc(domain, "Weather details: wind speed", "Wind"),
                       v: i18ndc(domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)) });
        }
        if (d.airQualityIndex !== undefined) {
            out.push({ k: i18ndc(domain, "Weather details: US EPA air quality index", "Air quality"),
                       v: String(Math.round(d.airQualityIndex)) });
        }
        return out;
    }

    // The welcome pane: the reading and what it does to the targets.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.ui.gap

        Text {
            visible: root.client.hasTemperature
            Layout.alignment: Qt.AlignVCenter
            text: String(Math.round(root.client.temperatureC))
            textFormat: Text.PlainText
            font: root.ui.heroFont
            color: root.ui.ink
            lineHeight: root.ui.sizeHero
            lineHeightMode: Text.FixedHeight
        }
        Text {
            visible: root.client.hasTemperature
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: root.ui.snap(root.ui.sizeHero * 0.14)
            text: "°C"
            textFormat: Text.PlainText
            font: root.ui.boldFont
            color: root.ui.inkMuted
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Text {
                Layout.fillWidth: true
                text: root.client.hasWeatherReading
                    ? (weatherText.conditionLabel(root.client.weatherCondition) || i18nd(root.domain, "No weather reading"))
                    : root.stateNote
                textFormat: Text.PlainText
                font: root.ui.boldFont
                color: root.client.hasWeatherReading ? root.ui.ink : root.ui.inkMuted
                elide: Text.ElideRight
            }

            Row {
                visible: root.client.hasWeatherReading && !root.client.weatherStale
                spacing: root.ui.gap

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.change >= 2 ? i18ndc(root.domain, "Weather effect: the displays are dimmed", "Dimming")
                        : (root.change <= -2 ? i18ndc(root.domain, "Weather effect: the displays are brightened", "Boosting")
                                             : i18ndc(root.domain, "Weather effect: none", "No effect"))
                    textFormat: Text.PlainText
                    font: root.ui.smallFont
                    color: root.ui.ink
                }

                // Ten green blocks: the weather multiplier, shown when it changes the targets.
                Row {
                    id: effect

                    anchors.verticalCenter: parent.verticalCenter
                    spacing: root.ui.snap(3)
                    visible: Math.abs(root.change) >= 2

                    readonly property int lit: Math.round(Math.max(0, Math.min(1, root.client.weatherMultiplier)) * 10)

                    Repeater {
                        model: 10

                        delegate: Rectangle {
                            required property int index

                            width: root.ui.snap(7)
                            height: root.ui.snap(root.ui.sizeBody * 0.6)
                            color: index < effect.lit ? root.ui.green : root.ui.trough
                            border.width: root.ui.line
                            border.color: root.ui.etchDark
                        }
                    }
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: Math.abs(root.change) >= 2
                    text: root.change >= 2 ? i18ndc(root.domain, "Percent points removed", "−%1%", root.change)
                                           : i18ndc(root.domain, "Percent points added", "+%1%", -root.change)
                    textFormat: Text.PlainText
                    font: root.ui.figureFont
                    color: root.ui.ink
                }
            }

            Text {
                visible: root.client.hasWeatherReading && root.client.weatherStale
                text: i18ndc(root.domain, "Weather reading is out of date", "Out of date")
                textFormat: Text.PlainText
                font: root.ui.smallFont
                color: root.ui.inkMuted
            }
        }
    }

    // Details: a list view with a blue band over the rows.
    ColumnLayout {
        Layout.fillWidth: true
        visible: root.extras.length > 0 && root.client.hasWeatherReading
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: root.ui.rowH
            border.width: root.ui.line
            border.color: root.ui.frameDeep
            gradient: Gradient {
                GradientStop { position: 0; color: root.ui.frameLight }
                GradientStop { position: 1; color: root.ui.frame }
            }

            Text {
                x: root.ui.pad
                anchors.verticalCenter: parent.verticalCenter
                text: i18ndc(root.domain, "Weather details band caption", "Details")
                textFormat: Text.PlainText
                font: root.ui.smallBoldFont
                color: root.ui.white
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: list.implicitHeight + 2 * root.ui.line
            color: root.ui.white
            border.width: root.ui.line
            border.color: root.ui.clientEdge

            Column {
                id: list

                x: root.ui.line
                y: root.ui.line
                width: parent.width - 2 * root.ui.line

                Repeater {
                    model: root.extras

                    delegate: Item {
                        id: entry

                        required property var modelData
                        required property int index

                        width: list.width
                        height: root.ui.rowH

                        Text {
                            x: root.ui.pad
                            anchors.verticalCenter: parent.verticalCenter
                            width: entry.width * 0.5 - root.ui.pad
                            text: entry.modelData.k
                            textFormat: Text.PlainText
                            font: root.ui.bodyFont
                            color: root.ui.ink
                            elide: Text.ElideRight
                        }
                        Rectangle {
                            x: entry.width * 0.5
                            width: root.ui.hairline
                            height: parent.height
                            color: root.ui.gridline
                        }
                        Text {
                            x: entry.width * 0.5 + root.ui.pad
                            anchors.verticalCenter: parent.verticalCenter
                            width: entry.width * 0.5 - root.ui.pad * 2
                            text: entry.modelData.v
                            textFormat: Text.PlainText
                            font: root.ui.figureFont
                            color: root.ui.ink
                            elide: Text.ElideRight
                        }
                        Rectangle {
                            y: parent.height - root.ui.hairline
                            width: entry.width
                            height: root.ui.hairline
                            color: root.ui.gridline
                        }
                    }
                }
            }
        }
    }

    ForecastTable {
        Layout.fillWidth: true
        visible: root.hostsForecast && root.available && available
        theme: root.theme
        client: root.client
        settings: root.settings
    }

    SunStatus {
        Layout.fillWidth: true
        visible: root.hostsSun && root.available && available
        theme: root.theme
        client: root.client
        settings: root.settings
    }
}
