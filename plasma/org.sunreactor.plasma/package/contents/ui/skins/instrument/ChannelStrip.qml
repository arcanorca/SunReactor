/*
 * One display as a channel strip:
 *
 *   01  mi-monitor                              [MAN] [AUTO]   39 %
 *       ├────┼────┼────┼────┼────┼────┼────┼────┼────┼────┤
 *       ══════════════▮═════════▽════════════════════════   fader
 *       TARGET 41 · APPLIED 39                       RANGE 1–50 ▾
 *
 * The fader writes a manual override (throttled for DDC/CI); the triangle is
 * where the automatic curve wants to be; the range opens the day-maximum /
 * night-minimum faders. Knows nothing about the daemon: it reports what the
 * user asked for and the Displays module decides what that means. Unknown
 * values stay "—" and the fader shows no thumb.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property InstrumentStyle style
    property string channel: ""
    property string name: ""
    /*! What the fader shows: the override, else the applied value; -1 unknown. */
    property int percent: -1
    /*! What the daemon last wrote; -1 unknown. */
    property int applied: -1
    /*! Where the automatic curve is now; -1 unknown. */
    property real target: -1
    property int minPct: 15
    property int maxPct: 60
    property bool hasLimits: false
    property bool hasOverride: false
    property real overrideUntil: 0
    property bool controllable: true
    /*! Short state tag ("OFF", "NO LINK", "MAN"), empty for none. */
    property string stateText: ""
    property color stateTone: style.warning
    property bool limitsExpanded: false

    signal requested(int percent)
    signal limitsRequested(int minPct, int maxPct)
    signal clearOverrideRequested()

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property bool hasValue: percent >= 0

    spacing: style.snap(style.gap * 0.75)
    Layout.fillWidth: true

    Timer {
        id: throttle
        interval: 250
        onTriggered: root.requested(Math.round(fader.value))
    }

    Timer {
        id: limitsThrottle
        interval: 250
        onTriggered: root.limitsRequested(Math.round(minFader.value), Math.round(maxFader.value))
    }

    TextMetrics {
        id: channelMetrics
        font: root.style.dataSmallFont
        text: "ALL"
    }

    readonly property real indent: style.snap(channelMetrics.width + style.gap2)

    // --- name row ------------------------------------------------------------------
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        Text {
            Layout.preferredWidth: root.indent - root.style.gap
            Layout.alignment: Qt.AlignBaseline
            text: root.channel
            textFormat: Text.PlainText
            font: root.style.dataSmallFont
            color: root.style.inkFaint
        }

        Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignBaseline
            text: root.name
            textFormat: Text.PlainText
            font: root.style.bodyFont
            color: root.controllable ? root.style.ink : root.style.inkMuted
            elide: Text.ElideMiddle
        }

        Tag {
            Layout.alignment: Qt.AlignVCenter
            visible: root.stateText.length > 0
            style: root.style
            text: root.stateText
            tone: root.stateTone
            filled: root.hasOverride
        }

        Text {
            Layout.alignment: Qt.AlignBaseline
            visible: root.hasOverride && root.overrideUntil > 0
            text: i18ndc(root.domain, "Placeholder is a time of day", "Until %1", Format.timeText(root.overrideUntil))
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }

        PushButton {
            Layout.alignment: Qt.AlignVCenter
            visible: root.hasOverride
            style: root.style
            text: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
            onClicked: root.clearOverrideRequested()
        }

        Row {
            Layout.alignment: Qt.AlignBaseline
            Layout.leftMargin: root.style.gap
            spacing: root.style.snap(root.style.gap * 0.5)

            Digits {
                id: valueDigits

                style: root.style
                size: Math.round(root.style.sizeMid * 0.74)
                font: root.style.midFont
                text: root.hasValue ? String(root.percent) : "--"
                color: !root.hasValue ? root.style.inkMuted
                    : (fader.pressed || fader.visualFocus ? root.style.accent : root.style.ink)
            }

            Text {
                anchors.baseline: valueDigits.baseline
                text: "%"
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkMuted
            }
        }
    }

    // --- fader ---------------------------------------------------------------------
    Fader {
        id: fader

        Layout.fillWidth: true
        Layout.leftMargin: root.indent
        style: root.style
        known: root.hasValue
        enabled: root.controllable && root.hasValue
        target: root.target
        limitMin: root.hasLimits ? root.minPct : -1
        limitMax: root.hasLimits ? root.maxPct : -1
        value: root.hasValue ? root.percent : 0

        // While dragging, ignore status updates: the daemon reports the value
        // it has written, which lags behind the thumb.
        onPressedChanged: {
            if (pressed) {
                return;
            }
            throttle.stop();
            root.requested(Math.round(value));
            value = Qt.binding(() => root.hasValue ? root.percent : 0);
        }
        onMoved: throttle.restart()

        Accessible.name: root.name
        Accessible.description: root.hasValue ? i18ndc(root.domain, "Brightness percentage", "%1%", root.percent)
                                              : i18ndc(root.domain, "No value is known yet", "—")
    }

    // --- legend row ------------------------------------------------------------------
    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: root.indent
        spacing: root.style.gap2

        Row {
            Layout.alignment: Qt.AlignVCenter
            spacing: root.style.gap
            visible: root.target >= 0

            Text {
                anchors.baseline: targetValue.baseline
                text: i18ndc(root.domain, "Strip legend: where the automatic curve is now", "Target")
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkMuted
            }
            Text {
                id: targetValue
                text: String(Math.round(root.target))
                textFormat: Text.PlainText
                font: root.style.dataSmallFont
                color: root.style.accent2
            }
        }

        Row {
            Layout.alignment: Qt.AlignVCenter
            spacing: root.style.gap
            visible: root.applied >= 0 && root.applied !== root.percent

            Text {
                anchors.baseline: appliedValue.baseline
                text: i18ndc(root.domain, "Strip legend: what the daemon last wrote", "Applied")
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkMuted
            }
            Text {
                id: appliedValue
                text: String(root.applied)
                textFormat: Text.PlainText
                font: root.style.dataSmallFont
                color: root.style.ink
            }
        }

        Text {
            Layout.alignment: Qt.AlignVCenter
            visible: !root.hasValue
            text: i18ndc(root.domain, "Strip legend: the display has not reported a value", "No reading")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }

        Item {
            Layout.fillWidth: true
        }

        Tag {
            Layout.alignment: Qt.AlignVCenter
            visible: root.hasLimits
            style: root.style
            interactive: true
            bare: !root.limitsExpanded
            filled: root.limitsExpanded
            tone: root.limitsExpanded ? root.style.ink : root.style.inkMuted
            text: i18ndc(root.domain, "Strip control: automatic range, opens the limit faders", "Range %1–%2 %3",
                         root.minPct, root.maxPct, root.limitsExpanded ? "▴" : "▾")
            Accessible.name: i18ndc(root.domain, "Range limits", "%1%–%2%", root.minPct, root.maxPct)
            onClicked: root.limitsExpanded = !root.limitsExpanded
        }
    }

    // --- limits: day maximum and night minimum ------------------------------------------
    GridLayout {
        Layout.fillWidth: true
        Layout.leftMargin: root.indent
        Layout.topMargin: root.style.gap
        visible: root.hasLimits && root.limitsExpanded
        columns: 3
        columnSpacing: root.style.gap2
        rowSpacing: root.style.gap

        Text {
            Layout.alignment: Qt.AlignVCenter
            text: i18ndc(root.domain, "Day maximum boundary", "Day max")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }
        Fader {
            id: maxFader

            Layout.fillWidth: true
            style: root.style
            showScale: false
            from: Math.max(0, Math.round(minFader.value))
            to: 100
            value: root.maxPct
            onMoved: limitsThrottle.restart()
            onPressedChanged: {
                if (!pressed) {
                    limitsThrottle.stop();
                    root.limitsRequested(Math.round(minFader.value), Math.round(maxFader.value));
                }
            }
            Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
        }
        Text {
            Layout.alignment: Qt.AlignVCenter
            Layout.minimumWidth: limitMetrics.width
            horizontalAlignment: Text.AlignRight
            text: Math.round(maxFader.value)
            textFormat: Text.PlainText
            font: root.style.dataSmallFont
            color: root.style.ink
        }

        Text {
            Layout.alignment: Qt.AlignVCenter
            text: i18ndc(root.domain, "Night minimum boundary", "Night min")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }
        Fader {
            id: minFader

            Layout.fillWidth: true
            style: root.style
            showScale: false
            from: 0
            to: Math.min(100, Math.round(maxFader.value))
            value: root.minPct
            onMoved: limitsThrottle.restart()
            onPressedChanged: {
                if (!pressed) {
                    limitsThrottle.stop();
                    root.limitsRequested(Math.round(minFader.value), Math.round(maxFader.value));
                }
            }
            Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
        }
        Text {
            Layout.alignment: Qt.AlignVCenter
            Layout.minimumWidth: limitMetrics.width
            horizontalAlignment: Text.AlignRight
            text: Math.round(minFader.value)
            textFormat: Text.PlainText
            font: root.style.dataSmallFont
            color: root.style.ink
        }

        TextMetrics {
            id: limitMetrics
            font: root.style.dataSmallFont
            text: "100"
        }
    }
}
