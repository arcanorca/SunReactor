/*
 * Nintendo DS Home Menu square analog clock, driven by the daemon clock
 * (client.nowEpochS). Dial geometry and proportions come from NDS-Clock-SVG-Kit
 * (canonical 388x388 units: 6x6 grid, 12/3/6/9 numerals, 8 hour markers, 20u
 * center pivot, 96u hour / 126u minute / 146u second hands), rendered with
 * pixel-snapped QML geometry so the #000000 frame and 6x6 grid stay crisp at
 * every DPR and popup scale.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

Item {
    id: root

    required property NdsStyle style
    required property SunReactorClient client
    property real size: Math.round(style.gu * 3.1)

    readonly property bool hasTime: client.nowEpochS > 0
    readonly property var localDate: hasTime ? new Date(client.nowEpochS * 1000) : null
    readonly property real seconds: localDate ? localDate.getSeconds() : 0
    readonly property real minutes: localDate ? localDate.getMinutes() + seconds / 60 : 0
    readonly property real hours: localDate ? (localDate.getHours() % 12) + minutes / 60 : 0

    // Reference pose from NDS-Clock-SVG-Kit extraction_report.json when time is unavailable
    readonly property real hourAngle: localDate ? hours * 30 : 72.78
    readonly property real minuteAngle: localDate ? minutes * 6 : 350.8
    readonly property real secondAngle: localDate ? seconds * 6 : 297.04

    readonly property real cx: width / 2
    readonly property real cy: height / 2
    readonly property real unitScale: width / 388

    // Exact orthogonal rects in 388x388 canonical space from nds_home_clock_dial.svg
    readonly property var numeralRects: [
        // 1
        [162, 16, 16, 80],
        // 2
        [182, 16, 44, 16], [210, 32, 16, 16], [182, 48, 44, 16], [182, 64, 16, 16], [182, 80, 44, 16],
        // 3
        [326, 154, 48, 16], [358, 170, 16, 16], [326, 186, 48, 16], [358, 202, 16, 16], [326, 218, 48, 16],
        // 6
        [170, 292, 48, 16], [170, 308, 16, 64], [186, 324, 32, 16], [202, 340, 16, 16], [186, 356, 32, 16],
        // 9
        [14, 154, 48, 16], [14, 170, 16, 16], [46, 170, 16, 64], [14, 186, 32, 16], [14, 218, 32, 16]
    ]
    readonly property var markerRects: [
        [98, 34, 16, 16], [274, 34, 16, 16],
        [34, 98, 16, 16], [338, 98, 16, 16],
        [34, 274, 16, 16], [338, 274, 16, 16],
        [98, 338, 16, 16], [274, 338, 16, 16]
    ]

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    // Face + crisp 1px #000000 outer frame matching the chart card
    Rectangle {
        anchors.fill: parent
        radius: 0
        color: root.style.cardSurface
        border.width: 1
        border.color: root.style.frameBlack
    }

    // Subtle 2px-pitch LCD scanlines (#EFEFEF) inside the frame
    Repeater {
        model: Math.max(0, Math.floor((root.height - 2) / 2))

        delegate: Rectangle {
            required property int index

            x: 1
            y: 2 + index * 2
            width: Math.max(0, root.width - 2)
            height: 1
            color: root.style.cardScanline
        }
    }

    // 6x6 internal grid lines (#D0D0D0, 1px crisp)
    Repeater {
        model: [1, 2, 3, 4, 5]

        delegate: Item {
            required property int modelData

            readonly property int pos: Math.round(1 + (root.width - 2) * modelData / 6)

            Rectangle {
                x: parent.pos
                y: 1
                width: 1
                height: Math.max(0, root.height - 2)
                color: root.style.gridInner
            }
            Rectangle {
                x: 1
                y: parent.pos
                width: Math.max(0, root.width - 2)
                height: 1
                color: root.style.gridInner
            }
        }
    }

    // 12 / 3 / 6 / 9 numerals (#B0B0B0)
    Repeater {
        model: root.numeralRects

        delegate: Rectangle {
            required property var modelData

            readonly property int x0: Math.round(modelData[0] * root.unitScale)
            readonly property int y0: Math.round(modelData[1] * root.unitScale)
            readonly property int x1: Math.round((modelData[0] + modelData[2]) * root.unitScale)
            readonly property int y1: Math.round((modelData[1] + modelData[3]) * root.unitScale)

            x: x0
            y: y0
            width: Math.max(1, x1 - x0)
            height: Math.max(1, y1 - y0)
            color: root.style.clockNumerals
        }
    }

    // 8 square hour markers (#787878)
    Repeater {
        model: root.markerRects

        delegate: Rectangle {
            required property var modelData

            readonly property int x0: Math.round(modelData[0] * root.unitScale)
            readonly property int y0: Math.round(modelData[1] * root.unitScale)
            readonly property int x1: Math.round((modelData[0] + modelData[2]) * root.unitScale)
            readonly property int y1: Math.round((modelData[1] + modelData[3]) * root.unitScale)

            x: x0
            y: y0
            width: Math.max(2, x1 - x0)
            height: Math.max(2, y1 - y0)
            color: root.style.clockHands
        }
    }

    // Hour hand (96u long, 8u thick)
    Rectangle {
        id: hourHand

        readonly property int handW: Math.max(2, Math.round(8 * root.unitScale))
        readonly property int handL: Math.round(96 * root.unitScale)

        x: root.cx - handW / 2
        y: root.cy - handL
        width: handW
        height: handL
        color: root.style.clockHands
        antialiasing: true

        transform: Rotation {
            origin.x: hourHand.width / 2
            origin.y: hourHand.height
            angle: root.hourAngle
        }
    }

    // Minute hand (126u long, 8u thick)
    Rectangle {
        id: minuteHand

        readonly property int handW: Math.max(2, Math.round(8 * root.unitScale))
        readonly property int handL: Math.round(126 * root.unitScale)

        x: root.cx - handW / 2
        y: root.cy - handL
        width: handW
        height: handL
        color: root.style.clockHands
        antialiasing: true

        transform: Rotation {
            origin.x: minuteHand.width / 2
            origin.y: minuteHand.height
            angle: root.minuteAngle
        }
    }

    // Second hand (146u long, 7u thick, contrast-safe IPL accent)
    Rectangle {
        id: secondHand

        readonly property int handW: Math.max(1, Math.round(7 * root.unitScale))
        readonly property int handL: Math.round(146 * root.unitScale)

        x: root.cx - handW / 2
        y: root.cy - handL
        width: handW
        height: handL
        color: root.style.accentInk
        antialiasing: true

        transform: Rotation {
            origin.x: secondHand.width / 2
            origin.y: secondHand.height
            angle: root.secondAngle
        }
    }

    // Center pivot (20u square, #47484A)
    Rectangle {
        readonly property int piv: Math.max(4, Math.round(20 * root.unitScale))

        x: Math.round(root.cx - piv / 2)
        y: Math.round(root.cy - piv / 2)
        width: piv
        height: piv
        radius: 0
        color: root.style.clockPivot
    }
}
