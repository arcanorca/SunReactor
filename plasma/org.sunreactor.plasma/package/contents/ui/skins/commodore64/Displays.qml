/*
 * Commodore 64 Displays: one line per display, `MI-MONITOR      39%`, and under it
 * the PETSCII block gauge that sets its brightness. With several displays an
 * ALL DISPLAYS line comes first. A held-by-hand value shows a CLR keycap that
 * drops the override. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: client.isConnected && client.monitors ? client.monitors : []
    readonly property bool available: client.isConnected && monitors.length > 0

    C64Style {
        id: style

        theme: root.theme
    }

    /*! The rows: an ALL DISPLAYS line when there are several, then one per display. */
    readonly property var rowsModel: {
        const out = [];
        if (monitors.length > 1) {
            out.push({ master: true, logicalId: i18nd(root.domain, "All displays"), percent: client.globalPercent,
                       hasOverride: client.isOverrideActive, enabled: true, unreachable: false });
        }
        for (const monitor of monitors) {
            out.push(monitor);
        }
        return out;
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Title of the displays panel", "Displays")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + 2 * style.margin
        regionName: "c64-displays"

        readonly property int cols: style.cols(artWidth)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.margin
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: "LOAD \"MI-MONITOR\",8,1"
            }

            Repeater {
                model: surface.cols > 0 ? root.rowsModel : []

                delegate: Column {
                    id: monitorRow

                    required property var modelData
                    required property int index

                    readonly property bool master: modelData.master === true
                    readonly property bool unavailable: modelData.unreachable === true
                        || modelData.topology === "temporarily_unavailable"
                    readonly property bool controllable: modelData.enabled !== false && !unavailable
                    readonly property int percent: modelData.percent !== undefined ? modelData.percent : -1
                    readonly property bool overridden: modelData.hasOverride === true
                    readonly property int nameCols: surface.cols - 5 - (overridden ? 5 : 0)

                    width: body.width

                    Row {
                        height: style.ch

                        Item {
                            width: monitorRow.nameCols * style.cw
                            height: style.ch

                            C64Text {
                                ink: style.ink
                                text: Petscii.fit(monitorRow.modelData.logicalId, monitorRow.nameCols)
                            }
                        }
                        Item {
                            width: 4 * style.cw
                            height: style.ch

                            C64Text {
                                ink: style.hot
                                text: (monitorRow.percent >= 0 ? monitorRow.percent + "%" : "--").padStart(4, " ")
                            }
                        }
                        Item {
                            width: style.cw
                            height: style.ch
                        }
                        C64Keycap {
                            visible: monitorRow.overridden
                            text: "CLR"
                            fill: style.ink
                            ink: style.screen
                            Accessible.name: i18ndc(root.domain, "Accessible name of the key that clears a manual brightness", "Clear")

                            onActivated: monitorRow.master
                                ? root.client.clearAllOverrides()
                                : root.client.clearMonitorOverride(monitorRow.modelData.logicalId)
                        }
                    }

                    Item {
                        width: body.width
                        height: style.ch

                        C64Gauge {
                            visible: monitorRow.controllable
                            cells: surface.cols - 2
                            value: monitorRow.percent < 0 ? 0 : monitorRow.percent
                            active: monitorRow.controllable
                            ink: style.ink
                            quiet: style.quiet
                            accessibleName: monitorRow.modelData.logicalId

                            onCommitted: v => monitorRow.master
                                ? root.client.setGlobalOverride(v, root.settings.overrideMinutes)
                                : root.client.setMonitorOverride(monitorRow.modelData.logicalId, v,
                                                                 root.settings.overrideMinutes)
                        }
                        C64Text {
                            visible: !monitorRow.controllable
                            ink: style.quiet
                            text: monitorRow.unavailable ? "NO SIGNAL" : "OFF"
                        }
                    }

                }
            }
        }
    }
}
