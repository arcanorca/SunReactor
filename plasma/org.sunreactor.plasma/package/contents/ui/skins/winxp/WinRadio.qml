/*
 * An XP radio button: a round well with a white-to-panel face, the blue dot
 * when chosen, the caption beside it. `isDefault` puts the default ring round
 * the caption: a blue rim with the orange-yellow ring inside.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property WinStyle style
    property string text: ""
    property bool checked: false
    property bool isDefault: false
    readonly property real well: root.style.snap(root.style.sizeBody * 1.1)

    signal clicked()

    implicitWidth: caption.x + caption.width
    implicitHeight: root.style.snap(Math.max(root.well, root.style.rowH))
    activeFocusOnTab: true

    Accessible.role: Accessible.RadioButton
    Accessible.name: root.text
    Accessible.checkable: true
    Accessible.checked: root.checked
    Accessible.onPressAction: root.clicked()
    Keys.onSpacePressed: root.clicked()

    Rectangle {
        id: dot

        x: 0
        y: (root.height - height) / 2
        width: root.well
        height: root.well
        radius: width / 2
        border.width: root.style.line
        border.color: root.style.frameDeep
        gradient: Gradient {
            GradientStop { position: 0; color: root.style.white }
            GradientStop { position: 1; color: root.style.panel }
        }

        Rectangle {
            anchors.centerIn: parent
            width: Math.round(dot.width * 0.46)
            height: width
            radius: width / 2
            color: root.style.accent
            visible: root.checked
        }
    }

    Item {
        id: caption

        x: root.well + root.style.gap
        width: label.implicitWidth + 2 * root.style.gap
        height: root.height

        Rectangle {
            anchors.fill: parent
            visible: root.isDefault
            radius: 3
            color: "transparent"
            border.width: root.style.line
            border.color: root.style.frameDeep

            Rectangle {
                anchors.fill: parent
                anchors.margins: root.style.snap(root.style.line * 2)
                radius: 2
                color: "transparent"
                border.width: root.style.line
                border.color: root.style.orange
            }
        }

        Text {
            id: label

            anchors.centerIn: parent
            text: root.text
            textFormat: Text.PlainText
            font: root.style.bodyFont
            color: root.style.ink
        }
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.clicked();
        }
    }
}
