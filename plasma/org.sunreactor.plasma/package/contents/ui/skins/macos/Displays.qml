/*
 * Displays, macos: Control Center's brightness sliders in one inset list. A row
 * per display, and an All row once there is more than one. The module reports
 * what the user asked for; the daemon decides what that means.
 *
 *   ╭──────────────────────────────────────────────╮
 *   │ All displays                                 │
 *   │ ██████████████████████░░░░░░░░░░░░░░░░  39%  │
 *   │ ──────────────────────────────────────────── │
 *   │ mi-monitor                    [Auto] [Range] │
 *   │ ████████████████░░░░░░░░░░░░░░░░░░░░░░  41%  │
 *   ╰──────────────────────────────────────────────╯
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../modules/Monitors.js" as Monitors

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: Monitors.hasDisplayData(client)
    property var expandedLimits: ({})

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Accessible name of the brightness list", "Displays")

    Surface {
        Layout.fillWidth: true
        style: root.style
        radius: root.style.radiusList
        padding: 0
        spacing: 0

        DisplayRow {
            visible: root.showAllDisplays
            Layout.fillWidth: true
            style: root.style
            domain: root.domain
            name: root.settings.allDisplaysLabel
            percent: root.client.globalPercent
            controllable: root.client.isConnected
            hasLimits: false
            hasOverride: false
            divided: true

            onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
        }

        Repeater {
            model: root.monitors

            delegate: DisplayRow {
                id: row

                required property var modelData
                required property int index

                readonly property bool unavailable: Monitors.unavailable(modelData)
                readonly property bool hasHold: modelData.hasOverride !== true && Monitors.hasExternalHold(modelData)
                readonly property string fullNotice: Monitors.monitorNotice(
                    modelData, root.settings.contestedNotice, root.settings.deferredNotice)
                readonly property string tagNotice: Monitors.monitorTag(
                    modelData, root.settings.pausedTag, root.settings.queuedTag)

                Layout.fillWidth: true
                style: root.style
                domain: root.domain
                name: modelData.logicalId
                percent: modelData.percent
                minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                hasLimits: Monitors.hasValidLimits(modelData)
                limitsExpanded: Monitors.isExpanded(root.expandedLimits, modelData.logicalId)
                hasOverride: modelData.hasOverride === true || hasHold
                controllable: root.client.isConnected && modelData.enabled && !unavailable
                divided: index < root.monitors.length - 1
                notice: fullNotice
                stateText: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "Off");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "No link");
                    }
                    if (tagNotice.length > 0) {
                        return tagNotice;
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "Tag: this brightness was set by hand", "Manual");
                    }
                    if (hasHold) {
                        return i18ndc(root.domain, "External brightness hold badge", "Ext · %1",
                                      Monitors.externalHoldAge(Monitors.externalHoldSeconds(modelData)));
                    }
                    return "";
                }
                stateTone: unavailable && modelData.enabled ? root.style.danger
                    : (tagNotice.length > 0 ? root.style.accent : root.style.inkSecondary)

                onRequested: percent => root.client.setMonitorOverride(modelData.logicalId, percent,
                                                                       root.settings.overrideMinutes)
                onLimitsToggled: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, modelData.logicalId)
                onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId, minVal, maxVal)
                onClearOverrideRequested: () => hasHold
                    ? root.client.clearExternalHold(modelData.logicalId)
                    : root.client.clearMonitorOverride(modelData.logicalId)
            }
        }
    }
}
