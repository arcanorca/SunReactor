/*
 * A section of the hackergreen popup: a heading line, then its body.
 *
 *   == DISPLAYS =====================================  1 of 2
 *   mi-monitor [██████░░░░░░░░░░░░░░░░░░░] 39%
 *
 * `bracket` draws the heading as [ TITLE ] over a dashed rule instead. The rule
 * is text (= or -) on the cell grid, so it stays aligned at every scale. The
 * body is `bodyHeight` tall and holds the module's own children.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../instrument"
import "../terminal"
import "../terminal/Term.js" as Term

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property string title: ""
    property bool bracket: false
    /*! Right end of the heading: runs ({ t, c, b }), empty for none. */
    property var metaRuns: []
    /*! Height of the body, logical px. */
    property real bodyHeight: 0
    /*! Phosphor glow on the body: for bodies that only change with the daemon's data,
        never on hover (the glow is rendered from the body, so it repaints with it). */
    property bool glowing: false

    default property alias content: body.data

    readonly property int cells: tm.cols(width)
    readonly property string leftText: bracket ? "[ " + title + " ]" : "== " + title + " =="
    readonly property int leftCells: tm.len(leftText)
    readonly property int metaCells: meta.usedCells
    readonly property int ruleCells: Math.max(0, cells - leftCells - 1 - (metaCells > 0 ? metaCells + 1 : 0))

    implicitHeight: tm.cy(1) + bodyHeight

    TermRuns {
        id: head

        tm: root.tm
        cells: root.cells
        defaultColor: root.theme.fg
        runs: root.bracket
            ? [{ t: "[ ", c: root.theme.textMuted }, { t: root.title, c: root.theme.fg, b: true },
               { t: " ]", c: root.theme.textMuted }]
            : [{ t: "== ", c: root.theme.borderInactive }, { t: root.title, c: root.theme.fg, b: true },
               { t: " ==", c: root.theme.borderInactive }]
    }

    TermText {
        tm: root.tm
        x: root.tm.cx(root.leftCells + 1)
        value: Term.repeat(root.bracket ? "-" : "=", root.ruleCells)
        color: root.theme.borderInactive
    }

    TermRuns {
        id: meta

        tm: root.tm
        x: root.tm.cx(root.cells - root.metaCells)
        cells: root.cells
        runs: root.metaRuns
        defaultColor: root.theme.textMuted
    }

    Glow {
        anchors.fill: body
        source: body
        strength: root.glowing ? root.theme.glow : 0
    }

    Item {
        id: body

        y: root.tm.cy(1)
        width: root.width
        height: root.bodyHeight
    }
}
