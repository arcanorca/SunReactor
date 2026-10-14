/*
 * A brightness slider of 20 LCD cells, 5 % each. Lit cells are the ink, the
 * rest the ghost; the cells outside the display's limits show only a floor, and
 * a backlit marker shows where the curve targets the display.
 *
 * Keyboard: Left/Down -1, Right/Up +1, Page Up/Down +-10, Home/End 0/100.
 * Mouse: press or drag picks the cell under the pointer. `committed(value)` is
 * emitted at most every 250 ms while the user is moving (DDC/CI dislikes request
 * storms) and once more on release. A Slider to screen readers.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property CasioStyle style
    /*! 0..100, or -1 when unknown (then the bar is inert). */
    property int value: -1
    property int minPct: -1
    property int maxPct: -1
    /*! Where the curve targets the display, or -1. */
    property int targetPct: -1
    property bool interactive: true
    property string accessibleName: ""

    readonly property int cells: 20
    readonly property real pitch: width / cells
    readonly property real cellWidth: pitch * 0.7
    readonly property real headroom: style.snap(style.sizeCap * 0.7)
    readonly property real cellHeight: style.snap(style.unit * 0.8)

    readonly property bool known: value >= 0
    readonly property bool live: interactive && known && enabled
    property bool editing: false
    property int localValue: 0
    readonly property int shown: editing ? localValue : Math.max(0, value)
    readonly property int lit: shown <= 0 ? 0 : Math.round(shown * cells / 100)

    signal committed(int value)

    // Accessible value interface (read by QAccessibleQuickItem for sliders).
    readonly property int minimumValue: 0
    readonly property int maximumValue: 100
    readonly property int stepSize: 1

    implicitHeight: headroom + cellHeight
    implicitWidth: 120
    activeFocusOnTab: live

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: known ? value + "%" : ""
    Accessible.focusable: live
    Accessible.onIncreaseAction: nudge(1)
    Accessible.onDecreaseAction: nudge(-1)

    function setLocal(v) {
        const clamped = Math.max(0, Math.min(100, Math.round(v)));
        if (!editing) {
            editing = true;
            localValue = Math.max(0, value);
        }
        if (clamped === localValue) {
            return;
        }
        localValue = clamped;
        throttle.restart();
    }

    function nudge(delta) {
        if (!live) {
            return;
        }
        setLocal((editing ? localValue : value) + delta);
        settle.restart();
    }

    function finish() {
        throttle.stop();
        if (editing) {
            root.committed(localValue);
        }
        settle.restart();
    }

    function valueAtX(x) {
        const cell = Math.max(0, Math.min(cells - 1, Math.floor(x / pitch)));
        return (cell + 1) * 100 / cells;
    }

    // While the daemon catches up, keep showing what the user picked.
    Timer {
        id: throttle

        interval: 250
        onTriggered: root.committed(root.localValue)
    }

    Timer {
        id: settle

        interval: 1500
        onTriggered: {
            if (!pointer.pressed) {
                if (throttle.running) {
                    throttle.stop();
                    root.committed(root.localValue);
                }
                root.editing = false;
            }
        }
    }

    Keys.onPressed: event => {
        switch (event.key) {
        case Qt.Key_Left:
        case Qt.Key_Down:
            root.nudge(-1);
            break;
        case Qt.Key_Right:
        case Qt.Key_Up:
            root.nudge(1);
            break;
        case Qt.Key_PageDown:
            root.nudge(-10);
            break;
        case Qt.Key_PageUp:
            root.nudge(10);
            break;
        case Qt.Key_Home:
            root.nudge(-root.shown);
            break;
        case Qt.Key_End:
            root.nudge(100 - root.shown);
            break;
        default:
            return;
        }
        event.accepted = true;
    }

    // The target marker: a backlit square above the cell the curve aims at.
    readonly property int targetCell: Math.max(0, Math.min(cells - 1, Math.ceil(targetPct * cells / 100) - 1))
    readonly property real markSize: style.snap(style.sizeCap * 0.45)

    Rectangle {
        visible: root.targetPct >= 0 && root.known
        x: root.style.snap(root.targetCell * root.pitch + (root.pitch - root.markSize) / 2)
        y: 0
        width: root.markSize
        height: root.markSize
        color: root.style.backlight
    }

    // The cells.
    Repeater {
        model: root.cells

        delegate: Rectangle {
            id: cell

            required property int index

            readonly property int hi: (index + 1) * 100 / root.cells
            readonly property int lo: index * 100 / root.cells
            readonly property bool on: index < root.lit
            readonly property bool inRange: (root.minPct < 0 || hi > root.minPct)
                && (root.maxPct < 0 || lo < root.maxPct)

            x: root.style.snap(index * root.pitch + (root.pitch - root.cellWidth) / 2)
            width: Math.max(root.style.hairline, root.style.snap(index * root.pitch
                + (root.pitch + root.cellWidth) / 2) - x)
            // Outside the limits a cell is only a floor, on the bar's baseline.
            readonly property real floorHeight: root.style.snap(root.cellHeight * 0.14)
            y: on || inRange ? root.headroom : root.headroom + root.cellHeight - floorHeight
            height: on || inRange ? root.cellHeight : floorHeight
            color: {
                if (on) {
                    return inRange ? root.style.ink : root.style.inkMuted;
                }
                return inRange ? root.style.ghost : root.style.inkFaint;
            }
        }
    }

    MouseArea {
        id: pointer

        anchors.fill: parent
        enabled: root.live
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.setLocal(root.valueAtX(mouse.x));
        }
        onPositionChanged: mouse => {
            if (pressed) {
                root.setLocal(root.valueAtX(mouse.x));
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }

    // Keyboard focus: a backlit frame around the whole bar.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -root.style.line2 * 2
        visible: root.activeFocus
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.backlight
    }
}
