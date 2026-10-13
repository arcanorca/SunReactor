/*
 * A line of styled spans, like a ratatui `Line` of `Span`s: each run is
 * { t: text, c: colour, b: bold, inv: inverse background colour, k: key for
 * span() }. Runs sit
 * on consecutive cells; `cells` cuts the line (the last visible run ends in
 * "…"). The line is `usedCells` wide, so callers can right-align or centre it.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property TermMetrics tm
    property var runs: []
    /*! Maximum width in cells, -1 = unlimited. */
    property int cells: -1
    property color defaultColor: "white"

    /*! The runs after cutting: [{t, c, b, inv, at}] with `at` = start cell. */
    readonly property var placed: {
        const out = [];
        let at = 0;
        for (const run of runs) {
            if (!run || !run.t) {
                continue;
            }
            let text = String(run.t);
            const room = cells >= 0 ? cells - at : 1e9;
            if (room <= 0) {
                break;
            }
            if (tm.len(text) > room) {
                text = tm.fit(text, room);
            }
            out.push({ t: text, c: run.c !== undefined ? run.c : defaultColor, b: run.b === true,
                       inv: run.inv !== undefined ? run.inv : "", at: at, k: run.k || "" });
            at += tm.len(text);
        }
        return out;
    }
    readonly property int usedCells: placed.length > 0
        ? placed[placed.length - 1].at + tm.len(placed[placed.length - 1].t) : 0

    /*! [startCell, cells] of the run tagged `k` (for hit areas), or null. */
    function span(key) {
        const run = placed.find(r => r.k === key);
        return run ? [run.at, tm.len(run.t)] : null;
    }

    width: tm.cx(usedCells)
    height: tm.ch

    Repeater {
        model: root.placed

        delegate: TermText {
            required property var modelData

            tm: root.tm
            x: root.tm.cx(modelData.at)
            value: modelData.t
            color: modelData.c
            strong: modelData.b
            inverse: modelData.inv !== ""
            inverseColor: modelData.inv !== "" ? modelData.inv : "transparent"
        }
    }
}
