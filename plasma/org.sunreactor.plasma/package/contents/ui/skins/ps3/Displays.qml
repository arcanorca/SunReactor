/*
 * Displays, PlayStation 3: the XMB item list of the brightness page. One item
 * per display, and an "All displays" item once there is more than one. Each
 * item shows its name and value, a thin slider and its state; the item the
 * pointer or focus is on is emphasised (see DisplayRow.qml). Sliders write
 * overrides, never on scroll.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "Plot.js" as Plot

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0

    spacing: 0

    // The emphasised item's halo reaches a gap past the first and last row:
    // the module keeps that room so its clip never cuts the halo.
    Item {
        Layout.fillWidth: true
        implicitHeight: root.style.gap
    }

    DisplayRow {
        visible: root.showAllDisplays
        style: root.style
        name: i18nd(root.domain, "All displays")
        percent: root.client.globalPercent
        stateText: root.client.isOverrideActive
            ? i18ndc(root.domain, "This brightness was set by hand", "Manual") : ""
        onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
    }

    Repeater {
        model: root.monitors

        delegate: DisplayRow {
            id: display

            required property var modelData

            readonly property bool unavailable: modelData.unreachable
                || modelData.topology === "temporarily_unavailable"

            style: root.style
            name: modelData.logicalId
            percent: modelData.percent
            target: Plot.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS)
            minPct: modelData.minPct !== undefined ? modelData.minPct : 15
            maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
            hasLimits: true
            hasOverride: modelData.hasOverride === true
            controllable: modelData.enabled && !unavailable
            stateText: {
                if (!modelData.enabled) {
                    return i18ndc(root.domain, "This display is excluded from automation", "Off");
                }
                if (unavailable) {
                    return i18ndc(root.domain, "This display is not answering", "Unavailable");
                }
                if (modelData.hasOverride) {
                    return i18ndc(root.domain, "This brightness was set by hand", "Manual");
                }
                return "";
            }

            onRequested: percent => root.client.setMonitorOverride(modelData.logicalId, percent,
                                                                   root.settings.overrideMinutes)
            onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId, minVal, maxVal)
            onClearOverrideRequested: () => root.client.clearMonitorOverride(modelData.logicalId)
        }
    }

    Item {
        Layout.fillWidth: true
        implicitHeight: root.style.gap
    }
}
