/*
 * A bar of slanted (parallelogram) cells: the lit cells up to the value, ghost
 * cells for the rest, the automatic target as a cyan triangle, the automatic
 * range as a rail under the cells.
 *
 * As a slider it behaves like the pixel skin's cell slider: a click or drag picks
 * the cell under the pointer; Left/Down -1, Right/Up +1, PageUp/PageDown +-10,
 * Home/End to 0 and 100. Changes are committed at most every 250 ms while the
 * user is still moving, and once on release. Off (interactive false) it is a
 * static meter, for the weather multiplier and cloud cover.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "Geometry.js" as G

Item {
    id: root

    required property CyberStyle style
    /*! 0..100; -1 when unknown (nothing is lit and the slider is inert). */
    property int value: -1
    property int cells: 20
    /*! Where the automatic curve wants the value now; -1 for none. */
    property real target: -1
    /*! The automatic range; -1 for none. */
    property int minMark: -1
    property int maxMark: -1
    property bool interactive: false
    property color litColor: style.yellow
    property string accessibleName: ""
    property string valueText: value >= 0 ? value + "%" : "--"

    signal committed(int value)

    // Accessible value interface (read by QAccessibleQuickItem for sliders).
    readonly property int minimumValue: 0
    readonly property int maximumValue: 100
    readonly property int stepSize: 1

    readonly property real markerH: style.snap(style.unit * 0.45)
    readonly property real cellH: style.snap(style.unit * 0.8)
    readonly property real skew: style.snap(cellH * 0.3)
    readonly property real gapPx: style.hairline * 3
    readonly property real pitch: width / Math.max(1, cells)
    readonly property real cellW: Math.max(1, pitch - gapPx - skew)
    readonly property bool hasRange: minMark >= 0 && maxMark >= minMark
    readonly property real railY: markerH + cellH + style.hairline * 4
    readonly property real railH: style.hairline * 3
    readonly property real half: style.snap(style.unit * 0.3)

    readonly property bool known: value >= 0
    readonly property bool live: interactive && known && enabled
    property bool editing: false
    property int localValue: 0
    readonly property int shown: editing ? localValue : Math.max(0, value)
    readonly property int lit: shown <= 0 ? 0 : Math.max(1, Math.round(shown * cells / 100))
    /*! The lit colour while the value is known; a disabled or unknown bar goes muted. */
    readonly property color activeColor: known && enabled ? litColor : style.muted
    readonly property color hotColor: (editing || activeFocus) && live ? style.ink : activeColor

    readonly property string litPath: G.cells(0, Math.max(0, lit - 1), pitch, cellW, cellH, skew)
    readonly property string hotPath: lit > 0 ? G.cells(lit - 1, lit, pitch, cellW, cellH, skew) : ""
    readonly property string ghostPath: G.cells(lit, cells, pitch, cellW, cellH, skew)
    readonly property real targetX: (Math.min(cells - 1, Math.floor(Math.max(0, target) * cells / 100)) + 0.5) * pitch
    readonly property string targetPath: target >= 0
        ? G.triangleDown(targetX, 0, half, markerH - style.hairline) : ""
    readonly property string railPath: hasRange
        ? G.rect(width * minMark / 100, railY, Math.max(style.hairline, width * (maxMark - minMark) / 100), railH)
        : ""

    implicitWidth: 120
    implicitHeight: railY + railH
    activeFocusOnTab: live
    focus: false

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: valueText
    Accessible.focusable: live
    Accessible.onIncreaseAction: root.step(1)
    Accessible.onDecreaseAction: root.step(-1)

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

    function valueAt(x) {
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
            delta = -root.shown;
            break;
        case Qt.Key_End:
            delta = 100 - root.shown;
            break;
        default:
            return;
        }
        event.accepted = true;
        root.step(delta);
    }

    // Automatic range: a rail under the cells.
    Shape {
        anchors.fill: parent
        visible: root.hasRange
        ShapePath {
            strokeWidth: -1
            fillColor: root.style.muted
            PathSvg { path: root.railPath }
        }
    }

    // Ghost cells, then the lit ones; the last lit cell is the hot one.
    Shape {
        x: 0
        y: root.markerH
        width: root.width
        height: root.cellH
        ShapePath {
            strokeWidth: -1
            fillColor: root.style.ghost
            PathSvg { path: root.ghostPath }
        }
        ShapePath {
            strokeWidth: -1
            fillColor: root.activeColor
            PathSvg { path: root.litPath }
        }
        ShapePath {
            strokeWidth: -1
            fillColor: root.hotColor
            PathSvg { path: root.hotPath }
        }
    }

    // The automatic target, above the cells.
    Shape {
        anchors.fill: parent
        visible: root.target >= 0
        ShapePath {
            strokeWidth: -1
            fillColor: root.style.cyan
            PathSvg { path: root.targetPath }
        }
    }

    // Keyboard focus: a cyan frame around the whole bar.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -root.style.hairline * 2
        visible: root.activeFocus
        color: "transparent"
        border.width: root.style.hairline
        border.color: root.style.cyan
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        enabled: root.live
        cursorShape: Qt.PointingHandCursor
        onPressed: event => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.setLocal(root.valueAt(event.x));
        }
        onPositionChanged: event => {
            if (pressed) {
                root.setLocal(root.valueAt(event.x));
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }
}
