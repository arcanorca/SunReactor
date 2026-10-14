/*
 * A horizontal tick ruler: minor and major ticks on whole device pixels, with
 * optional numerals under (or over) the majors. Used under the light cycle
 * (hours), above faders (percent) and on the sun-times day scale.
 *
 *   from/to            value range mapped to the full width
 *   minor/major        tick spacing in value units (0 = none)
 *   labelEvery         numeral spacing in value units (0 = none)
 *   label(v)           numeral text for a value
 *   ticksDown          true: ticks hang from the top edge, numerals below
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property InstrumentStyle style
    property real from: 0
    property real to: 100
    property real minor: 10
    property real major: 50
    property real labelEvery: 0
    property var label: v => String(v)
    property bool ticksDown: true
    property real minorLength: style.snap(style.sizeLegend * 0.35)
    property real majorLength: style.snap(style.sizeLegend * 0.7)
    property color tickColor: style.ruleStrong
    property color minorColor: style.rule
    property color labelColor: style.inkFaint
    /*! Keep the end numerals inside the ruler instead of centring them. */
    property bool clampLabels: true

    readonly property real span: Math.max(1e-6, to - from)
    readonly property real tickBand: majorLength
    readonly property real labelHeight: labelEvery > 0 ? metrics.height : 0

    function xAt(v) {
        // The last tick sits on the last device pixel column, not past it: rounding
        // to the nearest device pixel can put it half a pixel beyond the ruler.
        const x = style.snap((v - from) / span * (width - style.hairline));
        return Math.min(x, style.floorSnap(width - style.hairline));
    }

    implicitHeight: style.snap(tickBand + (labelEvery > 0 ? style.hairline * 2 + labelHeight : 0))

    TextMetrics {
        id: metrics
        font: root.style.dataSmallFont
        text: "00"
    }

    Repeater {
        model: root.minor > 0 ? Math.floor(root.span / root.minor + 1e-6) + 1 : 0

        delegate: Rectangle {
            required property int index
            readonly property real value: root.from + index * root.minor
            readonly property bool isMajor: root.major > 0
                && Math.abs(value / root.major - Math.round(value / root.major)) < 1e-6

            x: root.xAt(value)
            y: root.ticksDown ? 0 : root.height - height
            width: root.style.hairline
            height: isMajor ? root.majorLength : root.minorLength
            color: isMajor ? root.tickColor : root.minorColor
        }
    }

    Repeater {
        model: root.labelEvery > 0 ? Math.floor(root.span / root.labelEvery + 1e-6) + 1 : 0

        delegate: Text {
            required property int index
            readonly property real value: root.from + index * root.labelEvery
            readonly property real centre: root.xAt(value) - implicitWidth / 2

            x: root.clampLabels ? Math.max(0, Math.min(root.width - implicitWidth, centre)) : centre
            y: root.ticksDown ? root.tickBand + root.style.hairline * 2 : 0
            text: root.label(value)
            textFormat: Text.PlainText
            font: root.style.dataSmallFont
            color: root.labelColor
        }
    }
}
