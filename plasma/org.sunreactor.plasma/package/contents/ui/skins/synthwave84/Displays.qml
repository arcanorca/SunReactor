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
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../instrument/Curve.js" as Curve

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    /*! The automation card closes this column when the theme lists the automation module. */
    readonly property bool hostsModes: root.theme.modules.indexOf("automation") >= 0
    readonly property bool available: client.isConnected && (monitors.length > 0 || hostsModes)

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

        ChannelHead {
            channel: i18ndc(root.domain, "Channel label of the master fader for every display", "All")
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
        }
        NeonSlider {
            Layout.fillWidth: true
            style: root.style
            neon: root.style.cyan
            known: root.client.globalPercent >= 0
            enabled: known
            value: root.client.globalPercent >= 0 ? root.client.globalPercent : 0
            Accessible.name: i18nd(root.domain, "All displays")
            onPressedChanged: {
                if (pressed) {
                    return;
                }
                root.client.setGlobalOverride(Math.round(value), root.settings.overrideMinutes);
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

            readonly property bool unavailable: modelData.unreachable === true
                || modelData.topology === "temporarily_unavailable"
            readonly property bool controllable: modelData.enabled === true && !unavailable
            readonly property bool hasValue: modelData.percent !== undefined && modelData.percent >= 0
            readonly property real target: Curve.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS)
            readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : 15
            readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
            readonly property string stateText: {
                if (!modelData.enabled) {
                    return i18ndc(root.domain, "Tag: this display is excluded from automation", "Off");
                }
                if (unavailable) {
                    return i18ndc(root.domain, "Tag: this display is not answering", "No link");
                }
                if (modelData.hasOverride === true) {
                    return i18ndc(root.domain, "Tag: this brightness was set by hand", "Manual");
                }
                return "";
            }
            readonly property color stateTone: !modelData.enabled ? root.style.muted
                : (unavailable ? root.style.red : root.style.yellow)
            property bool limitsOpen: false

            Layout.fillWidth: true
            style: root.style
            neon: unavailable ? root.style.red : root.style.pink

            ChannelHead {
                channel: String(card.index + 1).padStart(2, "0")
                name: card.modelData.logicalId
                percent: card.hasValue ? card.modelData.percent : -1
                stateText: card.stateText
                stateTone: card.stateTone
                rangeText: i18ndc(root.domain, "Range limits", "%1–%2%", card.minPct, card.maxPct)
                rangeOpen: card.limitsOpen
                onRangeClicked: card.limitsOpen = !card.limitsOpen
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
                    limitMin: card.minPct
                    limitMax: card.maxPct
                    known: card.hasValue
                    enabled: card.controllable && card.hasValue
                    value: card.hasValue ? card.modelData.percent : 0
                    Accessible.name: card.modelData.logicalId
                    Accessible.description: card.hasValue
                        ? i18ndc(root.domain, "Brightness percentage", "%1%", card.modelData.percent)
                        : i18ndc(root.domain, "No value is known yet", "—")
                    // While dragging, status updates are ignored: the daemon reports
                    // what it has written, which lags the thumb.
                    onPressedChanged: {
                        if (pressed) {
                            return;
                        }
                        root.client.setMonitorOverride(card.modelData.logicalId, Math.round(value),
                                                       root.settings.overrideMinutes);
                        value = Qt.binding(() => card.hasValue ? card.modelData.percent : 0);
                    }
                }
                Pill {
                    visible: card.modelData.hasOverride === true
                    Layout.alignment: Qt.AlignVCenter
                    style: root.style
                    text: i18ndc(root.domain, "Button: drop the brightness set by hand", "Auto")
                    tone: root.style.pink
                    onClicked: root.client.clearMonitorOverride(card.modelData.logicalId)
                }
            }

            // The limits: a night minimum and a day maximum, two more tubes.
            ColumnLayout {
                Layout.fillWidth: true
                visible: card.limitsOpen
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
                        from: 0
                        to: Math.min(100, Math.round(maxSlider.value))
                        value: card.minPct
                        Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                        onPressedChanged: {
                            if (!pressed) {
                                root.client.setMonitorLimits(card.modelData.logicalId, Math.round(minSlider.value),
                                                             Math.round(maxSlider.value));
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
                        from: Math.round(minSlider.value)
                        to: 100
                        value: card.maxPct
                        Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                        onPressedChanged: {
                            if (!pressed) {
                                root.client.setMonitorLimits(card.modelData.logicalId, Math.round(minSlider.value),
                                                             Math.round(maxSlider.value));
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
