/*
 * Displays, synthwave84: one neon card per display, tighter than a page of cards.
 * Its first line is the channel (cyan caps), the name, the tag of its state, the
 * range chip and the value in the readout face; under it a neon tube slider with
 * the curve's target as a tick and the automatic range as a cyan band, and an
 * "Auto" chip when a setting was made by hand. The range chip opens two more tubes
 * for the limits. An "All" card drives every display at once once there is more
 * than one. When the theme lists the automation module, its card closes the column
 * (ModeRows), so the page needs no heading of its own for it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../../components"
import "../../engine"
import "../../modules" as Mod
import "../../modules/DayCurve.js" as DayCurve
import "../../modules/Monitors.js" as Monitors

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    /*! The automation card closes this column when the theme lists the automation module. */
    readonly property bool hostsModes: root.theme.modules.indexOf("automation") >= 0
    readonly property bool available: Monitors.hasStatus(client) && (monitors.length > 0 || hostsModes)
    property var expandedLimits: ({})

    spacing: style.rowGap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(domain, "Accessible name of the display faders", "Displays")

    /*! The first line of a channel: its label and name, a state tag, the range chip, the value. */
    component ChannelHead: RowLayout {
        id: head

        required property string channel
        required property string name
        required property int percent
        property string stateText: ""
        property color stateTone: root.style.pink
        property string notice: ""
        property string rangeText: ""
        property bool rangeOpen: false

        signal rangeClicked()

        Layout.fillWidth: true
        spacing: root.style.gap2

        Text {
            Layout.alignment: Qt.AlignVCenter
            text: head.channel
            textFormat: Text.PlainText
            font: root.style.labelFont
            color: root.style.cyan
        }
        Text {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            Layout.alignment: Qt.AlignVCenter
            text: head.name
            textFormat: Text.PlainText
            font: root.style.bodyStrongFont
            color: root.style.ink
            elide: Text.ElideRight
        }
        Pill {
            visible: head.stateText.length > 0
            Layout.alignment: Qt.AlignVCenter
            style: root.style
            text: head.stateText
            tone: head.stateTone

            HoverHandler {
                id: stateHover
            }

            QQC2.ToolTip.visible: stateHover.hovered && head.notice.length > 0
            QQC2.ToolTip.text: head.notice
            QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
        }
        Pill {
            visible: head.rangeText.length > 0
            Layout.alignment: Qt.AlignVCenter
            style: root.style
            text: head.rangeText
            tone: root.style.cyan
            arrow: true
            filled: head.rangeOpen
            ringed: head.rangeOpen
            onClicked: head.rangeClicked()
        }
        Text {
            Layout.alignment: Qt.AlignVCenter
            text: head.percent >= 0 ? head.percent + "%" : "—"
            textFormat: Text.PlainText
            font: root.style.readoutFont
            color: root.style.ink
        }
    }

    // --- all displays ---------------------------------------------------------------
    NeonCard {
        visible: root.showAllDisplays
        Layout.fillWidth: true
        style: root.style
        neon: root.style.cyan

        CommitThrottle {
            id: masterThrottle

            reportedValue: root.client.globalPercent
            onCommitted: val => {
                root.client.setGlobalOverride(val, root.settings.overrideMinutes);
                if (!masterSlider.pressed) {
                    masterSlider.value = Qt.binding(() => root.client.globalPercent >= 0 ? root.client.globalPercent : 0);
                }
            }
        }

        ChannelHead {
            channel: i18ndc(root.domain, "Channel label of the master fader for every display", "All")
            name: root.settings.allDisplaysLabel
            percent: root.client.globalPercent
        }
        NeonSlider {
            id: masterSlider

            Layout.fillWidth: true
            style: root.style
            neon: root.style.cyan
            known: root.client.globalPercent >= 0
            enabled: known && root.client.isConnected
            value: root.client.globalPercent >= 0 ? root.client.globalPercent : 0
            Accessible.name: root.settings.allDisplaysLabel
            onMoved: pressed ? masterThrottle.schedule(Math.round(value)) : masterThrottle.nudge(Math.round(value))
            onPressedChanged: {
                if (pressed) {
                    return;
                }
                masterThrottle.release(Math.round(value));
                value = Qt.binding(() => root.client.globalPercent >= 0 ? root.client.globalPercent : 0);
            }
        }
    }

    // --- one card per display -------------------------------------------------------
    Repeater {
        model: root.monitors
        delegate: NeonCard {
            id: card

            required property var modelData
            required property int index

            readonly property bool unavailable: Monitors.unavailable(modelData)
            readonly property bool controllable: root.client.isConnected && modelData.enabled === true && !unavailable
            readonly property bool hasValue: modelData.percent !== undefined && modelData.percent >= 0
            readonly property real target: DayCurve.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS)
            readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : -1
            readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
            readonly property bool hasLimits: Monitors.hasValidLimits(modelData)
            readonly property bool hasHold: modelData.hasOverride !== true && Monitors.hasExternalHold(modelData)
            readonly property string fullNotice: Monitors.monitorNotice(
                modelData, root.settings.contestedNotice, root.settings.deferredNotice)
            readonly property string tagNotice: Monitors.monitorTag(
                modelData, root.settings.pausedTag, root.settings.queuedTag)
            readonly property string stateText: {
                if (!modelData.enabled) {
                    return i18ndc(root.domain, "Tag: this display is excluded from automation", "Off");
                }
                if (unavailable) {
                    return i18ndc(root.domain, "Tag: this display is not answering", "No link");
                }
                if (tagNotice.length > 0) {
                    return tagNotice;
                }
                if (modelData.hasOverride === true) {
                    return i18ndc(root.domain, "Tag: this brightness was set by hand", "Manual");
                }
                if (hasHold) {
                    return i18ndc(root.domain, "External brightness hold badge", "Ext · %1",
                                  Monitors.externalHoldAge(Monitors.externalHoldSeconds(modelData)));
                }
                return "";
            }
            readonly property color stateTone: !modelData.enabled ? root.style.muted
                : (unavailable ? root.style.red : root.style.yellow)
            readonly property bool limitsOpen: Monitors.isExpanded(root.expandedLimits, card.modelData.logicalId)

            CommitThrottle {
                id: cardThrottle

                reportedValue: card.hasValue ? card.modelData.percent : -1
                onCommitted: val => {
                    root.client.setMonitorOverride(card.modelData.logicalId, val, root.settings.overrideMinutes);
                    if (!slider.pressed) {
                        slider.value = Qt.binding(() => card.hasValue ? card.modelData.percent : 0);
                    }
                }
            }
            CommitThrottle {
                id: minThrottle

                reportedValue: card.minPct
                onCommitted: val => {
                    root.client.setMonitorLimits(card.modelData.logicalId, val, Math.round(maxSlider.value));
                    if (!minSlider.pressed) {
                        minSlider.value = Qt.binding(() => card.minPct >= 0 ? card.minPct : 0);
                    }
                }
            }
            CommitThrottle {
                id: maxThrottle

                reportedValue: card.maxPct
                onCommitted: val => {
                    root.client.setMonitorLimits(card.modelData.logicalId, Math.round(minSlider.value), val);
                    if (!maxSlider.pressed) {
                        maxSlider.value = Qt.binding(() => card.maxPct >= 0 ? card.maxPct : 100);
                    }
                }
            }

            Layout.fillWidth: true
            style: root.style
            neon: unavailable ? root.style.red : root.style.pink

            ChannelHead {
                channel: String(card.index + 1).padStart(2, "0")
                name: card.modelData.logicalId
                percent: card.hasValue ? card.modelData.percent : -1
                stateText: card.stateText
                stateTone: card.stateTone
                notice: card.fullNotice
                rangeText: card.hasLimits ? i18ndc(root.domain, "Range limits", "%1–%2%", card.minPct, card.maxPct) : ""
                rangeOpen: card.limitsOpen
                onRangeClicked: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, card.modelData.logicalId)
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap2

                NeonSlider {
                    id: slider

                    Layout.fillWidth: true
                    style: root.style
                    neon: card.neon
                    target: card.target
                    limitMin: card.hasLimits ? card.minPct : -1
                    limitMax: card.hasLimits ? card.maxPct : -1
                    known: card.hasValue
                    enabled: card.controllable && card.hasValue
                    value: card.hasValue ? card.modelData.percent : 0
                    Accessible.name: card.modelData.logicalId
                    Accessible.description: card.hasValue
                        ? i18ndc(root.domain, "Brightness percentage", "%1%", card.modelData.percent)
                        : i18ndc(root.domain, "No value is known yet", "—")
                    onMoved: pressed ? cardThrottle.schedule(Math.round(value)) : cardThrottle.nudge(Math.round(value))
                    // While dragging, status updates are ignored: the daemon reports
                    // what it has written, which lags the thumb.
                    onPressedChanged: {
                        if (pressed) {
                            return;
                        }
                        cardThrottle.release(Math.round(value));
                        value = Qt.binding(() => card.hasValue ? card.modelData.percent : 0);
                    }
                }
                Pill {
                    visible: card.modelData.hasOverride === true || card.hasHold
                    Layout.alignment: Qt.AlignVCenter
                    style: root.style
                    text: i18ndc(root.domain, "Button: drop the brightness set by hand", "Auto")
                    tone: root.style.pink
                    onClicked: card.hasHold
                        ? root.client.clearExternalHold(card.modelData.logicalId)
                        : root.client.clearMonitorOverride(card.modelData.logicalId)
                }
            }

            // The limits: a night minimum and a day maximum, two more tubes.
            ColumnLayout {
                Layout.fillWidth: true
                visible: card.hasLimits && card.limitsOpen
                spacing: root.style.gap

                RowLayout {
                    Layout.fillWidth: true
                    spacing: root.style.gap2

                    Text {
                        Layout.preferredWidth: root.style.unit * 3
                        text: i18ndc(root.domain, "Night minimum boundary", "Night min")
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.cyan
                    }
                    NeonSlider {
                        id: minSlider

                        Layout.fillWidth: true
                        style: root.style
                        neon: root.style.cyan
                        enabled: card.controllable && card.hasLimits
                        from: 0
                        to: Math.min(100, Math.round(maxSlider.value))
                        value: card.minPct >= 0 ? card.minPct : 0
                        Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                        onMoved: pressed ? minThrottle.schedule(Math.round(value)) : minThrottle.nudge(Math.round(value))
                        onPressedChanged: {
                            if (!pressed) {
                                minThrottle.release(Math.round(value));
                                value = Qt.binding(() => card.minPct >= 0 ? card.minPct : 0);
                            }
                        }
                    }
                    Text {
                        Layout.preferredWidth: root.style.unit * 1.6
                        horizontalAlignment: Text.AlignRight
                        text: Math.round(minSlider.value) + "%"
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.ink
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: root.style.gap2

                    Text {
                        Layout.preferredWidth: root.style.unit * 3
                        text: i18ndc(root.domain, "Day maximum boundary", "Day max")
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.cyan
                    }
                    NeonSlider {
                        id: maxSlider

                        Layout.fillWidth: true
                        style: root.style
                        neon: root.style.cyan
                        enabled: card.controllable && card.hasLimits
                        from: Math.round(minSlider.value)
                        to: 100
                        value: card.maxPct >= 0 ? card.maxPct : 100
                        Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                        onMoved: pressed ? maxThrottle.schedule(Math.round(value)) : maxThrottle.nudge(Math.round(value))
                        onPressedChanged: {
                            if (!pressed) {
                                maxThrottle.release(Math.round(value));
                                value = Qt.binding(() => card.maxPct >= 0 ? card.maxPct : 100);
                            }
                        }
                    }
                    Text {
                        Layout.preferredWidth: root.style.unit * 1.6
                        horizontalAlignment: Text.AlignRight
                        text: Math.round(maxSlider.value) + "%"
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.ink
                    }
                }
            }
        }
    }

    // --- the automation controls, in a card of their own -----------------------------
    NeonCard {
        visible: root.hostsModes
        Layout.fillWidth: true
        style: root.style
        neon: root.style.pink

        ModeRows {
            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
