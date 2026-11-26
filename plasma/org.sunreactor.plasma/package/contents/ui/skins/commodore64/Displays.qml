/*
 * Commodore 64 Displays: one line per display, `MI-MONITOR      39%`, and under it
 * the PETSCII block gauge that sets its brightness. With several displays an
 * ALL DISPLAYS line comes first. A held-by-hand value shows a CLR keycap that
 * drops the override. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "Petscii.js" as Petscii
import "../../modules/Monitors.js" as Monitors

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: Monitors.hasStatus(client) && client.monitors ? client.monitors : []
    readonly property bool available: Monitors.hasDisplayData(client)

    C64Style {
        id: style

        theme: root.theme
    }

    /*! The rows: an ALL DISPLAYS line when there are several, then one per display. */
    readonly property var rowsModel: Monitors.rowsWithMaster(
        monitors, client.globalPercent, client.isOverrideActive, settings.allDisplaysLabel)
    property var expandedLimits: ({})

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Title of the displays panel", "Displays")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + style.padTop + style.padBottom
        regionName: "c64-displays"

        readonly property int cols: style.cols(artWidth)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.padTop
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: "LOAD \"" + i18ndc(root.domain, "C64 BASIC header label for Displays section", "DISPLAYS") + "\",8,1"
                maximumArtWidth: body.width
            }

            Repeater {
                model: surface.cols > 0 ? root.rowsModel : []

                delegate: Column {
                    id: monitorRow

                    required property var modelData
                    required property int index
                    readonly property bool expanded: !master && Monitors.isExpanded(root.expandedLimits, modelData.logicalId)

                    readonly property bool master: modelData.master === true
                    readonly property bool unavailable: Monitors.unavailable(modelData)
                    readonly property bool offMark: modelData.enabled === false || unavailable
                    readonly property bool controllable: root.client.isConnected && !offMark
                    readonly property int percent: modelData.percent !== undefined ? modelData.percent : -1
                    readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                    readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                    readonly property bool hasLimits: !master && Monitors.hasValidLimits(modelData)
                    readonly property bool overridden: modelData.hasOverride === true
                    readonly property string notice: master ? "" : Monitors.monitorNotice(
                        modelData, root.settings.contestedNotice, root.settings.deferredNotice)
                    readonly property string noticeTag: master ? "" : Monitors.monitorTag(
                        modelData, root.settings.pausedTag, root.settings.queuedTag)
                    readonly property int tagCols: noticeTag.length > 0 ? Petscii.cells(noticeTag) + 1 : 0
                    /*! Name, optional notice tag, then the 4-cell percent, a gap, CLR key (3 cells) when held, RNG key (3 cells) for limits. */
                    readonly property int nameCols: Math.max(4, surface.cols - 5 - tagCols - (overridden ? 4 : 0) - (hasLimits ? 4 : 0))

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
                            visible: monitorRow.noticeTag.length > 0
                            width: monitorRow.tagCols * style.cw
                            height: style.ch

                            C64Text {
                                ink: style.hot
                                text: monitorRow.noticeTag
                            }
                            HoverHandler {
                                id: c64TagHover
                            }
                            QQC2.ToolTip.visible: c64TagHover.hovered && monitorRow.notice.length > 0
                            QQC2.ToolTip.text: monitorRow.notice
                            QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
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
                            enabled: root.client.isConnected
                            text: "CLR"
                            fill: style.ink
                            ink: style.screen
                            Accessible.name: i18ndc(root.domain, "Accessible name of the key that clears a manual brightness", "Clear")

                            onActivated: monitorRow.master
                                ? root.client.clearAllOverrides()
                                : root.client.clearMonitorOverride(monitorRow.modelData.logicalId)
                        }
                        Item {
                            visible: monitorRow.overridden && monitorRow.hasLimits
                            width: style.cw
                            height: style.ch
                        }
                        C64Keycap {
                            visible: monitorRow.hasLimits
                            text: "RNG"
                            fill: monitorRow.expanded ? style.hot : style.ink
                            ink: style.screen
                            Accessible.name: i18ndc(root.domain, "Range limits", "%1%–%2%", monitorRow.minPct, monitorRow.maxPct)

                            onActivated: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, monitorRow.modelData.logicalId)
                        }
                    }

                    Item {
                        width: body.width
                        height: style.ch

                        C64Gauge {
                            visible: !monitorRow.offMark
                            cells: surface.cols - 2
                            cw: style.cw
                            value: monitorRow.percent < 0 ? 0 : monitorRow.percent
                            active: monitorRow.controllable && monitorRow.percent >= 0
                            ink: style.ink
                            quiet: style.quiet
                            accessibleName: monitorRow.modelData.logicalId

                            onCommitted: v => monitorRow.master
                                ? root.client.setGlobalOverride(v, root.settings.overrideMinutes)
                                : root.client.setMonitorOverride(monitorRow.modelData.logicalId, v,
                                                                 root.settings.overrideMinutes)
                        }
                        C64Text {
                            visible: monitorRow.offMark
                            ink: style.quiet
                            text: monitorRow.unavailable ? "NO SIGNAL" : "OFF"
                        }
                    }

                    Column {
                        visible: monitorRow.hasLimits && monitorRow.expanded
                        width: body.width

                        Repeater {
                            model: [
                                { label: "MIN", which: 0 },
                                { label: "MAX", which: 1 },
                            ]

                            delegate: Row {
                                id: limitRow

                                required property var modelData
                                readonly property int current: modelData.which === 0 ? monitorRow.minPct : monitorRow.maxPct

                                height: style.ch

                                C64Text {
                                    ink: style.quiet
                                    text: limitRow.modelData.label + " "
                                }
                                C64Gauge {
                                    id: limitGauge
                                    cells: Math.max(4, surface.cols - 10)
                                    cw: style.cw
                                    from: limitRow.modelData.which === 0 ? 0 : monitorRow.minPct
                                    to: limitRow.modelData.which === 0 ? monitorRow.maxPct : 100
                                    value: limitRow.current
                                    active: root.client.isConnected && monitorRow.hasLimits
                                    ink: style.ink
                                    quiet: style.quiet
                                    accessibleName: limitRow.modelData.which === 0
                                        ? i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                                        : i18ndc(root.domain, "Day maximum boundary", "Day maximum")

                                    onCommitted: v => limitRow.modelData.which === 0
                                        ? root.client.setMonitorLimits(monitorRow.modelData.logicalId,
                                                                       Math.min(v, monitorRow.maxPct), monitorRow.maxPct)
                                        : root.client.setMonitorLimits(monitorRow.modelData.logicalId,
                                                                       monitorRow.minPct, Math.max(v, monitorRow.minPct))
                                }
                                C64Text {
                                    ink: style.hot
                                    text: (limitGauge.shownValue + "%").padStart(4, " ")
                                }
                            }
                        }
                    }

                }
            }
        }
    }
}
