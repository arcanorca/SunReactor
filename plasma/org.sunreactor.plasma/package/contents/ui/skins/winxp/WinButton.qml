/*
 * An XP push button: a 3 px rounded face from white to the panel colour, a
 * 1 px rim. Hover warms the face towards XP's orange-yellow hot state and the
 * ring marks the hovered or the default button. `current` keeps the pressed
 * face on a choice that is on.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property WinStyle style
    property string text: ""
    property bool current: false
    property bool isDefault: false
    readonly property bool hovered: mouse.containsMouse
    readonly property bool down: mouse.pressed

    signal clicked()

    implicitWidth: Math.max(root.style.snap(root.style.sizeBody * 3.6), label.implicitWidth + 2 * root.style.pad)
    implicitHeight: root.style.snap(root.style.sizeBody * 1.7)
    opacity: enabled ? 1 : 0.5
    activeFocusOnTab: true

    Accessible.role: Accessible.Button
    Accessible.name: root.text
    Accessible.onPressAction: root.clicked()
    Keys.onSpacePressed: root.clicked()
    Keys.onReturnPressed: root.clicked()

    Rectangle {
        anchors.fill: parent
        radius: root.style.radiusButton
        border.width: root.style.line
        border.color: root.style.frameDeep
        gradient: Gradient {
            GradientStop {
                position: 0
                color: root.down || root.current ? root.style.pressTop
                    : (root.hovered ? root.style.hoverTop : root.style.white)
            }
            GradientStop {
                position: 1
                color: root.down || root.current ? root.style.pressBottom
                    : (root.hovered ? root.style.hoverBottom : root.style.panel)
            }
        }
    }

    // The orange-yellow ring: the default button, or the one under the pointer.
    Rectangle {
        anchors.fill: parent
        anchors.margins: root.style.snap(root.style.gap * 0.5)
        visible: root.isDefault || root.hovered
        radius: 2
        color: "transparent"
        border.width: root.style.line
        border.color: root.style.orange
    }

    // Keyboard focus: a dark inner outline.
    Rectangle {
        anchors.fill: parent
        anchors.margins: root.style.snap(root.style.gap * 0.5 + root.style.line * 2)
        visible: root.activeFocus
        radius: 1
        color: "transparent"
        border.width: root.style.line
        border.color: root.style.ink
    }

    Text {
        id: label

        anchors.centerIn: parent
        width: Math.min(implicitWidth, root.width - 2 * root.style.pad)
        text: root.text
        textFormat: Text.PlainText
        font: root.style.bodyFont
        color: root.style.ink
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
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
