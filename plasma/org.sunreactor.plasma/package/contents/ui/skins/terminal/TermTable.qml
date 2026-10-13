/*
 * An aligned table on the cell grid, the TUI's schedule table:
 *
 *        Event        Offset  Time     Target
 *     ─────────────────────────────────────────
 *     ●  Rise 50%          —  08:54       34%
 *
 * `columns`: [{ title, cells, right }]; a column with cells 0 takes the room
 * that is left. Columns are separated by two cells. `rows`: [{ values: [...],
 * colors: [...] (optional per cell), current, strong, muted }]. A three-cell
 * gutter carries the cursor (`selected`, TermCursor) and the "current" dot.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "Term.js" as Term

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property var columns: []
    property var rows: []
    property int cells: tm.cols(width)
    /*! Index of the cursor row, -1 = none. */
    property int selected: -1
    property bool focused: false
    property color currentColor: theme.secondaryAccent

    signal activated(int index)

    readonly property int gutter: 3
    readonly property var widths: {
        let fixed = 0;
        let flex = -1;
        for (let i = 0; i < columns.length; ++i) {
            if (columns[i].cells > 0) {
                fixed += columns[i].cells;
            } else {
                flex = i;
            }
        }
        const gaps = Math.max(0, columns.length - 1) * 2;
        const rest = Math.max(4, cells - gutter - gaps - fixed);
        return columns.map((c, i) => i === flex ? rest : c.cells);
    }
    readonly property var starts: {
        const out = [];
        let at = gutter;
        for (const w of widths) {
            out.push(at);
            at += w + 2;
        }
        return out;
    }
    function cellText(value, column) {
        const w = widths[column];
        return columns[column].right ? Term.padStart(value, w) : Term.padEnd(value, w);
    }

    implicitHeight: tm.cy(2 + rows.length)

    // Header.
    Repeater {
        model: root.columns

        delegate: TermText {
            required property var modelData
            required property int index

            tm: root.tm
            x: root.tm.cx(root.starts[index])
            value: root.cellText(modelData.title, index)
            color: root.theme.textMuted
        }
    }

    // Rule under the header.
    Rectangle {
        x: 0
        y: root.tm.lineY(1)
        width: root.tm.cx(root.cells)
        height: root.tm.lw
        color: root.theme.borderInactive
        antialiasing: false
    }

    Repeater {
        model: root.rows

        delegate: Item {
            id: row

            required property var modelData
            required property int index
            readonly property bool isSelected: index === root.selected

            y: root.tm.cy(2 + index)
            width: root.tm.cx(root.cells)
            height: root.tm.ch

            TermCursor {
                id: cursor

                tm: root.tm
                theme: root.theme
                cells: root.cells
                current: row.isSelected
                active: root.focused
            }

            TermText {
                tm: root.tm
                x: root.tm.cx(1)
                visible: row.modelData.current === true
                value: "●"
                color: cursor.current && cursor.active ? cursor.textColor : root.currentColor
            }

            Repeater {
                model: row.modelData.values

                delegate: TermText {
                    required property var modelData
                    required property int index

                    tm: root.tm
                    x: root.tm.cx(root.starts[index])
                    value: root.cellText(String(modelData), index)
                    strong: row.modelData.strong === true || (row.isSelected && root.focused)
                    color: {
                        if (cursor.current && cursor.active) {
                            return cursor.textColor;
                        }
                        const given = row.modelData.colors ? row.modelData.colors[index] : undefined;
                        if (given !== undefined && given !== null && given !== "") {
                            return given;
                        }
                        return row.modelData.muted === true ? root.theme.textMuted : root.theme.fg;
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                onClicked: root.activated(row.index)
            }
        }
    }
}
