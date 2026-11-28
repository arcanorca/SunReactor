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
import "../../components/weathericons"
import "../../modules"
import "../../components"
import "../../Format.js" as Format
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

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

    readonly property string effect: weatherText.effect(client)

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Title of the weather panel", "Weather")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + style.padTop + style.padBottom
        regionName: "c64-weather"

        readonly property int cols: style.cols(artWidth)
        readonly property int spriteCols: 5
        readonly property int infoCols: cols - spriteCols - 1

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.padTop
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: root.client.locationCity.length > 0
                    ? "LOAD \"" + root.client.locationCity.toUpperCase() + "\",8,1"
                    : "LOAD \"WEATHER\",8,1"
                maximumArtWidth: body.width
                maximumLines: 1
            }

            Row {
                id: block

                spacing: style.cw

                // The Petscii picture in the "auto" style; the chosen Weather icons style
                // is drawn as a square of the same footprint in its place.
                ThemedWeatherIcon {
                    visible: root.theme.weatherIconStyle !== "auto"
                    theme: root.theme
                    condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
                    night: root.client.hasWeatherReading && root.client.weatherIsNight
                    size: Math.max(sprite.width, sprite.height)
                }

                Column {
                    id: sprite

                    visible: root.theme.weatherIconStyle === "auto"
                    width: surface.spriteCols * style.cw

                    Repeater {
                        model: Petscii.sprite(root.client.hasWeatherReading ? root.client.weatherCondition : "",
                                              root.client.weatherIsNight)

                        // The three-cell picture centred in the five-cell column.
                        delegate: C64Text {
                            required property string modelData

                            ink: style.hot
                            text: " " + modelData + " "
                        }
                    }
                }

                Column {
                    width: surface.infoCols * style.cw

                    C64Text {
                        ink: style.hot
                        text: "TEMP " + (root.client.hasTemperature
                            ? Format.tempText(root.client.temperatureC, root.client.isFahrenheit)
                            : "--" + Format.tempUnit(root.client.isFahrenheit))
                        maximumArtWidth: surface.infoCols * style.cw
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
