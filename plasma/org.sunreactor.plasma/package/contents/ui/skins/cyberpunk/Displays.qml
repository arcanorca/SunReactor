/*
 * Displays, cyberpunk: one titled panel for the displays. Its rows share the panel
 * and are ruled apart: an "all displays" master row once there is more than one,
 * then one row per display. When the theme lists the automation module, its
 * controls close the same panel (ModeRows), so the brightness page has one frame
 * for the displays and the mode. The slider writes overrides only; keyboard, click
 * and drag work as in skins/instrument/Displays.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../modules/DayCurve.js" as DayCurve
import "../../modules/Monitors.js" as Monitors

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    readonly property bool showAll: monitors.length > 1
    /*! The automation controls close this panel when the theme lists them; Automation.qml then stays away. */
    readonly property bool hostsModes: root.theme.modules.indexOf("automation") >= 0
    readonly property bool available: Monitors.hasStatus(client) && (monitors.length > 0 || hostsModes)
    property var expandedLimits: ({})

    spacing: style.rowGap

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        title: i18ndc(root.domain, "Panel title: the display channels", "Displays")
        tag: root.monitors.length > 0 ? i18ndc(root.domain, "Number of display channels", "%1 CH", root.monitors.length) : ""
        contentSpacing: 0

        CyberChannel {
            visible: root.showAll
            Layout.fillWidth: true
            style: root.style
            domain: root.domain
            channel: ""
            name: root.settings.allDisplaysLabel
            percent: root.client.globalPercent
            controllable: root.client.isConnected
            onRequested: v => root.client.setGlobalOverride(v, root.settings.overrideMinutes)
        }

        Repeater {
            model: root.monitors

            delegate: CyberChannel {
                id: strip

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
                channel: String(index + 1).padStart(2, "0")
                ruled: root.showAll || index > 0
                name: modelData.logicalId
                percent: modelData.percent
                applied: modelData.appliedPercent
                target: DayCurve.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS)
                minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                hasLimits: Monitors.hasValidLimits(modelData)
                limitsOpen: Monitors.isExpanded(root.expandedLimits, modelData.logicalId)
                hasOverride: modelData.hasOverride === true
                canClear: modelData.hasOverride === true || hasHold
                controllable: root.client.isConnected && modelData.enabled && !unavailable
                notice: fullNotice
                stateText: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "OFF");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "NO LINK");
                    }
                    if (tagNotice.length > 0) {
                        return tagNotice;
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "Tag: this brightness was set by hand", "MAN");
                    }
                    if (hasHold) {
                        return i18ndc(root.domain, "External brightness hold badge, compact", "EXT %1",
                                      Monitors.externalHoldAge(Monitors.externalHoldSeconds(modelData)));
                    }
                    return "";
                }
                stateTone: !modelData.enabled ? root.style.faint
                    : ((hasHold || tagNotice.length > 0) ? root.style.yellow : root.style.red)

                onRequested: v => root.client.setMonitorOverride(modelData.logicalId, v, root.settings.overrideMinutes)
                onLimitsToggled: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, modelData.logicalId)
                onLimitsRequested: (lo, hi) => root.client.setMonitorLimits(modelData.logicalId, lo, hi)
                onClearRequested: hasHold
                    ? root.client.clearExternalHold(modelData.logicalId)
                    : root.client.clearMonitorOverride(modelData.logicalId)
            }
        }

        ModeRows {
            visible: root.hostsModes
            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
