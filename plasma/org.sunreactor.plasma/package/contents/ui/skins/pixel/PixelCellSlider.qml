/*
 * A slider made of discrete cells (DESIGN.md section 4, pixel rule 3), inside
 * a PixelSurface (art px). 0..100 % in `cells` cells; the cells outside the
 * display's min..max limits are drawn as a thin floor so the usable range is
 * visible, with ticks at both limits.
 *
 * Keyboard: Left/Down -1, Right/Up +1, Page Up/Down +-10, Home/End 0/100.
 * Mouse: press or drag picks the cell under the pointer. Changes are emitted
 * as `committed(value)` at most every 250 ms while the user is still moving
 * (DDC/CI dislikes request storms) and once more on release.
 * Screen readers see a Slider with value/minimumValue/maximumValue/stepSize.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property Theme theme
    required property PixelStyle style
    /*! Current value 0..100, or -1 when unknown (then the slider is inert). */
    property int value: -1
    property int from: 0
    property int to: 100
    property int cells: 20
    property int cellHeight: 6
    property int minMark: -1
    property int maxMark: -1
    property bool interactive: true
    property string accessibleName: ""
    property string valueText: value >= 0 ? value + "%" : "—"

    signal committed(int value)

    // Accessible value interface (read by QAccessibleQuickItem for sliders).
    readonly property int minimumValue: from
    readonly property int maximumValue: to
    readonly property int stepSize: 1

    property bool editing: false
    property int localValue: 0
    readonly property bool known: value >= 0
    readonly property int shown: editing ? localValue : Math.max(0, value)
    readonly property bool live: interactive && known && enabled

    /*! Cell pitch and gap fit the width; the slider is `cells * pitch - gap` wide. */
    readonly property int pitch: Math.max(3, Math.floor((width + gap) / cells))
    readonly property int gap: style.kind === "c64" || style.kind === "amiga" ? 0 : 1
    readonly property int cellWidth: pitch - gap
    readonly property int trackWidth: cells * pitch - gap
    readonly property int lit: shown <= 0 ? 0 : Math.max(1, Math.round(shown * cells / 100))

    implicitHeight: cellHeight + 3
    height: implicitHeight
    activeFocusOnTab: live
    focus: false

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: valueText
    Accessible.focusable: live
    Accessible.onIncreaseAction: step(1)
    Accessible.onDecreaseAction: step(-1)

    function setLocal(v) {
        const clamped = Math.max(from, Math.min(to, Math.round(v)));
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

    function step(delta) {
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
        return Math.round((cell + 1) * 100 / cells);
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
            if (!mouse.pressed) {
                if (throttle.running) {
                    throttle.stop();
                    root.committed(root.localValue);
                }
                root.editing = false;
            }
        }
    }

    Keys.onPressed: event => {
        let delta = 0;
        switch (event.key) {
        case Qt.Key_Left:
        case Qt.Key_Down:
            delta = -1;
            break;
        case Qt.Key_Right:
        case Qt.Key_Up:
            delta = 1;
            break;
        case Qt.Key_PageDown:
            delta = -10;
            break;
        case Qt.Key_PageUp:
            delta = 10;
            break;
        case Qt.Key_Home:
            delta = root.from - root.shown;
            break;
        case Qt.Key_End:
            delta = root.to - root.shown;
            break;
        default:
            return;
        }
        event.accepted = true;
        root.step(delta);
    }

    // --- cells ---------------------------------------------------------------------
    Repeater {
        model: root.cells

        delegate: Item {
            id: cell

            required property int index
            readonly property int lowPct: Math.round(index * 100 / root.cells)
            readonly property int highPct: Math.round((index + 1) * 100 / root.cells)
            readonly property bool on: index < root.lit
            readonly property bool outside: (root.minMark >= 0 && highPct <= root.minMark)
                || (root.maxMark >= 0 && lowPct >= root.maxMark)
            readonly property bool last: on && index === root.lit - 1

            x: index * root.pitch
            y: 0
            width: root.cellWidth
            height: root.cellHeight

            // Lit cell.
            Rectangle {
                anchors.fill: parent
                visible: cell.on && root.style.kind !== "dots"
                antialiasing: false
                color: !root.known || !root.enabled ? root.style.ghost
                    : (cell.last && (root.editing || root.activeFocus) ? root.style.hot : root.litColor)
            }
            // Nothing: round dots.
            Repeater {
                model: root.style.kind === "dots" && root.cellWidth >= 3 ? 1 : 0
                delegate: Item {
                    width: cell.width
                    height: cell.height
                    Rectangle {
                        x: 1; y: 0; width: cell.width - 2; height: cell.height
                        antialiasing: false
                        color: cell.on ? (cell.last && root.activeFocus ? root.style.hot : root.litColor)
                                       : root.style.ghost
                    }
                    Rectangle {
                        x: 0; y: 1; width: cell.width; height: cell.height - 2
                        antialiasing: false
                        color: cell.on ? (cell.last && root.activeFocus ? root.style.hot : root.litColor)
                                       : root.style.ghost
                    }
                }
            }
            // Unlit cell: ghost block, hollow box (1-bit) or floor (outside the limits).
            Rectangle {
                visible: !cell.on && root.style.kind !== "dots" && !root.hollow
                x: 0
                y: cell.outside ? cell.height - 1 : 0
                width: cell.width
                height: cell.outside ? 1 : cell.height
                antialiasing: false
                color: root.style.ghost
            }
            Item {
                visible: !cell.on && root.hollow
                anchors.fill: parent
                Rectangle { x: 0; y: cell.outside ? cell.height - 1 : 0; width: cell.width; height: 1; color: root.style.ink; antialiasing: false }
                Rectangle { visible: !cell.outside; x: 0; y: cell.height - 1; width: cell.width; height: 1; color: root.style.ink; antialiasing: false }
                Rectangle { visible: !cell.outside; x: 0; y: 0; width: 1; height: cell.height; color: root.style.ink; antialiasing: false }
                Rectangle { visible: !cell.outside; x: cell.width - 1; y: 0; width: 1; height: cell.height; color: root.style.ink; antialiasing: false }
            }
        }
    }

    readonly property bool hollow: style.oneBit
    readonly property color litColor: {
        switch (style.kind) {
        case "amiga":
            return theme.borderActive;
        case "neon":
            return theme.accent;
        default:
            return style.ink;
        }
    }

    // Limit ticks under the track.
    Repeater {
        model: [root.minMark, root.maxMark].filter(v => v >= 0)

        delegate: Rectangle {
            required property int modelData
            x: Math.min(root.trackWidth - 1, Math.max(0, Math.round(modelData / 100 * root.trackWidth) - 1))
            y: root.cellHeight + 1
            width: 1
            height: 2
            antialiasing: false
            color: root.style.dim
        }
    }

    // Keyboard focus: a ring in the emphasis colour, one pixel out.
    Item {
        visible: root.activeFocus
        x: -2
        y: -2
        width: root.trackWidth + 4
        height: root.cellHeight + 4
        Rectangle { x: 0; y: 0; width: parent.width; height: 1; color: root.style.hot; antialiasing: false }
        Rectangle { x: 0; y: parent.height - 1; width: parent.width; height: 1; color: root.style.hot; antialiasing: false }
        Rectangle { x: 0; y: 0; width: 1; height: parent.height; color: root.style.hot; antialiasing: false }
        Rectangle { x: parent.width - 1; y: 0; width: 1; height: parent.height; color: root.style.hot; antialiasing: false }
    }

    MouseArea {
        id: mouse

        x: 0
        y: -1
        width: root.trackWidth
        height: root.cellHeight + 3
        enabled: root.live
        cursorShape: root.live ? Qt.PointingHandCursor : Qt.ArrowCursor
        preventStealing: true
        onPressed: event => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.setLocal(root.valueAtX(event.x));
        }
        onPositionChanged: event => {
            if (pressed) {
                root.setLocal(root.valueAtX(event.x));
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }
}
