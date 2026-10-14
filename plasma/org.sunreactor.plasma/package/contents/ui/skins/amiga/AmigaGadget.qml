/*
 * A Workbench gadget button: raised, with a black outline. `lit` is the chosen
 * one (orange face); a press shows the inverse (black face, white label). The
 * drop-down gadget carries a small triangle after its label (`caret`). Activation
 * by click, Space or Enter; focus shows a dotted inner outline.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "AmigaChrome.js" as Chrome

Item {
    id: root

    required property Theme theme
    required property AmigaStyle style
    property string text: ""
    property bool lit: false
    property bool caret: false
    readonly property bool pressed: area.pressed
    readonly property int labelArt: label.artWidth
    readonly property int caretArt: caret ? 6 : 0
    /*! Art px inside the outline on each side of the label (2; a row that is too narrow may use 1). */
    property int gap: 2

    /*! Art px; the label's own width plus the outline, padding and the drop-down. */
    readonly property int artWidth: labelArt + 2 * gap + caretArt
    readonly property int artHeight: 11

    implicitWidth: artWidth
    implicitHeight: artHeight
    width: artWidth
    height: artHeight

    signal activated()

    Accessible.role: Accessible.Button
    Accessible.name: text
    Accessible.checkable: lit
    Accessible.checked: lit
    Accessible.onPressAction: activated()

    activeFocusOnTab: true
    Keys.onSpacePressed: activated()
    Keys.onReturnPressed: activated()
    Keys.onEnterPressed: activated()

    readonly property string faceState: pressed ? "pressed" : (lit ? "lit" : "raised")

    Repeater {
        model: Chrome.gadgetRects(root.artWidth, root.artHeight, root.faceState, {
            body: root.style.body, face: root.style.face, ink: root.style.ink, hot: root.style.hot })

        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.c
            antialiasing: false
        }
    }

    // Drop-down triangle, 5 x 3 art px, two art px in from the outline.
    Repeater {
        model: root.caret ? [{ x: root.artWidth - 6, y: 4, w: 5, h: 1 }, { x: root.artWidth - 5, y: 5, w: 3, h: 1 },
                             { x: root.artWidth - 4, y: 6, w: 1, h: 1 }] : []

        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: root.pressed ? root.style.body : root.style.ink
            antialiasing: false
        }
    }

    // Focus: a dotted inner outline, one art pixel every other cell.
    Repeater {
        model: root.activeFocus && !root.pressed ? Math.floor((root.artWidth - 6) / 2) : 0

        delegate: Rectangle {
            required property int index
            x: 3 + index * 2
            y: 2
            width: 1
            height: 1
            color: root.style.ink
            antialiasing: false
        }
    }

    AmigaText {
        id: label

        x: Math.floor((root.artWidth - root.caretArt - artWidth) / 2)
        y: Math.floor((root.artHeight - capHeight) / 2)
        text: root.text
        color: root.pressed ? root.style.body : root.style.ink
    }

    MouseArea {
        id: area

        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.activated();
        }
    }
}
