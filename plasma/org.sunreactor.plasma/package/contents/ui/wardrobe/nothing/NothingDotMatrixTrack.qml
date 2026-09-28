/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: track

    property real progress: 0.0 // 0.0 to 1.0
    property bool daylight: true
    property color activeColor: "#FFFFFF"
    property color dotColor: "#E50914" // Nothing Red
    property int dotCount: 28

    implicitHeight: 14

    Row {
        anchors.fill: parent
        spacing: Math.max(2, (parent.width - (track.dotCount * 4)) / Math.max(1, track.dotCount - 1))

        Repeater {
            model: track.dotCount

            Rectangle {
                id: dot
                required property int index

                width: 4
                height: 4
                radius: 2
                anchors.verticalCenter: parent.verticalCenter

                readonly property real dotFraction: index / Math.max(1, track.dotCount - 1)
                readonly property bool isPast: dotFraction <= track.progress
                readonly property bool isCurrent: Math.abs(dotFraction - track.progress) < (1.0 / track.dotCount)

                color: {
                    if (dot.isCurrent) return track.dotColor;
                    if (dot.isPast) return track.daylight ? track.activeColor : Qt.rgba(1, 1, 1, 0.4);
                    return Qt.rgba(1, 1, 1, 0.12);
                }

                // Subtle breathing pulse for current sun position dot
                SequentialAnimation on opacity {
                    running: dot.isCurrent && track.visible
                    loops: Animation.Infinite
                    NumberAnimation { to: 0.4; duration: 800; easing.type: Easing.InOutQuad }
                    NumberAnimation { to: 1.0; duration: 800; easing.type: Easing.InOutQuad }
                }
            }
        }
    }
}
