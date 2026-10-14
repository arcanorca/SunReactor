/*
 * Amiga Displays: a Workbench window with one row per display. Each row: the
 * name on top (elided with the face's ellipsis); under it the proportional
 * gadget with the range ticks, and on its line the value in Workbench digits
 * and the gadgets - AUTO when a setting was made by hand (back to automatic),
 * LIM to open the night and day limits, which open below the row. Sliders write
 * overrides, throttled; unknown values show "--" and an inert gadget. Module
 * contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool available: client.isConnected && monitors.length > 0
    readonly property var rows: {
        const out = [];
        if (monitors.length > 1) {
            out.push({ master: true, logicalId: i18nd(root.domain, "All displays"), percent: root.client.globalPercent,
                       hasOverride: root.client.isOverrideActive, enabled: true, unreachable: false });
        }
        for (const m of monitors) {
            out.push(m);
        }
        return out;
    }

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "displays"

        AmigaWindow {
            id: panel

            theme: root.theme
            style: tokens
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the displays window", "Displays")
            contentHeight: column.height

            Column {
                id: column

                width: panel.contentWidth
                spacing: 0

                Repeater {
                    model: root.rows

                    delegate: Item {
                        id: row

                        required property var modelData
                        required property int index
                        property bool expanded: false

                        readonly property bool master: modelData.master === true
                        readonly property bool unavailable: modelData.unreachable === true
                            || modelData.topology === "temporarily_unavailable"
                        readonly property bool controllable: modelData.enabled !== false && !unavailable
                        readonly property int percent: modelData.percent !== undefined ? modelData.percent : -1
                        readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                        readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                        readonly property bool offMark: modelData.enabled === false || unavailable

                        // The name on the top line (rows 0-7); the value and gadgets, and the
                        // slider under them, on the line below from row 8, so the name never
                        // meets a gadget and may use the whole width. A row is 19 art px.
                        width: panel.contentWidth
                        height: expanded && !master ? limits.y + limits.height : 19

                        AmigaText {
                            id: name

                            x: 0
                            y: 0
                            text: row.modelData.logicalId
                            color: tokens.ink
                            maximumArtWidth: Math.max(8, row.width - 4)
                        }

                        Row {
                            id: rightGroup

                            x: row.width - width
                            y: 8
                            spacing: 2

                            AmigaText {
                                y: 2
                                text: row.offMark ? i18ndc(root.domain, "This display is excluded or not answering", "Off")
                                    : (slider.known ? slider.shown + "%" : "--")
                                color: slider.editing || slider.activeFocus ? tokens.hot : tokens.ink
                            }
                            AmigaGadget {
                                visible: row.modelData.hasOverride === true
                                theme: root.theme
                                style: tokens
                                text: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                                onActivated: row.master ? root.client.clearAllOverrides()
                                                        : root.client.clearMonitorOverride(row.modelData.logicalId)
                            }
                            AmigaGadget {
                                visible: !row.master && row.maxPct >= 0
                                theme: root.theme
                                style: tokens
                                lit: row.expanded
                                text: i18ndc(root.domain, "Range limits, compact", "Lim")
                                onActivated: row.expanded = !row.expanded
                            }
                        }

                        AmigaSlider {
                            id: slider

                            theme: root.theme
                            style: tokens
                            y: 10
                            width: Math.max(16, row.width - rightGroup.width - 4)
                            value: row.percent
                            enabled: row.controllable
                            minMark: row.master ? -1 : row.minPct
                            maxMark: row.master ? -1 : row.maxPct
                            accessibleName: row.modelData.logicalId
                            onCommitted: v => row.master
                                ? root.client.setGlobalOverride(v, root.settings.overrideMinutes)
                                : root.client.setMonitorOverride(row.modelData.logicalId, v, root.settings.overrideMinutes)
                        }

                        // Night minimum and day maximum, opened by LIM, below the row.
                        Column {
                            id: limits

                            visible: row.expanded && !row.master
                            y: 19
                            width: row.width
                            spacing: 2

                            Repeater {
                                model: [
                                    { label: i18ndc(root.domain, "Night minimum boundary, compact", "Night"), which: 0 },
                                    { label: i18ndc(root.domain, "Day maximum boundary, compact", "Day"), which: 1 },
                                ]

                                delegate: Item {
                                    id: limit

                                    required property var modelData
                                    readonly property int current: modelData.which === 0 ? row.minPct : row.maxPct

                                    width: row.width
                                    height: 11

                                    AmigaText {
                                        id: limitLabel

                                        y: 2
                                        text: limit.modelData.label
                                        color: tokens.ink
                                    }
                                    AmigaSlider {
                                        id: limitSlider

                                        theme: root.theme
                                        style: tokens
                                        x: limitLabel.artWidth + 4
                                        y: 1
                                        width: row.width - x - 26
                                        value: limit.current
                                        accessibleName: limit.modelData.label
                                        onCommitted: v => limit.modelData.which === 0
                                            ? root.client.setMonitorLimits(row.modelData.logicalId, Math.min(v, row.maxPct), row.maxPct)
                                            : root.client.setMonitorLimits(row.modelData.logicalId, row.minPct, Math.max(v, row.minPct))
                                    }
                                    AmigaText {
                                        x: row.width - artWidth
                                        y: 2
                                        text: limitSlider.shown + "%"
                                        color: tokens.ink
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
