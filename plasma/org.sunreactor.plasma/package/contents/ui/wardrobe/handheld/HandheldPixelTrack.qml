/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

import QtQuick

Item {
    id: track

    property real progress: 0.0 // 0.0 to 1.0
    property bool daylight: true
    property color shade0: "#9BBC0F" // Lightest
    property color shade1: "#8BAC0F"
    property color shade2: "#306230"
    property color shade3: "#0F380F" // Darkest
    property color burgundy: "#8B1D42"

    implicitHeight: 16

    // Outer pixel frame (shade 3)
    Rectangle {
        anchors.fill: parent
        color: track.shade1
        border.width: 1
        border.color: track.shade3
        radius: 0

        // Inactive track (shade 2 dither / fill)
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            color: track.shade2
            opacity: 0.3
        }

        // Active elapsed track (shade 3)
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: 1
            width: Math.max(0, (parent.width - 2) * track.progress)
            color: track.shade3
        }

        // Stepped pixel sun indicator
        Rectangle {
            width: 8
            height: 12
            x: Math.max(1, Math.min(parent.width - 9, (parent.width - 8) * track.progress))
            anchors.verticalCenter: parent.verticalCenter
            color: track.burgundy
            border.width: 1
            border.color: track.shade3

            // Center pixel dot
            Rectangle {
                width: 2
                height: 2
                anchors.centerIn: parent
                color: track.shade0
            }
        }
    }
}
