/*
 * Amiga Sun: a plain strip of the three sun times (rise, noon, set) over a
 * recessed trough. The sun is the orange knob, placed by the day's progress
 * between sunrise and sunset; the solar elevation is a short figure at its
 * right. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat
import "AmigaChrome.js" as Chrome

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { label: i18ndc(root.domain, "Sunrise, compact", "Rise"), epoch: root.client.sunriseEpochS },
        { label: i18ndc(root.domain, "Solar noon, compact", "Noon"), epoch: root.client.solarNoonEpochS },
        { label: i18ndc(root.domain, "Sunset, compact", "Set"), epoch: root.client.sunsetEpochS },
    ]
    readonly property real dayProgress: {
        const rise = root.client.sunriseEpochS;
        const set = root.client.sunsetEpochS;
        if (set <= rise) {
            return 0;
        }
        return Math.max(0, Math.min(1, (root.client.nowEpochS - rise) / (set - rise)));
    }
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    AmigaStyle {
        id: tokens

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

        AmigaWindow {
            id: panel

            theme: root.theme
            style: tokens
            width: surface.artWidth
            plain: true
            contentHeight: 23

            Repeater {
                model: root.entries

                delegate: Item {
                    id: column

                    required property var modelData
                    required property int index

                    readonly property int colWidth: Math.floor(panel.contentWidth / 3)

                    x: index * colWidth
                    width: colWidth
                    height: 23

                    AmigaText {
                        x: Math.floor((column.width - artWidth) / 2)
                        y: 0
                        text: column.modelData.label
                        color: tokens.ink
                    }
                    AmigaText {
                        x: Math.floor((column.width - artWidth) / 2)
                        y: 8
                        text: PixelFormat.timeParts(column.modelData.epoch).hm
                        color: tokens.ink
                    }
                }
            }

            // The day's trough: the knob is the sun.
            Item {
                id: trough

                readonly property int elevationWidth: root.showElevation ? 32 : 0

                y: 16
                width: panel.contentWidth - elevationWidth - (root.showElevation ? 3 : 0)
                height: 7

                Repeater {
                    model: Chrome.troughRects(trough.width, { face: tokens.face, ink: tokens.ink, body: tokens.body })
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
                    model: root.available ? Chrome.knobRects(1 + Math.round(root.dayProgress * Math.max(0, trough.width - 11)),
                                                             { ink: tokens.ink, hot: tokens.hot, body: tokens.body }) : []
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
            }

            AmigaText {
                visible: root.showElevation
                x: panel.contentWidth - artWidth
                y: 16
                text: Math.round(root.client.solarElevation) + "°"
                color: tokens.ink
            }
        }
    }
}
