/*
 * Circular OLED dot-matrix target dial, Nothing OS style.
 *
 * Uses polar coordinates so the ring is geometrically circular at every DPR
 * rather than snapped onto a coarse square dot grid. Zero idle CPU: dots only
 * update when `value` or `known` changes.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property NothingStyle style
    /*! Target brightness in [0, 1]. */
    property real value: 0
    /*! True when a live target is known. */
    property bool known: true
    /*! Display text inside the dial ("39" or "--"). */
    property string digits: "--"

    readonly property int dotCount: 36
    readonly property real dialSize: style.snap(style.unit * 5.65)
    readonly property real center: dialSize / 2
    readonly property real ringRadius: style.snap(dialSize * 0.425)
    readonly property real dotSize: style.snap(Math.max(3, style.unit * 0.21))
    readonly property real headSize: style.snap(dotSize * 1.45)
    readonly property real clampedValue: Math.max(0, Math.min(1, value))
    readonly property int litDots: known
        ? (clampedValue <= 0 ? 0 : Math.max(1, Math.min(dotCount, Math.round(clampedValue * dotCount))))
        : 0

    implicitWidth: dialSize
    implicitHeight: dialSize

    // Recessed OLED circular dial backing + crisp hairline bezel
    Rectangle {
        anchors.fill: parent
        radius: width / 2
        antialiasing: true
        color: root.style.dialSurface
        border.width: root.style.hairline
        border.color: root.style.outline
    }

    // Subtle 12 / 3 / 6 / 9 cardinal micro-indices outside the dot ring
    Repeater {
        model: 4

        delegate: Rectangle {
            required property int index

            readonly property real angle: index * Math.PI / 2 - Math.PI / 2
            readonly property real tickR: root.dialSize * 0.478
            readonly property real tickSize: root.style.snap(2)

            width: tickSize
            height: tickSize
            radius: tickSize / 2
            antialiasing: true
            x: root.center + tickR * Math.cos(angle) - width / 2
            y: root.center + tickR * Math.sin(angle) - height / 2
            color: index === 0 ? root.style.theme.alpha(root.style.ink, 0.36) : root.style.dotGhost
        }
    }

    // True circular 36-LED perimeter gauge
    Repeater {
        model: root.dotCount

        delegate: Rectangle {
            required property int index

            readonly property real angle: index * (2 * Math.PI / root.dotCount) - Math.PI / 2
            readonly property bool isLit: index < root.litDots
            readonly property bool isHead: root.known && root.litDots > 0 && index === root.litDots - 1
            readonly property real d: isHead ? root.headSize : root.dotSize

            width: d
            height: d
            radius: d / 2
            antialiasing: true
            x: root.center + root.ringRadius * Math.cos(angle) - width / 2
            y: root.center + root.ringRadius * Math.sin(angle) - height / 2
            color: isHead ? root.style.accent : (isLit ? root.style.ink : root.style.dotGhost)
        }
    }

    // Centered Doto numeral + clean monochrome "%" unit
    RowLayout {
        anchors.centerIn: parent
        spacing: Math.max(1, Math.round(root.style.gap * 0.25))

        Text {
            Layout.alignment: Qt.AlignBaseline
            text: root.digits
            textFormat: Text.PlainText
            font: root.style.dotHero
            color: root.known ? root.style.ink : root.style.muted
        }

        Text {
            Layout.alignment: Qt.AlignBaseline
            text: "%"
            textFormat: Text.PlainText
            font: root.style.label
            color: root.style.muted
        }
    }
}
