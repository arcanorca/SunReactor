/*
 * Nintendo DS touch screen, displays: one large tile per display (name, value,
 * and a chunky cell bar that takes touch, drag and keys), "All displays" first
 * once there is more than one. Unknown stays "--" with an inert bar. Sliders
 * write overrides (throttled), never on scroll. Module contract:
 * modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel"
import "Nds.js" as Nds

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

    NdsLid {
        id: surface

        width: parent.width
        theme: root.theme
        grid: root.theme.grid
        fillsWidth: true
        lidHeight: rows.height + 8
        regionName: "displays"

        Column {
            id: rows

            x: 4
            y: 4
            width: surface.artWidth - 8
            spacing: 1

            Repeater {
                model: root.rows

                delegate: Item {
                    id: tile

                    required property var modelData
                    required property int index

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

                    // Name line (rows 2-8), two rows of air, then the bar: its min and
                    // max marks sit two rows under the cells, so the tile ends at row 19.
                    width: rows.width
                    height: 20

                    Repeater {
                        model: Nds.box([], 0, 0, tile.width, tile.height, root.theme.bg, root.theme.borderInactive)
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
                        id: name

                        theme: root.theme
                        x: 4
                        y: 2
                        face: pstyle.bodyFace
                        capitals: true
                        color: pstyle.ink
                        text: tile.modelData.logicalId
                        maximumArtWidth: Math.max(8, value.x - x - 6)
                    }

                    PixelLabel {
                        id: value

                        theme: root.theme
                        x: tile.width - 4 - artWidth
                        y: 2
                        face: pstyle.bodyFace
                        capitals: true
                        color: tile.hint.length > 0 ? pstyle.dim
                            : (tile.modelData.hasOverride === true || slider.editing ? root.theme.accent : pstyle.ink)
                        text: tile.hint.length > 0 ? tile.hint : (slider.known ? slider.shown + "%" : "--")
                    }

                    PixelCellSlider {
                        id: slider

                        theme: root.theme
                        style: pstyle
                        cellHeight: 4
                        x: 4
                        y: 11
                        width: tile.width - 8
                        value: tile.percent
                        enabled: tile.controllable
                        minMark: tile.master ? -1 : tile.minPct
                        maxMark: tile.master ? -1 : tile.maxPct
                        accessibleName: tile.modelData.logicalId
                        onCommitted: v => tile.master
                            ? root.client.setGlobalOverride(v, root.settings.overrideMinutes)
                            : root.client.setMonitorOverride(tile.modelData.logicalId, v, root.settings.overrideMinutes)
                    }
                }
            }
        }
    }
}
