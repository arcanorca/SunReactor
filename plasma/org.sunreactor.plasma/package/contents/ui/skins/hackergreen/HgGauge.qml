/*
 * An ASCII gauge on the cell grid, [██████░░░░]: `cells` wide including the
 * brackets. Filled cells are rectangles (seamless at any scale), the free part
 * is the bundled "░" glyph. It is also the slider: press or drag sets the value
 * (`moved` while dragging, `released` at the end). `marker` draws a thin tick
 * (the target under a manual value); -1 draws none, value -1 is unknown.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../terminal"
import "../terminal/Term.js" as Term

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property int cells: 12
    /*! 0..100, or -1 when unknown (empty track). */
    property real value: -1
    property real marker: -1
    property bool interactive: true
    property color fillColor: theme.accent
    property color trackColor: theme.borderInactive
    property color bracketColor: theme.textMuted

    readonly property int inner: Math.max(1, cells - 2)
    readonly property int filled: value < 0 ? 0 : Math.round(Math.max(0, Math.min(100, value)) / 100 * inner)
    readonly property int markerCell: marker < 0 ? -1 : Math.round(Math.max(0, Math.min(100, marker)) / 100 * (inner - 1))

    signal moved(int value)
    signal released(int value)

    implicitWidth: tm.cx(cells)
    implicitHeight: tm.ch

    TermText {
        tm: root.tm
        value: "["
        color: root.bracketColor
    }

    Rectangle {
        x: root.tm.cx(1)
        width: root.tm.cx(root.filled)
        height: root.tm.ch
        color: root.fillColor
        antialiasing: false
    }

    TermText {
        tm: root.tm
        x: root.tm.cx(1 + root.filled)
        value: Term.repeat("░", root.inner - root.filled)
        color: root.trackColor
    }

    TermText {
        tm: root.tm
        visible: root.markerCell >= 0
        x: root.tm.cx(1 + root.markerCell)
        value: "|"
        strong: true
        color: root.theme.fg
    }

    TermText {
        tm: root.tm
        x: root.tm.cx(root.cells - 1)
        value: "]"
        color: root.bracketColor
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        enabled: root.interactive
        hoverEnabled: true
        cursorShape: root.interactive ? Qt.PointingHandCursor : Qt.ArrowCursor
        preventStealing: true

        function valueAt(mx) {
            const inside = mx - root.tm.cw;
            return Math.round(Math.max(0, Math.min(1, inside / Math.max(1, root.inner * root.tm.cw))) * 100);
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
