/*
 * Pixel Displays: one row per display - name, state tag or the way back to
 * automatic, the limits toggle and the value - over a discrete cell slider
 * that shows the display's min..max range. "All displays" leads once there is
 * more than one. Unknown stays "--" with an inert slider; sliders write
 * overrides (throttled), never on scroll. Module contract: modules/Modules.js.
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
            out.push({ master: true, logicalId: i18nd(domain, "All displays"), percent: client.globalPercent,
                       hasOverride: client.isOverrideActive, enabled: true, unreachable: false });
        }
        for (const m of monitors) {
            out.push(m);
        }
        return out;
    }

    PixelStyle {
        id: pstyle
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

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the displays panel", "Displays")
            contentHeight: column.height

            Column {
                id: column
                width: panel.contentWidth
                spacing: 2

                Repeater {
                    model: root.rows
                    delegate: Item {
                        id: row

                        required property var modelData
                        required property int index
                        property bool expanded: false
                        /*! The name / buttons / value line: the height of a framed button. */
                        readonly property int line: 11

                        readonly property bool master: modelData.master === true
                        readonly property bool unavailable: modelData.unreachable === true
                            || modelData.topology === "temporarily_unavailable"
                        readonly property bool controllable: modelData.enabled !== false && !unavailable
                        readonly property int percent: modelData.percent !== undefined ? modelData.percent : -1
                        readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                        readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                        readonly property string hint: {
                            if (modelData.enabled === false) {
                                return i18ndc(root.domain, "This display is excluded from automation", "Off");
                            }
                            if (unavailable) {
                                return i18ndc(root.domain, "This display is not answering, compact", "N/A");
                            }
                            return "";
                        }

                        width: panel.contentWidth
                        height: line + 1 + slider.height + (expanded && !master ? limits.height : 0)

                        PixelLabel {
                            id: name
                            theme: root.theme
                            y: Math.floor((row.line - capHeight) / 2)
                            text: row.modelData.logicalId
                            face: pstyle.bodyFace
                            color: pstyle.ink
                            maximumArtWidth: Math.max(8, rightGroup.x - 4)
                        }

                        Row {
                            id: rightGroup
                            x: row.width - width
                            spacing: 3

                            PixelButton {
                                visible: row.hint.length > 0
                                theme: root.theme
                                style: pstyle
                                variant: "tag"
                                tagColor: pstyle.dim
                                text: row.hint
                            }
                            PixelButton {
                                visible: row.modelData.hasOverride === true
                                theme: root.theme
                                style: pstyle
                                text: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                                accessibleName: i18ndc(root.domain, "@action:button Return to automatic brightness", "Automatic")
                                onClicked: row.master ? root.client.clearAllOverrides()
                                                      : root.client.clearMonitorOverride(row.modelData.logicalId)
                            }
                            PixelButton {
                                visible: !row.master && row.maxPct >= 0
                                theme: root.theme
                                style: pstyle
                                variant: "tab"
                                checked: row.expanded
                                text: i18ndc(root.domain, "Range limits, compact", "%1–%2", row.minPct, row.maxPct)
                                accessibleName: i18ndc(root.domain, "Range limits", "%1%–%2%", row.minPct, row.maxPct)
                                onClicked: row.expanded = !row.expanded
                            }
                            PixelDigits {
                                theme: root.theme
                                style: pstyle
                                y: Math.floor((row.line - height) / 2)
                                width: Math.max(artWidth, 18)
                                kind: pstyle.digits === "bitmap" ? "bitmap" : "block"
                                color: slider.editing || slider.activeFocus ? pstyle.hot : pstyle.ink
                                text: slider.known ? slider.shown + "%" : "--"
                            }
                        }

                        PixelCellSlider {
                            id: slider
                            theme: root.theme
                            style: pstyle
                            cellHeight: 5
                            x: 2
                            y: row.line + 1
                            width: row.width - 4
                            value: row.percent
                            enabled: row.controllable
                            minMark: row.master ? -1 : row.minPct
                            maxMark: row.master ? -1 : row.maxPct
                            accessibleName: row.modelData.logicalId
                            onCommitted: v => row.master
                                ? root.client.setGlobalOverride(v, root.settings.overrideMinutes)
                                : root.client.setMonitorOverride(row.modelData.logicalId, v, root.settings.overrideMinutes)
                        }

                        // Night minimum and day maximum, folded away until asked for.
                        Column {
                            id: limits
                            visible: row.expanded && !row.master
                            y: slider.y + slider.height + 2
                            width: row.width
                            spacing: 2

                            Repeater {
                                model: [
                                    { label: i18ndc(root.domain, "Night minimum boundary, compact", "Night"), symbol: "moon", which: 0 },
                                    { label: i18ndc(root.domain, "Day maximum boundary, compact", "Day"), symbol: "sun", which: 1 },
                                ]
                                delegate: Item {
                                    id: limit
                                    required property var modelData
                                    width: row.width
                                    height: 12
                                    readonly property int current: modelData.which === 0 ? row.minPct : row.maxPct

                                    PixelLabel {
                                        id: limitLabel
                                        theme: root.theme
                                        y: 1
                                        width: 34
                                        face: pstyle.bodyFace
                                        capitals: true
                                        color: pstyle.dim
                                        stipple: pstyle.stippleDim
                                        maximumArtWidth: 34
                                        text: limit.modelData.label
                                    }
                                    PixelCellSlider {
                                        id: limitSlider
                                        theme: root.theme
                                        style: pstyle
                                        x: 38
                                        y: 1
                                        width: row.width - 38 - 22
                                        cellHeight: 4
                                        value: limit.current
                                        accessibleName: limit.modelData.which === 0
                                            ? i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                                            : i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                                        onCommitted: v => limit.modelData.which === 0
                                            ? root.client.setMonitorLimits(row.modelData.logicalId, Math.min(v, row.maxPct), row.maxPct)
                                            : root.client.setMonitorLimits(row.modelData.logicalId, row.minPct, Math.max(v, row.minPct))
                                    }
                                    PixelLabel {
                                        theme: root.theme
                                        x: row.width - artWidth
                                        y: 1
                                        face: pstyle.bodyFace
                                        color: pstyle.ink
                                        text: limitSlider.shown + "%"
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
