/*
 * Pixel Forecast: the next intervals the daemon already fetched - time,
 * recoloured sky sprite, temperature - in as many columns as fit (up to 5).
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "PixelFormat.js" as PixelFormat
import "Sprites.js" as Sprites

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property int maxSlots: 5
    readonly property int slotMin: 26
    readonly property int slots: Math.max(1, Math.min(maxSlots, Math.floor(panel.contentWidth / slotMin)))
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === slots) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && client.forecast.length > 0

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.List
    Accessible.name: i18ndc(settings.domain, "Title of the forecast panel", "Forecast")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "forecast"

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.settings.domain, "Title of the forecast panel", "Forecast")
            contentHeight: 5 + 1 + 16 + 1 + 7

            Repeater {
                model: root.points

                delegate: Item {
                    id: slot
                    required property var modelData
                    required property int index
                    readonly property int slotWidth: Math.floor(panel.contentWidth / root.slots)
                    /*! The slot's sky frame; the sprite takes it from here (not from itself, which would loop its colours). */
                    readonly property int skyFrame: Sprites.conditionFrame(modelData.condition, modelData.isNight)

                    x: index * slotWidth
                    width: slotWidth
                    height: panel.contentHeight

                    Accessible.role: Accessible.ListItem
                    Accessible.name: PixelFormat.time(modelData.epochS) + " " + Math.round(modelData.temperatureC) + "°"

                    PixelLabel {
                        theme: root.theme
                        x: Math.floor((slot.width - artWidth) / 2)
                        face: "tiny"
                        capitals: true
                        color: pstyle.dim
                        text: PixelFormat.time(slot.modelData.epochS)
                    }
                    PixelSprite {
                        x: Math.floor((slot.width - 16) / 2)
                        y: 6
                        visible: pstyle.spriteWeather
                        source: Sprites.weatherAtlas()
                        frame: slot.skyFrame
                        colors: pstyle.weatherColors(slot.skyFrame)
                    }
                    WeatherGlyph {
                        x: Math.floor((slot.width - 16) / 2)
                        y: 6
                        visible: !pstyle.spriteWeather
                        size: 16
                        condition: slot.modelData.condition
                        night: slot.modelData.isNight
                        glyphStyle: root.theme.glyphStyle
                        palette: root.theme.glyphColours(pstyle.paper, pstyle.ink)
                    }
                    PixelLabel {
                        theme: root.theme
                        x: Math.floor((slot.width - artWidth) / 2)
                        y: 23
                        face: pstyle.bodyFace
                        color: pstyle.ink
                        text: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                                     Math.round(slot.modelData.temperatureC))
                    }
                }
            }
        }
    }
}
