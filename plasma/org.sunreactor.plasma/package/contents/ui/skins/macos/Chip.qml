/*
 * A small rounded gray control (the Auto return and the Range disclosure of a
 * display). A press emits clicked(); `open` turns its chevron.
 */

pragma ComponentBehavior: Bound

import QtQuick

Rectangle {
    id: root

    required property MacStyle style
    property alias text: label.text
    property bool chevron: false
    property bool open: false

    signal clicked()

    implicitWidth: row.implicitWidth + 2 * 8
    implicitHeight: style.chipHeight
    radius: height / 2
    antialiasing: true
    color: mouse.containsMouse ? style.segmentOn : style.chip

    activeFocusOnTab: true

    Accessible.role: Accessible.Button
    Accessible.name: label.text
    Accessible.onPressAction: root.clicked()

    Keys.onSpacePressed: root.clicked()
    Keys.onReturnPressed: root.clicked()

    Row {
        id: row

        anchors.centerIn: parent
        spacing: 4

        Text {
            id: label

            anchors.verticalCenter: parent.verticalCenter
            textFormat: Text.PlainText
            font: root.style.captionMedium
            color: root.style.ink
            Accessible.ignored: true
        }

        Chevron {
            visible: root.chevron
            anchors.verticalCenter: parent.verticalCenter
            size: 6
            up: root.open
            color: root.style.ink
        }
    }

    Rectangle {
        visible: root.activeFocus
        anchors.fill: parent
        radius: height / 2
        color: "transparent"
        border.width: root.style.hairline * 2
        border.color: root.style.accent
        antialiasing: true
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.clicked();
        }
    }
}
