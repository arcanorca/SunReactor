/*
 * Commodore 64 Forecast: five slots across the screen, each the time, then a
 * one-cell condition icon and the temperature. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property bool available: client.isConnected && client.weatherEnabled && client.forecast.length > 0
    /*! Up to five slots; fewer when the popup is narrow (at least four cells each). */
    readonly property int slotCount: 5
    readonly property var points: client.forecast.slice(0, slotCount)

    C64Style {
        id: style

        theme: root.theme
    }

    /*! The condition icon and the temperature in a slot of `cells`, one cell kept
     *  clear on the right; the degree sign goes first when the slot is narrow. */
    function slotTemperature(point, cells) {
        // The glyph stands for the condition in "auto"; a chosen style draws the icon above.
        const icon = root.theme.weatherIconStyle === "auto" ? Petscii.icon(point.condition, point.isNight) : "";
        const degrees = Math.round(point.temperatureC);
        const full = icon + degrees + "°";
        return Petscii.cells(full) <= cells - 1 ? full : Petscii.fit(icon + degrees, cells - 1);
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(settings.domain, "Title of the forecast panel", "Forecast")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + style.padTop + style.padBottom
        regionName: "c64-forecast"

        readonly property int cols: style.cols(artWidth)
        readonly property int shown: Math.max(1, Math.min(root.points.length, Math.floor(cols / 4)))
        readonly property int slotCols: Math.floor(cols / shown)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.padTop
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: "LOAD \"FORECAST\",8,1"
                maximumArtWidth: body.width
            }

            Row {
                Repeater {
                    model: root.points.slice(0, surface.shown)

                    delegate: Column {
                        id: slot

                        required property var modelData
                        required property int index

                        width: surface.slotCols * style.cw

                        C64Text {
                            ink: style.hot
                            text: PixelFormat.hour(new Date(slot.modelData.epochS * 1000).getHours())
                            maximumArtWidth: slot.width
                        }
                        // The chosen Weather icons style, two text rows high; "auto" uses the glyph.
                        ThemedWeatherIcon {
                            visible: root.theme.weatherIconStyle !== "auto"
                            theme: root.theme
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight === true
                            size: style.ch * 2
                        }
                        C64Text {
                            ink: style.ink
                            text: root.slotTemperature(slot.modelData, slot.width / style.cw)
                            maximumArtWidth: slot.width
                        }
                    }
                }
            }
        }
    }
}
