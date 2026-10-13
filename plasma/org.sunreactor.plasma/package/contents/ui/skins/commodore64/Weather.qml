/*
 * Commodore 64 Weather: a PETSCII sun, moon, cloud or rain sprite built from
 * graphics characters, beside the temperature, the condition (wrapped over two
 * lines when it must) and what the weather does to the displays. Module
 * contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled

    WeatherText {
        id: weatherText
    }

    C64Style {
        id: style

        theme: root.theme
    }

    readonly property string condition: {
        const text = weatherText.conditionLabel(client.weatherCondition);
        return client.hasWeatherReading && text.length > 0 ? text : i18nd(root.domain, "No weather reading");
    }

    readonly property string effect: {
        if (!client.hasWeatherReading) {
            switch (client.weatherState) {
            case "no_api_key":
                return i18nd(root.domain, "No API key is configured");
            case "loading":
                return i18nd(root.domain, "Looking up the current conditions…");
            case "unauthorized":
                return i18nd(root.domain, "The weather provider rejected the API key");
            case "rate_limited":
                return i18nd(root.domain, "The weather provider is rate limiting requests");
            case "network_error":
                return i18nd(root.domain, "The weather provider could not be reached");
            default:
                return i18nd(root.domain, "No reading yet");
            }
        }
        if (client.weatherStale) {
            return i18nd(root.domain, "Last reading is out of date");
        }
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
        if (change >= 2) {
            return i18ndc(root.domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(root.domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(root.domain, "Not changing brightness right now");
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Title of the weather panel", "Weather")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + 2 * style.margin
        regionName: "c64-weather"

        readonly property int cols: style.cols(artWidth)
        readonly property int spriteCols: 5
        readonly property int infoCols: cols - spriteCols - 1

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.margin
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: "LOAD \"WEATHER\",8,1"
            }

            Row {
                id: block

                spacing: style.cw

                Column {
                    id: sprite

                    width: surface.spriteCols * style.cw

                    Repeater {
                        model: Petscii.sprite(root.client.hasWeatherReading ? root.client.weatherCondition : "",
                                              root.client.weatherIsNight)

                        delegate: C64Text {
                            required property string modelData

                            ink: style.hot
                            text: modelData
                        }
                    }
                }

                Column {
                    width: surface.infoCols * style.cw

                    C64Text {
                        ink: style.hot
                        text: "TEMP " + (root.client.hasTemperature
                            ? Math.round(root.client.temperatureC) + "°C" : "--°C")
                    }
                    C64Text {
                        ink: style.ink
                        text: root.condition
                        maximumArtWidth: surface.infoCols * style.cw
                        maximumLines: 2
                    }
                    C64Text {
                        ink: style.quiet
                        text: root.effect
                        maximumArtWidth: surface.infoCols * style.cw
                        maximumLines: 2
                    }
                }
            }
        }
    }
}
