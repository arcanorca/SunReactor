/*
 * Pixel Sun times: sunrise, solar noon and sunset with their symbols, the next
 * one emphasised, over a cell track of the day with the sun (or moon) riding
 * it. The sun's height only when the user asked for it.
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
    readonly property real dayProgress: {
        const rise = client.sunriseEpochS;
        const set = client.sunsetEpochS;
        if (rise <= 0 || set <= rise) {
            return 0;
        }
        return Math.max(0, Math.min(1, (client.nowEpochS - rise) / (set - rise)));
    }
    readonly property bool daylight: client.nowEpochS >= client.sunriseEpochS && client.nowEpochS < client.sunsetEpochS
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: entries.map(e => e.label + " " + PixelFormat.time(e.epoch)).join(", ")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "suntimes"

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the sun times panel", "Sun")
            contentHeight: 26 + (elevation.visible ? 10 : 0)

            Repeater {
                model: root.entries

                delegate: Item {
                    id: entry
                    required property var modelData
                    required property int index
                    readonly property bool isNext: modelData.epoch === root.nextEpoch
                    readonly property int colWidth: Math.floor(panel.contentWidth / 3)

                    x: index * colWidth
                    width: colWidth
                    height: 16

                    Row {
                        x: Math.floor((entry.width - width) / 2)
                        spacing: 3
                        PixelSprite {
                            source: Sprites.symbolAtlas()
                            frame: Sprites.symbol(entry.modelData.symbol)
                            frameWidth: 8
                            frameHeight: 8
                            colors: { const c = pstyle.spriteColors.slice(); c[8] = "transparent"; c[4] = pstyle.dim; return c; }
                        }
                        PixelLabel {
                            y: 1
                            theme: root.theme
                            face: pstyle.bodyFace
                            capitals: true
                            color: pstyle.dim
                            stipple: pstyle.stippleDim
                            text: entry.modelData.label
                        }
                    }
                    PixelLabel {
                        theme: root.theme
                        x: Math.floor((entry.width - artWidth) / 2)
                        y: 9
                        face: pstyle.bodyFace
                        color: entry.isNext ? pstyle.hot : pstyle.ink
                        text: PixelFormat.time(entry.modelData.epoch)
                    }
                }
            }

            // The day as a cell track; the sky symbol rides it.
            Item {
                id: track
                y: 18
                width: panel.contentWidth
                height: 8
                readonly property int cells: Math.max(4, Math.floor((width - 8) / 3))
                readonly property int lit: Math.round(root.dayProgress * cells)

                Repeater {
                    model: track.cells
                    delegate: Rectangle {
                        required property int index
                        x: 4 + index * 3
                        y: 3
                        width: 2
                        height: 2
                        antialiasing: false
                        color: index < track.lit ? (root.daylight ? pstyle.hot : pstyle.dim) : pstyle.ghost
                    }
                }
                PixelSprite {
                    x: Math.round(root.dayProgress * (track.width - 8))
                    y: 0
                    source: Sprites.symbolAtlas()
                    frame: Sprites.symbol(root.daylight ? "sun" : "moon")
                    frameWidth: 8
                    frameHeight: 8
                    colors: pstyle.spriteColors
                }
            }

            PixelLabel {
                id: elevation
                visible: root.showElevation
                theme: root.theme
                y: 28
                face: pstyle.bodyFace
                color: pstyle.dim
                maximumArtWidth: panel.contentWidth
                text: root.client.solarElevation >= 0
                    ? i18ndc(root.domain, "Placeholder is an angle in degrees", "Sun %1° above the horizon",
                             root.client.solarElevation.toFixed(1))
                    : i18ndc(root.domain, "Placeholder is an angle in degrees", "Sun %1° below the horizon",
                             (-root.client.solarElevation).toFixed(1))
            }
        }
    }
}
