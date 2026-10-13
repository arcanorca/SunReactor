/*
 * A PETSCII block gauge, `▐████████░░░░▌`: `cells` cells between a right half
 * block and a left half block, filled cells in full blocks, the rest shaded.
 * Click or drag sets the value (committed on release); Left / Right step one
 * cell, PageUp / PageDown five, Home / End go to the ends.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    /*! Cells between the two half blocks (at least one). */
    property int cells: 20
    /*! 0..100 as the daemon reports it. */
    property int value: 0
    /*! False: shows the value, takes no input. */
    property bool active: true
    property color ink: "transparent"
    property color quiet: "transparent"
    property string accessibleName: ""

    /*! Emitted with 0..100 when the user sets a value. */
    signal committed(int value)

    readonly property int slots: Math.max(1, cells)

    /*! Value shown while a drag is in progress or just after release; -1 = none. */
    property int preview: -1
    readonly property int shownValue: preview >= 0 ? preview : value
    readonly property int filled: cellsOf(shownValue)
    readonly property string glyphs: "▐" + "█".repeat(filled) + "░".repeat(slots - filled) + "▌"

    readonly property int artWidth: (slots + 2) * 4
    readonly property int artHeight: 8

    implicitWidth: artWidth
    implicitHeight: artHeight
    width: artWidth
    height: artHeight
    activeFocusOnTab: active

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: shownValue + "%"

    onValueChanged: preview = -1

    function cellsOf(v) {
        return Math.round(Math.max(0, Math.min(100, v)) * slots / 100);
    }

    function valueOf(n) {
        return Math.round(Math.max(0, Math.min(slots, n)) * 100 / slots);
    }

    /*! The value a click at art x `ax` (from the gauge's left edge) sets. */
    function valueAtX(ax) {
        const i = Math.floor(ax / 4) - 1;
        return valueOf(Math.max(0, Math.min(slots - 1, i)) + 1);
    }

    function step(delta) {
        if (active) {
            committed(valueOf(cellsOf(shownValue) + delta));
        }
    }

    Keys.onLeftPressed: step(-1)
    Keys.onRightPressed: step(1)
    Keys.onPressed: event => {
        if (!active) {
            return;
        }
        if (event.key === Qt.Key_PageUp) {
            step(5);
            event.accepted = true;
        } else if (event.key === Qt.Key_PageDown) {
            step(-5);
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            committed(0);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            committed(100);
            event.accepted = true;
        }
    }

    C64Text {
        text: root.glyphs
        ink: root.active ? root.ink : root.quiet
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.active
        cursorShape: Qt.PointingHandCursor

        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.preview = root.valueAtX(mouse.x * root.artWidth / root.width);
        }
        onPositionChanged: mouse => {
            if (pressed) {
                root.preview = root.valueAtX(mouse.x * root.artWidth / root.width);
            }
        }
        onReleased: {
            if (root.preview >= 0) {
                root.committed(root.preview);
            }
        }
    }
}
