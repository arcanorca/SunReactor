/*
 * Displays, Wii style: one channel tile per display. Its name and the percent
 * it shows, a round-ended slider (the thumb is the brightness, the small dot
 * under the track is where the automatic curve wants it), and a line of what it
 * is doing. The range pill opens the day maximum and night minimum sliders; an
 * Auto bubble returns a display to automatic. Knows nothing of the daemon's
 * policy: it reports what the user asked for.
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
    readonly property WiiStyle style: WiiStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool available: client.isConnected && monitors.length > 0

    // Each strip carries its shadow room below it (WiiTile), so the spacing
    // between strips is the rest of the row gap: the gap itself does not change.
    spacing: Math.max(0, style.rowGap - style.shadowRoom)

    Repeater {
        model: root.monitors

        delegate: WiiTile {
            id: strip

            required property var modelData
            required property int index

            readonly property string logicalId: String(strip.modelData.logicalId)
            readonly property bool unavailable: strip.modelData.unreachable === true
                || strip.modelData.topology === "temporarily_unavailable"
            readonly property bool controllable: strip.modelData.enabled === true && !unavailable
            readonly property bool hasValue: strip.modelData.percent !== undefined && strip.modelData.percent >= 0
            readonly property int minPct: strip.modelData.minPct !== undefined ? strip.modelData.minPct : 15
            readonly property int maxPct: strip.modelData.maxPct !== undefined ? strip.modelData.maxPct : 60
            readonly property real target: Curve.targetAt(root.client.dayCurve, logicalId, root.client.nowEpochS)
            readonly property int applied: strip.modelData.appliedPercent !== undefined ? strip.modelData.appliedPercent : -1
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
                return "";
            }
            readonly property color stateTone: strip.modelData.enabled !== true ? root.style.inkSoft
                : (unavailable ? root.style.dangerInk : root.style.accentInk)
            property bool limitsOpen: false

            style: root.style
            Layout.fillWidth: true
            implicitHeight: body.implicitHeight + 2 * strip.pad
            lit: fader.activeFocus || fader.dragging

            ColumnLayout {
                id: body

                anchors.fill: parent
                spacing: root.style.gap

                // Channel badge, name, state, the value.
                RowLayout {
                    Layout.fillWidth: true
                    spacing: root.style.gap

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: root.style.snap(root.style.unit * 1.25)
                        implicitHeight: implicitWidth
                        radius: width / 2
                        antialiasing: true
                        color: root.style.paper
                        border.width: root.style.line
                        border.color: root.style.outline

                        Text {
                            anchors.centerIn: parent
                            text: String(strip.index + 1).padStart(2, "0")
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: root.style.inkSoft
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        text: strip.logicalId
                        textFormat: Text.PlainText
                        font: root.style.bodyBoldFont
                        color: strip.controllable ? root.style.ink : root.style.inkSoft
                        elide: Text.ElideMiddle
                    }
                    Text {
                        Layout.alignment: Qt.AlignVCenter
                        visible: text.length > 0
                        text: strip.stateText
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: strip.stateTone
                    }
                    WiiPill {
                        Layout.alignment: Qt.AlignVCenter
                        visible: strip.modelData.hasOverride === true
                        style: root.style
                        label: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                        variant: "outline"
                        Accessible.role: Accessible.Button
                        Accessible.name: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                        onClicked: root.client.clearMonitorOverride(strip.logicalId)
                    }
                    WiiPill {
                        Layout.alignment: Qt.AlignVCenter
                        style: root.style
                        hint: i18ndc(root.domain, "Automatic range of a display, as min–max percent", "%1–%2",
                                     strip.minPct, strip.maxPct)
                        variant: "outline"
                        arrow: true
                        current: strip.limitsOpen
                        Accessible.role: Accessible.Button
                        Accessible.name: i18ndc(root.domain, "Strip control: automatic range, opens the limit sliders", "Range %1–%2", strip.minPct, strip.maxPct)
                        onClicked: strip.limitsOpen = !strip.limitsOpen
                    }
                    Text {
                        Layout.alignment: Qt.AlignBaseline
                        text: strip.hasValue ? String(Math.round(strip.modelData.percent)) : "--"
                        textFormat: Text.PlainText
                        font: root.style.valueFont
                        color: fader.dragging ? root.style.accentInk : root.style.ink
                    }
                    Text {
                        Layout.alignment: Qt.AlignBaseline
                        text: "%"
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: root.style.inkSoft
                    }
                }

                WiiSlider {
                    id: fader

                    Layout.fillWidth: true
                    Layout.topMargin: root.style.gap
                    style: root.style
                    enabled: strip.controllable && strip.hasValue
                    known: strip.hasValue
                    from: 0
                    to: 100
                    value: strip.hasValue ? strip.modelData.percent : 0
                    target: strip.target
                    Accessible.name: strip.logicalId
                    Accessible.description: strip.hasValue
                        ? i18ndc(root.domain, "Brightness percentage", "%1%", Math.round(strip.modelData.percent))
                        : i18ndc(root.domain, "No value is known yet", "—")

                    onRequested: value => root.client.setMonitorOverride(strip.logicalId, value,
                                                                         root.settings.overrideMinutes)
                }

                // What the display is doing: target and applied.
                Text {
                    Layout.fillWidth: true
                    text: {
                        const parts = [];
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
                    color: root.style.inkSoft
                    elide: Text.ElideRight
                }

                // The limits: the day maximum and the night minimum.
                ColumnLayout {
                    Layout.fillWidth: true
                    visible: strip.limitsOpen
                    spacing: root.style.gap

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: root.style.gap

                        Text {
                            Layout.preferredWidth: root.style.snap(root.style.unit * 4.2)
                            text: i18ndc(root.domain, "Day maximum boundary", "Day max")
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: root.style.inkSoft
                        }
                        WiiSlider {
                            id: maxSlider

                            Layout.fillWidth: true
                            style: root.style
                            enabled: strip.controllable
                            from: 0
                            to: 100
                            value: strip.maxPct
                            Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                            onRequested: value => root.client.setMonitorLimits(strip.logicalId, strip.minPct, value)
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
                            color: root.style.inkSoft
                        }
                        WiiSlider {
                            id: minSlider

                            Layout.fillWidth: true
                            style: root.style
                            enabled: strip.controllable
                            from: 0
                            to: 100
                            value: strip.minPct
                            Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                            onRequested: value => root.client.setMonitorLimits(strip.logicalId, value, strip.maxPct)
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
