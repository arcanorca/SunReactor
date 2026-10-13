/*
 * Nintendo DS touch screen, forecast: the next intervals the daemon already
 * fetched as white tiles - time, sky sprite, temperature - up to five across.
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel"
import "../pixel/PixelFormat.js" as PixelFormat
import "../pixel/Sprites.js" as Sprites
import "Nds.js" as Nds

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property int maxSlots: 5
    readonly property int slots: Math.max(1, Math.min(maxSlots, Math.floor((surface.artWidth - 6) / 26)))
    readonly property int tileWidth: Math.floor((surface.artWidth - 8 - (slots - 1) * 2) / slots)
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

    NdsLid {
        id: surface

        width: parent.width
        theme: root.theme
        grid: root.theme.grid
        fillsWidth: true
        lidHeight: 4 + 28 + 4
        regionName: "forecast"

        Repeater {
            model: root.points

            delegate: Item {
                id: slot

                required property var modelData
                required property int index

                x: 4 + index * (root.tileWidth + 2)
                y: 4
                width: root.tileWidth
                height: 28

                Accessible.role: Accessible.ListItem
                Accessible.name: PixelFormat.time(modelData.epochS) + " " + Math.round(modelData.temperatureC) + "°"

                Repeater {
                    model: Nds.box([], 0, 0, slot.width, slot.height, root.theme.bg, root.theme.borderInactive)
                    delegate: Rectangle {
                        required property var modelData

                        x: modelData.x
                        y: modelData.y
                        width: modelData.w
                        height: modelData.h
                        color: modelData.c
                        antialiasing: false
                    }
                }

                PixelLabel {
                    theme: root.theme
                    x: Math.floor((slot.width - artWidth) / 2)
                    y: 2
                    face: "tiny"
                    capitals: true
                    color: pstyle.dim
                    text: PixelFormat.time(slot.modelData.epochS)
                }
                PixelSprite {
                    x: Math.floor((slot.width - 16) / 2)
                    y: 6
                    source: Sprites.weatherAtlas()
                    frame: Sprites.conditionFrame(slot.modelData.condition, slot.modelData.isNight)
                    colors: pstyle.spriteColors
                }
                PixelLabel {
                    theme: root.theme
                    x: Math.floor((slot.width - artWidth) / 2)
                    y: 20
                    face: pstyle.bodyFace
                    color: pstyle.ink
                    text: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                                 Math.round(slot.modelData.temperatureC))
                }
            }
        }
    }
}
