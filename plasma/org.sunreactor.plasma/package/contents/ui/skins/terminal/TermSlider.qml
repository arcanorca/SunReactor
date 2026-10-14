/*
 * A full-width, two-row brightness bar on the cell grid:
 *
 *     ▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇█░░░░░░░░░░░░░░░░░░░░░░░░   accent fill, bright head cell,
 *     ▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇█░░░░░░░░░░░░░░░░░░░░░░░░   muted track; the head is one cell
 *
 * The whole block (both rows, the full width) is the hit area: press or drag
 * sets the value (`moved` while dragging, `released` at the end). The wheel is
 * not used. Keyboard handling belongs to the module that owns the focus.
 * `marker` draws a tick at a second value (the curve's target while a manual
 * value overrides it). Drawn with rectangles, so it is seamless at any DPR.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    /*! Width in cells. */
    property int cells: 20
    /*! 0..100, or -1 when unknown (track only). */
    property real value: -1
    /*! 0..100 tick, or -1. */
    property real marker: -1
    property bool interactive: true
    property color fillColor: theme.accent
    property color headColor: theme.fg
    property color trackColor: theme.mix(theme.bg, theme.borderInactive, 0.32)

    signal moved(int value)
    signal released(int value)

    readonly property alias pressed: mouse.pressed
    /*! Cells the value runs over: the head sits on cell 0 at 0 % and the last at 100 %. */
    readonly property int span: Math.max(2, cells)
    readonly property int headCell: value >= 0 ? cellOf(value) : -1
    readonly property int markCell: marker >= 0 ? cellOf(marker) : -1

    function cellOf(pct) {
        return Math.round(Math.max(0, Math.min(100, pct)) / 100 * (span - 1));
    }
    function valueOf(cell) {
        return Math.round(Math.max(0, Math.min(span - 1, cell)) / (span - 1) * 100);
    }

    implicitWidth: tm.cx(cells)
    implicitHeight: tm.cy(2)
    width: implicitWidth
    height: implicitHeight

    // The track.
    Rectangle {
        x: 0
        y: 0
        width: root.tm.cx(root.span)
        height: root.tm.cy(2)
        color: root.trackColor
        antialiasing: false
    }

    // The filled part, up to the head.
    Rectangle {
        visible: root.headCell > 0
        x: 0
        y: 0
        width: root.tm.cx(root.headCell)
        height: root.tm.cy(2)
        color: root.fillColor
        antialiasing: false
    }

    // The target tick: a heavy line on the cell centre, full height.
    Rectangle {
        visible: root.markCell >= 0 && root.markCell !== root.headCell
        x: root.tm.lineX(root.markCell) - Math.floor(root.tm.lwDev / 2) / root.tm.dpr
        y: 0
        width: root.tm.lwHeavy
        height: root.tm.cy(2)
        color: root.theme.fg
        antialiasing: false
    }

    // The head: one cell, bright, so the handle is visible at 0 % and 100 % too.
    Rectangle {
        visible: root.value >= 0
        x: root.tm.cx(root.headCell)
        y: 0
        width: root.tm.cw
        height: root.tm.cy(2)
        color: root.interactive ? root.headColor : root.theme.textMuted
        antialiasing: false
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        enabled: root.interactive
        hoverEnabled: true
        cursorShape: root.interactive ? Qt.PointingHandCursor : Qt.ArrowCursor
        preventStealing: true

        function valueAt(mx) {
            return root.valueOf(Math.floor(mx / Math.max(1, root.tm.cw)));
        }

        onPressed: mouseEvent => root.moved(valueAt(mouseEvent.x))
        onPositionChanged: mouseEvent => {
            if (pressed) {
                root.moved(valueAt(mouseEvent.x));
            }
        }
        onReleased: mouseEvent => root.released(valueAt(mouseEvent.x))
    }
}
