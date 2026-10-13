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
import "../instrument/Curve.js" as Curve

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAll: monitors.length > 1
    /*! The automation controls close this panel when the theme lists them; Automation.qml then stays away. */
    readonly property bool hostsModes: root.theme.modules.indexOf("automation") >= 0
    readonly property bool available: client.isConnected && (monitors.length > 0 || hostsModes)

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
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
            onRequested: v => root.client.setGlobalOverride(v, root.settings.overrideMinutes)
        }

        Repeater {
            model: root.monitors

            delegate: CyberChannel {
                id: strip

                required property var modelData
                required property int index

                readonly property bool unavailable: modelData.unreachable
                    || modelData.topology === "temporarily_unavailable"

                Layout.fillWidth: true
                style: root.style
                domain: root.domain
                channel: String(index + 1).padStart(2, "0")
                ruled: root.showAll || index > 0
                name: modelData.logicalId
                percent: modelData.percent
                applied: modelData.appliedPercent
                target: Curve.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS)
                minPct: modelData.minPct !== undefined ? modelData.minPct : 15
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
                hasLimits: true
                hasOverride: modelData.hasOverride === true
                controllable: modelData.enabled && !unavailable
                stateText: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "OFF");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "NO LINK");
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "Tag: this brightness was set by hand", "MAN");
                    }
                    return "";
                }
                stateTone: modelData.enabled ? root.style.red : root.style.faint

                onRequested: v => root.client.setMonitorOverride(modelData.logicalId, v, root.settings.overrideMinutes)
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
