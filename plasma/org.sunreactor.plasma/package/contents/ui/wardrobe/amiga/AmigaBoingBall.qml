/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: ball

    property int size: 16
    property bool running: true

    implicitWidth: size
    implicitHeight: size

    Image {
        id: ballImg
        anchors.fill: parent
        source: Qt.resolvedUrl("../../../images/amiga-boing-ball.svg")
        fillMode: Image.PreserveAspectFit
        smooth: true
        asynchronous: true
        cache: true

        RotationAnimation on rotation {
            running: ball.running && ball.visible
            loops: Animation.Infinite
            from: 0
            to: 360
            duration: 3200
        }
    }
}
