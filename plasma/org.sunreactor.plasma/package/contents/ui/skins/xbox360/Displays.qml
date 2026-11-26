/*
 * Xbox 360 Displays: metallic blade cards per display with grooved sliders,
 * chrome-green orb thumbs, range pills, and persistent min/max limit drawers.
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
    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: Monitors.hasDisplayData(client)
    property var expandedLimits: ({})

    spacing: root.style.rowGap

    XboxCard {
        id: masterCard

        visible: root.showAllDisplays
        style: root.style
        Layout.fillWidth: true
        implicitHeight: masterBody.implicitHeight + 2 * masterCard.pad
        lit: masterFader.activeFocus || masterFader.dragging

        ColumnLayout {
            id: masterBody

            anchors.fill: parent
            spacing: root.style.gap * 0.6

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                Rectangle {
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: root.style.snap(root.style.unit * 1.6)
                    implicitHeight: root.style.snap(root.style.unit * 1.15)
                    radius: height / 2
                    color: root.style.allBadgeFill
                    border.width: 1
                    border.color: root.style.allBadgeEdge

                    Text {
                        anchors.centerIn: parent
                        text: i18ndc(root.domain, "Channel badge for master brightness row", "ALL")
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: "#ffffff"
                    }
                }
                Text {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    text: root.settings.allDisplaysLabel
                    textFormat: Text.PlainText
                    font: root.style.bodyBoldFont
                    color: root.style.ink
                    elide: Text.ElideRight
                }
                XboxPill {
                    Layout.alignment: Qt.AlignVCenter
                    visible: root.client.isOverrideActive
                    enabled: root.client.isConnected
                    style: root.style
                    label: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                    Accessible.role: Accessible.Button
                    Accessible.name: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                    onClicked: root.client.clearAllOverrides()
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: root.client.globalPercent >= 0 ? String(Math.round(root.client.globalPercent)) : "--"
                    textFormat: Text.PlainText
                    font: root.style.valueFont
                    color: masterFader.dragging ? root.style.greenBright : root.style.ink
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: "%"
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.muted
                }
            }

            XboxSlider {
                id: masterFader

                Layout.fillWidth: true
                style: root.style
                enabled: root.client.isConnected && root.client.globalPercent >= 0
                known: root.client.globalPercent >= 0
                from: 0
                to: 100
                value: root.client.globalPercent >= 0 ? root.client.globalPercent : 0
                Accessible.name: root.settings.allDisplaysLabel
                onRequested: value => root.client.setGlobalOverride(value, root.settings.overrideMinutes)
            }
        }
    }

    Repeater {
        model: root.monitors

        delegate: XboxCard {
            id: strip

            required property var modelData
            required property int index

            readonly property string logicalId: String(strip.modelData.logicalId)
            readonly property bool unavailable: Monitors.unavailable(strip.modelData)
            readonly property bool controllable: root.client.isConnected && strip.modelData.enabled === true && !unavailable
            readonly property bool hasValue: strip.modelData.percent !== undefined && strip.modelData.percent >= 0
            readonly property int minPct: strip.modelData.minPct !== undefined ? strip.modelData.minPct : -1
            readonly property int maxPct: strip.modelData.maxPct !== undefined ? strip.modelData.maxPct : -1
            readonly property bool hasLimits: Monitors.hasValidLimits(strip.modelData)
            readonly property real target: DayCurve.targetAt(root.client.dayCurve, logicalId, root.client.nowEpochS)
            readonly property int applied: strip.modelData.appliedPercent !== undefined ? strip.modelData.appliedPercent : -1
            readonly property bool hasHold: strip.modelData.hasOverride !== true && Monitors.hasExternalHold(strip.modelData)
            readonly property string notice: Monitors.monitorNotice(
                strip.modelData, root.settings.contestedNotice, root.settings.deferredNotice)
            readonly property string stateText: {
                if (strip.modelData.enabled !== true) {
                    return i18ndc(root.domain, "Tag: this display is excluded from automation", "Off");
                }
                if (unavailable) {
                    return i18ndc(root.domain, "Tag: this display is not answering", "No link");
                }
                if (strip.modelData.hasOverride === true) {
                    return i18ndc(root.domain, "Tag: this brightness was set by hand", "By hand");
                }
                if (hasHold) {
                    return i18ndc(root.domain, "External brightness hold badge", "Ext · %1",
                                  Monitors.externalHoldAge(Monitors.externalHoldSeconds(strip.modelData)));
                }
                return "";
            }
            readonly property bool limitsOpen: Monitors.isExpanded(root.expandedLimits, logicalId)

            style: root.style
            Layout.fillWidth: true
            implicitHeight: body.implicitHeight + 2 * strip.pad
            lit: fader.activeFocus || fader.dragging

            ColumnLayout {
                id: body

                anchors.fill: parent
                spacing: root.style.gap * 0.55

                RowLayout {
                    Layout.fillWidth: true
                    spacing: root.style.gap

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: root.style.snap(root.style.unit * 1.25)
                        implicitHeight: implicitWidth
                        radius: width / 2
                        color: strip.controllable ? root.style.indexBg : root.style.indexBgOff
                        border.width: 1
                        border.color: strip.controllable ? root.style.green : root.style.steel

                        Text {
                            anchors.centerIn: parent
                            text: String(strip.index + 1).padStart(2, "0")
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: strip.controllable ? root.style.greenBright : root.style.muted
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        text: strip.logicalId
                        textFormat: Text.PlainText
                        font: root.style.bodyBoldFont
                        color: strip.controllable ? root.style.ink : root.style.muted
                        elide: Text.ElideMiddle
                    }
                    Text {
                        Layout.alignment: Qt.AlignVCenter
                        visible: text.length > 0
                        text: strip.stateText
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: strip.unavailable ? root.style.danger : root.style.greenBright
                    }
                    XboxPill {
                        Layout.alignment: Qt.AlignVCenter
                        visible: strip.modelData.hasOverride === true || strip.hasHold
                        enabled: root.client.isConnected
                        style: root.style
                        label: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                        Accessible.role: Accessible.Button
                        Accessible.name: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                        onClicked: strip.hasHold
                            ? root.client.clearExternalHold(strip.logicalId)
                            : root.client.clearMonitorOverride(strip.logicalId)
                    }
                    XboxPill {
                        Layout.alignment: Qt.AlignVCenter
                        visible: strip.hasLimits
                        style: root.style
                        hint: i18ndc(root.domain, "Automatic range of a display, as min–max percent", "%1–%2",
                                     strip.minPct, strip.maxPct)
                        arrow: true
                        current: strip.limitsOpen
                        Accessible.role: Accessible.Button
                        Accessible.name: i18ndc(root.domain, "Strip control: automatic range, opens the limit sliders", "Range %1–%2", strip.minPct, strip.maxPct)
                        onClicked: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, strip.logicalId)
                        onArrowClicked: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, strip.logicalId)
                    }
                    Text {
                        Layout.alignment: Qt.AlignBaseline
                        text: strip.hasValue ? String(Math.round(strip.modelData.percent)) : "--"
                        textFormat: Text.PlainText
                        font: root.style.valueFont
                        color: fader.dragging ? root.style.greenBright : root.style.ink
                    }
                    Text {
                        Layout.alignment: Qt.AlignBaseline
                        text: "%"
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: root.style.muted
                    }
                }

                XboxSlider {
                    id: fader

                    Layout.fillWidth: true
                    style: root.style
                    enabled: strip.controllable && strip.hasValue
                    known: strip.hasValue
                    from: 0
                    to: 100
                    value: strip.hasValue ? strip.modelData.percent : 0
                    target: strip.target
                    Accessible.name: strip.logicalId
                    onRequested: value => root.client.setMonitorOverride(strip.logicalId, value, root.settings.overrideMinutes)
                }

                Text {
                    Layout.fillWidth: true
                    text: {
                        const parts = [];
                        if (strip.notice.length > 0) {
                            parts.push(strip.notice);
                        }
                        if (strip.target >= 0) {
                            parts.push(i18ndc(root.domain, "Strip legend: where the automatic curve is now",
                                              "Target %1", Math.round(strip.target)));
                        }
                        if (strip.applied >= 0 && strip.applied !== Math.round(strip.modelData.percent)) {
                            parts.push(i18ndc(root.domain, "Strip legend: what the daemon last wrote",
                                              "Applied %1", strip.applied));
                        }
                        if (!strip.hasValue) {
                            parts.push(i18ndc(root.domain, "Strip legend: the display has not reported a value",
                                              "No reading"));
                        }
                        return parts.join(" · ");
                    }
                    textFormat: Text.PlainText
                    font: root.style.captionFont
                    color: strip.notice.length > 0 ? root.style.greenBright : root.style.muted
                    elide: Text.ElideRight
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: strip.hasLimits && strip.limitsOpen
                    spacing: root.style.gap * 0.5

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: root.style.gap

                        Text {
                            Layout.preferredWidth: root.style.snap(root.style.unit * 4.2)
                            text: i18ndc(root.domain, "Day maximum boundary", "Day max")
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: root.style.muted
                        }
                        XboxSlider {
                            id: maxSlider

                            Layout.fillWidth: true
                            style: root.style
                            enabled: strip.controllable && strip.hasLimits
                            from: Math.min(100, Math.round(minSlider.shown))
                            to: 100
                            value: strip.maxPct
                            Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                            onRequested: value => root.client.setMonitorLimits(strip.logicalId, strip.minPct,
                                                                               Math.max(strip.minPct, value))
                        }
                        Text {
                            Layout.preferredWidth: root.style.snap(root.style.unit * 1.6)
                            horizontalAlignment: Text.AlignRight
                            text: String(Math.round(maxSlider.shown))
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: root.style.ink
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: root.style.gap

                        Text {
                            Layout.preferredWidth: root.style.snap(root.style.unit * 4.2)
                            text: i18ndc(root.domain, "Night minimum boundary", "Night min")
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: root.style.muted
                        }
                        XboxSlider {
                            id: minSlider

                            Layout.fillWidth: true
                            style: root.style
                            enabled: strip.controllable && strip.hasLimits
                            from: 0
                            to: Math.max(0, Math.round(maxSlider.shown))
                            value: strip.minPct
                            Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                            onRequested: value => root.client.setMonitorLimits(strip.logicalId,
                                                                               Math.min(value, strip.maxPct),
                                                                               strip.maxPct)
                        }
                        Text {
                            Layout.preferredWidth: root.style.snap(root.style.unit * 1.6)
                            horizontalAlignment: Text.AlignRight
                            text: String(Math.round(minSlider.shown))
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: root.style.ink
                        }
                    }
                }
            }
        }
    }
}
