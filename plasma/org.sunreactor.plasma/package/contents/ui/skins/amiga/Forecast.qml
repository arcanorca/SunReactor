/*
 * Amiga Forecast: the next hours as a plain strip under the weather window -
 * hour, sky sprite and temperature per slot, as many slots as fit the window
 * (five at most). Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat
import "../pixel/Sprites.js" as Sprites

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property int maxSlots: 5
    readonly property int slotMin: 28
    readonly property var upcoming: client.forecast.filter(p => p.epochS > client.nowEpochS)
    readonly property bool available: client.isConnected && client.weatherEnabled && client.forecast.length > 0

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.settings.domain, "Title of the forecast window", "Forecast")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "forecast"

        AmigaWindow {
            id: panel

            readonly property int slots: Math.max(1, Math.min(root.maxSlots, Math.floor(panel.contentWidth / root.slotMin)))

            theme: root.theme
            style: tokens
            width: surface.artWidth
            plain: true
            contentHeight: 31

            Row {
                id: strip

                Repeater {
                    model: root.upcoming.slice(0, panel.slots)

                    delegate: Item {
                        id: slot

                        required property var modelData
                        required property int index

                        width: Math.floor(panel.contentWidth / panel.slots)
                        height: 31

                        Accessible.role: Accessible.StaticText
                        Accessible.name: PixelFormat.time(modelData.epochS) + " "
                            + Math.round(modelData.temperatureC) + "°"

                        AmigaText {
                            x: Math.floor((slot.width - artWidth) / 2)
                            y: 0
                            text: PixelFormat.hour(new Date(slot.modelData.epochS * 1000).getHours())
                            color: tokens.ink
                        }

                        ThemedWeatherIcon {
                            x: Math.floor((slot.width - width) / 2)
                            y: 8
                            theme: root.theme
                            size: 16
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight

                            PixelSprite {
                                source: Sprites.weatherAtlas()
                                frame: Sprites.conditionFrame(slot.modelData.condition, slot.modelData.isNight)
                                colors: tokens.spriteColors
                            }
                        }

                        AmigaText {
                            x: Math.floor((slot.width - artWidth) / 2)
                            y: 23
                            text: Math.round(slot.modelData.temperatureC) + "°"
                            color: tokens.ink
                            maximumArtWidth: slot.width
                        }
                    }
                }
            }
        }
    }
}
