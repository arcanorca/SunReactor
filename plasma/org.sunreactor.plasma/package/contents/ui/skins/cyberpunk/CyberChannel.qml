/*
 * One display as a chamfered strip: channel, name, state tag, the value in
 * brackets, the slanted-cell slider, and a line with the target, the applied
 * value and the automatic range. The master strip (all displays, no channel)
 * has no line.
 *
 *   [01] MI-MONITOR                       MAN  [39%]
 *   ▰▰▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱▱▱▱▱
 *   TARGET [41]        APPLIED [39]                 RANGE [15–60]
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

CyberFrame {
    id: root

    property string domain: ""
    property string channel: ""
    property string name: ""
    property int percent: -1
    property int applied: -1
    property real target: -1
    property int minPct: 15
    property int maxPct: 60
    property bool hasLimits: false
    property bool hasOverride: false
    property bool controllable: true
    property string stateText: ""
    property color stateTone: style.red

    readonly property bool master: channel.length === 0

    signal requested(int percent)

    hatched: false
    edge: style.theme.alpha(style.yellow, 0.6)

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        Text {
            visible: !root.master
            text: root.channel
            textFormat: Text.PlainText
            font: root.style.techFont
            color: root.style.cyan
        }
        Text {
            Layout.fillWidth: true
            text: root.name
            textFormat: Text.PlainText
            font: root.style.headFont
            color: root.style.ink
            elide: Text.ElideRight
        }
        Text {
            visible: root.stateText.length > 0
            text: root.stateText
            textFormat: Text.PlainText
            font: root.style.labelFont
            color: root.stateTone
        }
        Bracket {
            style: root.style
            value: root.percent >= 0 ? root.percent + "%" : "--"
            font: root.style.midFont
            color: root.style.ink
            bracketColor: root.style.muted
        }
    }

    CellBar {
        Layout.fillWidth: true
        style: root.style
        value: root.percent
        target: root.target
        minMark: root.hasLimits ? root.minPct : -1
        maxMark: root.hasLimits ? root.maxPct : -1
        interactive: root.controllable
        litColor: root.hasOverride ? root.style.red : root.style.yellow
        accessibleName: root.name
        onCommitted: v => root.requested(v)
    }

    RowLayout {
        Layout.fillWidth: true
        visible: !root.master
        spacing: root.style.gap2

        Row {
            spacing: root.style.gap
            Text {
                visible: root.target >= 0
                anchors.verticalCenter: parent.verticalCenter
                text: root.domain.length > 0
                    ? i18ndc(root.domain, "Label before the automatic target value", "TARGET") : "TARGET"
                textFormat: Text.PlainText
                font: root.style.labelFont
                color: root.style.muted
            }
            Bracket {
                visible: root.target >= 0
                anchors.verticalCenter: parent.verticalCenter
                style: root.style
                value: String(Math.round(root.target))
                font: root.style.techFont
                color: root.style.cyan
                bracketColor: root.style.muted
            }
        }
        Row {
            spacing: root.style.gap
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.domain.length > 0
                    ? i18ndc(root.domain, "Label before the brightness applied last", "APPLIED") : "APPLIED"
                textFormat: Text.PlainText
                font: root.style.labelFont
                color: root.style.muted
            }
            Bracket {
                anchors.verticalCenter: parent.verticalCenter
                style: root.style
                value: root.applied >= 0 ? String(root.applied) : "--"
                font: root.style.techFont
                color: root.style.ink
                bracketColor: root.style.muted
            }
        }
        Item {
            Layout.fillWidth: true
        }
        Row {
            visible: root.hasLimits
            spacing: root.style.gap
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.domain.length > 0
                    ? i18ndc(root.domain, "Label before the automatic minimum and maximum", "RANGE") : "RANGE"
                textFormat: Text.PlainText
                font: root.style.labelFont
                color: root.style.muted
            }
            Bracket {
                anchors.verticalCenter: parent.verticalCenter
                style: root.style
                value: root.minPct + "–" + root.maxPct
                font: root.style.techFont
                color: root.style.ink
                bracketColor: root.style.muted
            }
        }
    }
}
