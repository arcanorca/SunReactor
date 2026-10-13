/*
 * A horizontal bar gauge on the cell grid, `▕█████░░░░▏`: `cells` wide
 * including the two end caps, filled in eighths of a cell like the block
 * elements ▏▎▍▌▋▊▉█. Drawn with rectangles, so it is seamless at any DPR.
 *
 * It is also the slider: press or drag sets the value (`moved` while
 * dragging, `released` at the end); keyboard handling belongs to the module
 * that owns the focus. `marker` draws a thin tick (e.g. the target) and
 * `range` an allowed band under the fill.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property int cells: 10
    /*! 0..100, or -1 when unknown (track only). */
    property real value: -1
    property real marker: -1
    property bool interactive: true
    property bool focused: false
    property color fillColor: theme.accent
    property color trackColor: theme.mix(theme.bg, theme.borderInactive, 0.55)
    property color capColor: focused ? theme.accent : theme.borderInactive

    readonly property alias pressed: mouse.pressed
    readonly property alias hovered: mouse.containsMouse

    signal moved(int value)
    signal released(int value)

    readonly property int innerCells: Math.max(1, cells - 2)
    readonly property int innerDev: innerCells * tm.cwDev
    /*! Block height: a full block minus a sliver, so stacked rows breathe. */
    readonly property int barDev: Math.max(2, tm.chDev - 2 * Math.round(tm.chDev * 0.2))
    readonly property int barTopDev: Math.floor((tm.chDev - barDev) / 2)
    readonly property int capDev: Math.max(tm.lwDev, Math.round(tm.cwDev / 8))

    function devOf(pct) {
        // Eighths of a cell, the resolution of the block elements.
        const eighths = Math.round(Math.max(0, Math.min(100, pct)) / 100 * innerCells * 8);
        return Math.round(eighths * tm.cwDev / 8);
    }

    implicitWidth: tm.cx(cells)
    implicitHeight: tm.ch
    width: implicitWidth
    height: implicitHeight

    // ▕ and ▏ caps.
    Rectangle {
        x: (root.tm.cwDev - root.capDev) / root.tm.dpr
        y: 0
        width: root.capDev / root.tm.dpr
        height: root.tm.ch
        color: root.capColor
        antialiasing: false
    }
    Rectangle {
        x: (root.tm.cwDev + root.innerDev) / root.tm.dpr
        y: 0
        width: root.capDev / root.tm.dpr
        height: root.tm.ch
        color: root.capColor
        antialiasing: false
    }

    // ░ track.
    Rectangle {
        x: root.tm.cw
        y: root.barTopDev / root.tm.dpr
        width: root.innerDev / root.tm.dpr
        height: root.barDev / root.tm.dpr
        color: root.trackColor
        antialiasing: false
    }

    // █ fill.
    Rectangle {
        visible: root.value >= 0
        x: root.tm.cw
        y: root.barTopDev / root.tm.dpr
        width: root.devOf(root.value) / root.tm.dpr
        height: root.barDev / root.tm.dpr
        color: root.fillColor
        antialiasing: false
    }

    // Marker tick, full cell height so it reads past the fill.
    Rectangle {
        visible: root.marker >= 0
        x: (root.tm.cwDev + Math.min(root.innerDev - root.tm.lwDev * 2, root.devOf(root.marker))) / root.tm.dpr
        y: 0
        width: root.tm.lwHeavy
        height: root.tm.ch
        color: root.theme.fg
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
            const inner = mx - root.tm.cw;
            return Math.round(Math.max(0, Math.min(1, inner / Math.max(1, root.innerDev / root.tm.dpr))) * 100);
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
