/*
 * Displays, instrument family: one channel strip per display (label, applied
 * vs target, fader with scale ticks and the automatic range, override state,
 * limit faders) and an "All" master strip once there is more than one.
 * Emissive flavours put the strips behind one pane of display glass, the way
 * a hi-fi deck shows every channel in a single VFD window.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "Curve.js" as Curve

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0

    spacing: style.rowGap

    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.domain, "Instrument legend over the brightness faders", "Displays")
        trailing: root.monitors.length > 1
            ? i18ndc(root.domain, "Number of display channels", "%1 ch", root.monitors.length) : ""
    }

    DisplayWindow {
        Layout.fillWidth: true
        style: root.style
        spacing: root.style.snap(root.style.rowGap * 0.75)

        ChannelStrip {
            visible: root.showAllDisplays
            style: root.style
            channel: i18ndc(root.domain, "Channel label of the master fader for every display", "All")
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
            applied: -1
            // The way back from a global override is the Automation module's.
            hasOverride: false
            stateText: ""
            onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
        }

        Rectangle {
            Layout.fillWidth: true
            visible: root.showAllDisplays
            implicitHeight: root.style.hairline
            color: root.style.emissive ? root.style.glassEdge : root.style.ruleFaint
        }

        Repeater {
            model: root.monitors

            delegate: ChannelStrip {
                id: strip

                required property var modelData
                required property int index

                readonly property bool unavailable: modelData.unreachable
                    || modelData.topology === "temporarily_unavailable"

                style: root.style
                channel: String(index + 1).padStart(2, "0")
                name: modelData.logicalId
                percent: modelData.percent
                applied: modelData.appliedPercent
                target: Curve.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS)
                minPct: modelData.minPct !== undefined ? modelData.minPct : 15
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
                hasLimits: true
                hasOverride: modelData.hasOverride === true
                overrideUntil: hasOverride ? root.client.overrideUntilEpochS : 0
                controllable: modelData.enabled && !unavailable
                stateText: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "Off");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "No link");
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "Tag: this brightness was set by hand", "Man");
                    }
                    return "";
                }
                stateTone: !modelData.enabled ? root.style.inkMuted
                    : (unavailable ? root.style.danger : root.style.warning)

                onRequested: percent => root.client.setMonitorOverride(modelData.logicalId, percent,
                                                                       root.settings.overrideMinutes)
                onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId, minVal, maxVal)
                onClearOverrideRequested: () => root.client.clearMonitorOverride(modelData.logicalId)
            }
        }
    }
}
