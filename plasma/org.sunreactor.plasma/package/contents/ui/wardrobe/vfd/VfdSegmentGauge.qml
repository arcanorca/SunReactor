/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: gauge

    property real value: 0.0 // 0.0 to 1.0
    property int segments: 24
    property color phosphorCyan: "#00F0A8"
    property color phosphorAmber: "#FFB000"
    property color ghostColor: "#051A13"

    implicitHeight: 14

    Row {
        anchors.fill: parent
        spacing: Math.max(1, (parent.width - (gauge.segments * 3)) / Math.max(1, gauge.segments - 1))

        Repeater {
            model: gauge.segments

            Rectangle {
                id: seg
                required property int index

                width: 3
                height: parent.height - 2
                anchors.verticalCenter: parent.verticalCenter
                radius: 1

                readonly property real segFraction: (index + 1) / gauge.segments
                readonly property bool isActive: segFraction <= gauge.value
                readonly property bool isPeak: segFraction > 0.80

                color: {
                    if (seg.isActive) {
                        return seg.isPeak ? gauge.phosphorAmber : gauge.phosphorCyan;
                    }
                    return gauge.ghostColor;
                }

                // Phosphor bloom
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: -1
                    radius: 2
                    visible: seg.isActive
                    color: "transparent"
                    border.width: 1
                    border.color: seg.isPeak ? gauge.phosphorAmber : gauge.phosphorCyan
                    opacity: 0.35
                }
            }
        }
    }
}
