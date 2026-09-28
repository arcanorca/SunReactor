/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

import QtQuick

Item {
    id: glow

    property real progress: 0.0
    property color neonCore: "#FFB765"
    property color neonGlow: "#FF7A18"
    property color glassDark: "#0B0A09"

    implicitHeight: 14

    // Dark glass tube envelope
    Rectangle {
        anchors.fill: parent
        radius: 4
        color: glow.glassDark
        border.width: 1
        border.color: "#3D2413"

        // Anode wire mesh overlay
        Image {
            anchors.fill: parent
            source: Qt.resolvedUrl("../../../images/nixie-mesh-tile.svg")
            fillMode: Image.Tile
            opacity: 0.28
        }

        // Discharge glow line (diffuse outer glow)
        Rectangle {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: 2
            width: Math.max(0, (parent.width - 4) * glow.progress)
            height: parent.height - 4
            radius: 3
            color: glow.neonGlow
            opacity: 0.45
        }

        // Hot plasma core line
        Rectangle {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: 4
            width: Math.max(0, (parent.width - 8) * glow.progress)
            height: 2
            radius: 1
            color: glow.neonCore
            opacity: 0.95
        }

        // Sun cathode point
        Rectangle {
            width: 8
            height: 8
            radius: 4
            x: Math.max(2, Math.min(parent.width - 10, (parent.width - 8) * glow.progress))
            anchors.verticalCenter: parent.verticalCenter
            color: glow.neonCore
            border.width: 1
            border.color: glow.neonGlow

            SequentialAnimation on opacity {
                loops: Animation.Infinite
                NumberAnimation { to: 0.6; duration: 400; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 1.0; duration: 400; easing.type: Easing.InOutQuad }
            }
        }
    }
}
