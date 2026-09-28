/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: guru

    property string logicalId: ""
    property string explanation: i18ndc("plasma_applet_org.sunreactor.plasma", "Amiga guru meditation alert", "Software Failure · Display Unreachable")
    property string fontName: "VT323"

    implicitWidth: layout.implicitWidth + 16
    implicitHeight: layout.implicitHeight + 12

    Rectangle {
        id: alertBox
        anchors.fill: parent
        color: "#000000"
        border.width: 2
        border.color: "#FF0000"

        // Inner red border
        Rectangle {
            anchors.fill: parent
            anchors.margins: 2
            color: "transparent"
            border.width: 1
            border.color: "#FF0000"
        }

        SequentialAnimation on opacity {
            running: guru.visible
            loops: Animation.Infinite
            NumberAnimation { to: 0.25; duration: 600; easing.type: Easing.InOutQuad }
            NumberAnimation { to: 1.0; duration: 600; easing.type: Easing.InOutQuad }
        }
    }

    ColumnLayout {
        id: layout
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.margins: 6
        spacing: 1

        Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignHCenter
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
            text: guru.explanation
            color: "#FF0000"
            font.family: guru.fontName
            font.pixelSize: 13
            font.bold: true
        }

        Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignHCenter
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
            text: "Guru Meditation #00000004.DDC_" + (guru.logicalId.length > 0 ? guru.logicalId : "FAIL")
            color: "#FF0000"
            font.family: guru.fontName
            font.pixelSize: 11
        }
    }
}
