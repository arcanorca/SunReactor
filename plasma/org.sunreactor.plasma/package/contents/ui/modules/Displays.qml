/*
 * Displays module: one row per display (name, value, override, limits) and an
 * "All displays" master row once there is more than one. Unknown values stay
 * "—" with a disabled slider; sliders write overrides, never on scroll.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../engine"
import "../components"
import "Monitors.js" as Monitors

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    /*! A master slider only earns its place once there is more than one display. */
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: Monitors.hasDisplayData(client)
    property var expandedLimits: ({})

    spacing: root.theme.spacing

    DisplayItem {
        visible: root.showAllDisplays
        theme: root.theme
        text: root.settings.allDisplaysLabel
        iconName: "brightness-high"
        percent: root.client.globalPercent
        controllable: root.client.isConnected
        hint: root.client.isOverrideActive ? i18ndc(root.domain, "This brightness was set by hand", "Manual") : ""
        onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.leftMargin: root.theme.spacing
        Layout.rightMargin: root.theme.spacing
        implicitHeight: root.theme.hairline
        color: root.theme.line
        visible: root.showAllDisplays
    }

    Repeater {
        model: root.monitors

        delegate: DisplayItem {
            id: display

            required property var modelData

            readonly property bool unavailable: Monitors.unavailable(modelData)
            readonly property string fullNotice: Monitors.monitorNotice(
                modelData, root.settings.contestedNotice, root.settings.deferredNotice)
            readonly property string tagNotice: Monitors.monitorTag(
                modelData, root.settings.pausedTag, root.settings.queuedTag)

            theme: root.theme
            text: modelData.logicalId
            iconName: "video-display-brightness"
            percent: modelData.percent
            minPct: modelData.minPct !== undefined ? modelData.minPct : -1
            maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
            hasLimits: Monitors.hasValidLimits(modelData)
            limitsExpanded: Monitors.isExpanded(root.expandedLimits, modelData.logicalId)
            externalHold: modelData.externalHold !== undefined ? modelData.externalHold : null
            hasOverride: modelData.hasOverride === true
            controllable: root.client.isConnected && modelData.enabled && !unavailable
            notice: fullNotice
            hint: {
                if (!modelData.enabled) {
                    return i18ndc(root.domain, "This display is excluded from automation", "Off");
                }
                if (unavailable) {
                    return i18ndc(root.domain, "This display is not answering", "Unavailable");
                }
                if (tagNotice.length > 0) {
                    return tagNotice;
                }
                if (modelData.hasOverride) {
                    return i18ndc(root.domain, "This brightness was set by hand", "Manual");
                }
                return "";
            }

            onRequested: percent => root.client.setMonitorOverride(modelData.logicalId, percent,
                                                                   root.settings.overrideMinutes)
            onLimitsToggled: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, modelData.logicalId)
            onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId, minVal, maxVal)
            onClearOverrideRequested: () => root.client.clearMonitorOverride(modelData.logicalId)
            onClearExternalHoldRequested: () => root.client.clearExternalHold(modelData.logicalId)

            Connections {
                target: root.client

                function resetExternalHoldDismissal() {
                    display.externalHoldDismissed = false;
                }

                function onMonitorsChanged() {
                    resetExternalHoldDismissal();
                }

                function onNowChanged() {
                    resetExternalHoldDismissal();
                }
            }
        }
    }
}
