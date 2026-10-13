/*
 * Commodore 64 Forecast: five slots across the screen, each the time, then a
 * one-cell condition icon and the temperature. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property bool available: client.isConnected && client.weatherEnabled && client.forecast.length > 0
    readonly property int slotCount: 5
    readonly property var points: client.forecast.slice(0, slotCount)

    C64Style {
        id: style

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(settings.domain, "Title of the forecast panel", "Forecast")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + 2 * style.margin
        regionName: "c64-forecast"

        readonly property int cols: style.cols(artWidth)
        readonly property int slotCols: Math.floor(cols / root.slotCount)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.margin
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: "LOAD \"FORECAST\",8,1"
            }

            Row {
                Repeater {
                    model: root.points

                    delegate: Column {
                        id: slot

                        required property var modelData
                        required property int index

                        width: surface.slotCols * style.cw

                        C64Text {
                            ink: style.hot
                            text: PixelFormat.time(slot.modelData.epochS)
                        }
                        C64Text {
                            ink: style.ink
                            text: Petscii.icon(slot.modelData.condition, slot.modelData.isNight) + " "
                                + Math.round(slot.modelData.temperatureC) + "°"
                        }
                    }
                }
            }
        }
    }
}
