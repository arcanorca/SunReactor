/*
 * A reverse-video keycap: the label in the screen colour on a solid block of
 * `fill` (light blue, or white when lit), the block exactly as wide as its cells
 * (no padding, as on the C64 screen). Used for the function-key tabs and the
 * mode control. Click, Space and Enter activate it; the focus ring is one art px
 * inside the block.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    property string text: ""
    property color fill: "transparent"
    property color ink: "transparent"
    property bool lit: false
    /*! Blank art px either side of the label (0: the block is the label's cells). */
    property int padX: 0

    signal activated()

    readonly property int artWidth: label.artWidth + 2 * padX
    readonly property int artHeight: 8

    implicitWidth: artWidth
    implicitHeight: artHeight
    width: artWidth
    height: artHeight
    activeFocusOnTab: true

    Accessible.role: Accessible.Button
    Accessible.name: text
    Accessible.checkable: true
    Accessible.checked: lit
    Accessible.onPressAction: root.activated()

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.activated();
            event.accepted = true;
        }
    }

    Rectangle {
        anchors.fill: parent
        color: root.fill
        antialiasing: false
    }

    Rectangle {
        visible: root.activeFocus
        anchors.fill: parent
        color: "transparent"
        border.width: 1
        border.color: root.ink
        antialiasing: false
    }

    C64Text {
        id: label

        x: root.padX
        text: root.text
        ink: root.ink
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.activated();
        }
    }
}
