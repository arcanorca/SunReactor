/*
 * The TUI's brightness range track (monitors.rs range_track): one absolute
 * 0..100 line, the allowed band between two handles drawn heavy, the applied
 * level as ●.
 *
 *     0 ──┃━━━━━━━●━━━━┃─────────────── 100
 *
 * The handle under the keyboard cursor becomes a block (█). Dragging moves
 * the nearer handle. Drawn with rectangles on the cell grid.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property int cells: 20
    property int minPct: 0
    property int maxPct: 100
    property int applied: -1
    /*! "", "min" or "max": the handle the keyboard cursor is on. */
    property string focusedHandle: ""

    signal moved(int minPct, int maxPct)
    signal released(int minPct, int maxPct)

    readonly property int track: Math.max(4, cells - 6)
    function pos(p) {
        return Math.round(Math.max(0, Math.min(100, p)) * (track - 1) / 100);
    }
    readonly property int minAt: pos(minPct)
    readonly property int maxAt: Math.max(minAt, pos(maxPct))

    width: tm.cx(cells)
    height: tm.ch

    TermText {
        tm: root.tm
        value: "0"
        color: root.theme.textMuted
    }
    TermText {
        tm: root.tm
        x: root.tm.cx(root.cells - 3)
        value: "100"
        color: root.theme.textMuted
    }

    Item {
        id: line

        x: root.tm.cx(2)
        width: root.tm.cx(root.track)
        height: root.tm.ch

        // ─ outside the band.
        Rectangle {
            y: root.tm.lineY(0)
            width: parent.width
            height: root.tm.lw
            color: root.theme.borderInactive
            antialiasing: false
        }
        // ━ the allowed band.
        Rectangle {
            x: root.tm.cx(root.minAt)
            y: root.tm.lineY(0) - Math.floor(root.tm.lwDev / 2) / root.tm.dpr
            width: root.tm.cx(root.maxAt - root.minAt)
            height: root.tm.lwHeavy
            color: root.theme.accent
            antialiasing: false
        }
        TermText {
            visible: root.applied >= 0 && root.pos(root.applied) !== root.minAt && root.pos(root.applied) !== root.maxAt
            tm: root.tm
            x: root.tm.cx(root.pos(root.applied))
            value: "●"
            strong: true
            color: root.theme.fg
        }
        // ┃ handles, █ under the cursor.
        Repeater {
            model: [root.minAt, root.maxAt]

            delegate: Rectangle {
                required property int modelData
                required property int index
                readonly property bool block: root.focusedHandle === (index === 0 ? "min" : "max")

                x: block ? root.tm.cx(modelData) : root.tm.lineX(modelData) - Math.floor(root.tm.lwDev / 2) / root.tm.dpr
                width: block ? root.tm.cw : root.tm.lwHeavy
                height: root.tm.ch
                color: block ? root.theme.accent : root.theme.fg
                antialiasing: false
            }
        }

        MouseArea {
            anchors.fill: parent
            anchors.topMargin: -2
            anchors.bottomMargin: -2
            preventStealing: true
            cursorShape: Qt.SizeHorCursor

            property string dragging: ""

            function pctAt(mx) {
                return Math.round(Math.max(0, Math.min(1, mx / Math.max(1, width - root.tm.cw))) * 100);
            }
            function apply(mx) {
                const p = pctAt(mx);
                if (dragging === "min") {
                    root.moved(Math.min(p, root.maxPct), root.maxPct);
                } else {
                    root.moved(root.minPct, Math.max(p, root.minPct));
                }
            }

            onPressed: mouseEvent => {
                const p = pctAt(mouseEvent.x);
                dragging = Math.abs(p - root.minPct) <= Math.abs(p - root.maxPct) && p <= root.maxPct ? "min" : "max";
                apply(mouseEvent.x);
            }
            onPositionChanged: mouseEvent => {
                if (pressed) {
                    apply(mouseEvent.x);
                }
            }
            onReleased: root.released(root.minPct, root.maxPct)
        }
    }
}
