/*
 * Nintendo DS touch screen, sun times: rise, solar noon and set as three
 * columns - the sky symbol over the time - with the next one in the accent.
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

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { label: i18ndc(domain, "Sunrise, compact", "Rise"), epoch: client.sunriseEpochS, symbol: "rise" },
        { label: i18ndc(domain, "Solar noon, compact", "Noon"), epoch: client.solarNoonEpochS, symbol: "noon" },
        { label: i18ndc(domain, "Sunset, compact", "Set"), epoch: client.sunsetEpochS, symbol: "set" },
    ]
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: entries.map(e => e.label + " " + PixelFormat.time(e.epoch)).join(", ")

    NdsLid {
        id: surface

        width: parent.width
        theme: root.theme
        grid: root.theme.grid
        fillsWidth: true
        lidHeight: 21
        regionName: "suntimes"

        // The list card: the three times sit on white.
        Repeater {
            model: Nds.box([], 4, 2, surface.artWidth - 8, 17, root.theme.bg, root.theme.borderInactive)
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

        Repeater {
            model: root.entries

            delegate: Item {
                id: column

                required property var modelData
                required property int index
                readonly property bool isNext: modelData.epoch === root.nextEpoch
                readonly property int colWidth: Math.floor((surface.artWidth - 8) / 3)

                x: 4 + index * colWidth
                y: 0
                width: colWidth
                height: surface.lidHeight

                Accessible.role: Accessible.ListItem
                Accessible.name: modelData.label + " " + PixelFormat.time(modelData.epoch)

                PixelSprite {
                    x: Math.floor((column.width - 8) / 2)
                    y: 3
                    source: Sprites.symbolAtlas()
                    frame: Sprites.symbol(column.modelData.symbol)
                    frameWidth: 8
                    frameHeight: 8
                    colors: { const c = pstyle.spriteColors.slice(); c[8] = "transparent"; c[4] = root.theme.textMuted; return c; }
                }
                PixelLabel {
                    theme: root.theme
                    x: Math.floor((column.width - artWidth) / 2)
                    y: 12
                    face: pstyle.bodyFace
                    color: column.isNext ? root.theme.accent : pstyle.ink
                    text: PixelFormat.time(column.modelData.epoch)
                }
            }
        }
    }
}
