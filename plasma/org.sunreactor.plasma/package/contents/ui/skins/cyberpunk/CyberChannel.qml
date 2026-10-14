/*
 * One display's row in the Displays panel, cyberpunk. Line one: the channel, the
 * name (it elides first), the state tag and the value in brackets. Line two: the
 * slanted-cell bar, with the automatic target and range in tiny type at its right
 * end. The target is left out when the curve asks for what is already applied.
 * The master row (all displays, no channel) is the bar alone. A ruled row sits
 * under a hairline: the rows share one panel.
 *
 *   01 MI-MONITOR                            MAN [39%]
 *   ▰▰▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱   TARGET [41] RANGE [15–60]
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property CyberStyle style
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
    /*! A hairline above the row: the row follows another one in the panel. */
    property bool ruled: false

    readonly property bool master: channel.length === 0
    readonly property bool showTarget: !master && target >= 0 && Math.round(target) !== applied
    readonly property bool showRange: !master && hasLimits

    signal requested(int percent)

    spacing: style.gap * 0.5

    Rectangle {
        visible: root.ruled
        Layout.fillWidth: true
        Layout.topMargin: root.style.gap * 0.5
        implicitHeight: root.style.hairline
        color: root.style.ruleFaint
    }

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
            // The name gives way first: it elides rather than push the value off the row.
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            Layout.preferredWidth: 0
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
            font: root.style.figFont
            color: root.style.ink
            bracketColor: root.style.muted
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

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

        // The tiny readouts at the bar's right end; each drops out when it adds nothing.
        RowLayout {
            visible: root.showTarget || root.showRange
            Layout.alignment: Qt.AlignVCenter
            spacing: root.style.gap

            Text {
                visible: root.showTarget
                Layout.alignment: Qt.AlignVCenter
                text: root.domain.length > 0
                    ? i18ndc(root.domain, "Label before the automatic target value", "TARGET") : "TARGET"
                textFormat: Text.PlainText
                font: root.style.tinyFont
                color: root.style.muted
            }
            Bracket {
                visible: root.showTarget
                Layout.alignment: Qt.AlignVCenter
                style: root.style
                value: String(Math.round(root.target))
                font: root.style.tinyFont
                color: root.style.cyan
                bracketColor: root.style.muted
            }
            Text {
                visible: root.showRange
                Layout.alignment: Qt.AlignVCenter
                text: root.domain.length > 0
                    ? i18ndc(root.domain, "Label before the automatic minimum and maximum", "RANGE") : "RANGE"
                textFormat: Text.PlainText
                font: root.style.tinyFont
                color: root.style.muted
            }
            Bracket {
                visible: root.showRange
                Layout.alignment: Qt.AlignVCenter
                style: root.style
                value: root.minPct + "–" + root.maxPct
                font: root.style.tinyFont
                color: root.style.ink
                bracketColor: root.style.muted
            }
        }
    }
}
