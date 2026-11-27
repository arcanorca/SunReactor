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

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    // Extracted NTR Home Menu square analog dial (1px #000000 outer frame,
    // 6x6 #D0D0D0 grid, 12/3/6/9 numerals, and 8 hour markers)
    Image {
        anchors.fill: parent
        source: Qt.resolvedUrl("assets/ds_clock_dial.png")
        fillMode: Image.Stretch
        smooth: false
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
            opacity: 0.35
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
