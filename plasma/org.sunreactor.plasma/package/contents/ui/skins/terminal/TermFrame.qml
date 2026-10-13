/*
 * A titled box, the TUI's kit::panel + with_meta:
 *
 *     ╭ Title ──────────────────── meta ╮
 *     │ content                         │
 *     ╰ key hint · key hint ────────────╯
 *
 * The frame takes the whole cells that fit its width (centred, the remainder
 * under one cell goes to the margins) and `contentRows` + 2 rows. Content goes
 * in the body: 2 cells in (border + one cell of padding, ratatui's
 * Padding::horizontal(1)), one row down, `innerCols` cells wide.
 *
 * Lines are rectangles on the cell grid's centre lines, never glyphs, so
 * corners and edges meet without gaps at any device pixel ratio. Rounded
 * corners (glyph_style "rounded") are antialiased quarter arcs of half a cell;
 * "pixel" gives square corners. theme.border: none = only a heading rule
 * (kit::heading), line = single, double = two lines (═), bevel / inset =
 * heavy (━). Focus colours the border and title like a focused TUI panel.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property string title: ""
    /*! Right side of the top border: a string, or runs (TermRuns) when set. */
    property string rightTitle: ""
    property var rightRuns: []
    /*! Bottom border runs, e.g. key hints. */
    property var bottomRuns: []
    property bool focused: false
    property int contentRows: 1
    property string style: theme.border
    /*! Name reported to the screenshot harness (gap check), "" = none. */
    property string regionName: ""

    default property alias content: body.data
    readonly property alias body: body

    readonly property bool boxed: style !== "none"
    readonly property bool double: style === "double"
    readonly property bool heavy: style === "bevel" || style === "inset"
    readonly property int cols: tm.cols(width)
    /*! Cells available to content. */
    readonly property int innerCols: boxed ? Math.max(0, cols - 4) : cols
    readonly property int rows: contentRows + 2
    readonly property color lineColor: focused ? theme.borderActive : theme.borderInactive

    implicitHeight: tm.cy(rows)

    // --- geometry in device px (integers) ---------------------------------
    readonly property int offDev: Math.floor((Math.round(width * tm.dpr) - cols * tm.cwDev) / 2)
    readonly property int strokeDev: heavy ? tm.lwDev * 2 : tm.lwDev
    readonly property int leftDev: offDev + Math.floor((tm.cwDev - strokeDev) / 2)
    readonly property int rightDev: offDev + (cols - 1) * tm.cwDev + Math.floor((tm.cwDev - strokeDev) / 2)
    readonly property int topDev: Math.floor((tm.chDev - strokeDev) / 2)
    readonly property int bottomDev: (rows - 1) * tm.chDev + Math.floor((tm.chDev - strokeDev) / 2)
    readonly property int radiusDev: boxed && tm.rounded && !double ? Math.floor(Math.min(tm.cwDev, tm.chDev) / 2) : 0

    readonly property int titleCells: title.length > 0 ? tm.len(titleText.text) : 0
    readonly property int rightCells: rightRuns.length > 0 ? rightLine.usedCells
                                                            : (rightTitle.length > 0 ? tm.len(rightText.text) : 0)
    readonly property int bottomCells: bottomRuns.length > 0 ? bottomLine.usedCells : 0

    /*! Top/bottom border gaps [fromDev, toDev) where titles sit. */
    function gaps(leftCells, rightCellsCount) {
        const out = [];
        if (leftCells > 0) {
            out.push([offDev + tm.cwDev, offDev + (2 + leftCells + 1) * tm.cwDev]);
        }
        if (rightCellsCount > 0) {
            out.push([offDev + (cols - 3 - rightCellsCount) * tm.cwDev, offDev + (cols - 1) * tm.cwDev]);
        }
        return out;
    }

    /*! A horizontal run [x0, x1) at y, minus gaps, as device rects. */
    function hRun(x0, x1, y, h, cut) {
        const out = [];
        let start = x0;
        const sorted = cut.slice().sort((a, b) => a[0] - b[0]);
        for (const g of sorted) {
            if (g[1] <= start || g[0] >= x1) {
                continue;
            }
            if (g[0] > start) {
                out.push([start, y, g[0] - start, h]);
            }
            start = Math.max(start, g[1]);
        }
        if (x1 > start) {
            out.push([start, y, x1 - start, h]);
        }
        return out;
    }

    readonly property var segments: {
        if (cols < 4) {
            return [];
        }
        const s = strokeDev;
        const top = gaps(titleCells, rightCells);
        const bottom = gaps(bottomCells, 0);
        if (!boxed) {
            // Heading rule only: "Title ─────── meta" across the first row.
            const rule = [[offDev, offDev + (titleCells > 0 ? titleCells + 1 : 0) * tm.cwDev]];
            if (rightCells > 0) {
                rule.push([offDev + (cols - rightCells - 1) * tm.cwDev, offDev + cols * tm.cwDev]);
            }
            return hRun(offDev, offDev + cols * tm.cwDev, topDev, s, rule);
        }
        const r = radiusDev;
        // Each ring: its offset from the centre line (double = two rings).
        const rings = double ? [-s, s] : [0];
        let out = [];
        for (const d of rings) {
            const l = leftDev - d, rt = rightDev + d, t = topDev - d, b = bottomDev + d;
            out = out.concat(hRun(l + r, rt + s - r, t, s, top));
            out = out.concat(hRun(l + r, rt + s - r, b, s, bottom));
            out.push([l, t + r, s, b + s - t - 2 * r]);
            out.push([rt, t + r, s, b + s - t - 2 * r]);
        }
        return out;
    }

    // Box lines.
    Repeater {
        model: root.segments

        delegate: Rectangle {
            required property var modelData

            x: modelData[0] / root.tm.dpr
            y: modelData[1] / root.tm.dpr
            width: modelData[2] / root.tm.dpr
            height: modelData[3] / root.tm.dpr
            color: root.lineColor
            antialiasing: false
        }
    }

    // Rounded corners: quarter arcs, clipped from one rounded rectangle each.
    Repeater {
        model: root.radiusDev > 0 && root.cols >= 4 ? 4 : 0

        delegate: Item {
            id: corner

            required property int index
            readonly property bool atRight: index === 1 || index === 2
            readonly property bool atBottom: index >= 2
            readonly property int r: root.radiusDev

            x: (atRight ? root.rightDev + root.strokeDev - r : root.leftDev) / root.tm.dpr
            y: (atBottom ? root.bottomDev + root.strokeDev - r : root.topDev) / root.tm.dpr
            width: r / root.tm.dpr
            height: r / root.tm.dpr
            clip: true

            Rectangle {
                x: corner.atRight ? -corner.width : 0
                y: corner.atBottom ? -corner.height : 0
                width: corner.width * 2
                height: corner.height * 2
                radius: corner.width
                color: "transparent"
                border.width: root.strokeDev / root.tm.dpr
                border.color: root.lineColor
                antialiasing: true
            }
        }
    }

    TermText {
        id: titleText

        tm: root.tm
        visible: root.title.length > 0
        x: (root.offDev + (root.boxed ? 2 : 0) * root.tm.cwDev) / root.tm.dpr
        value: root.title
        cells: Math.max(0, root.cols - 6 - root.rightCells)
        strong: true
        color: root.focused ? root.theme.accent : root.theme.fg
    }

    TermText {
        id: rightText

        tm: root.tm
        visible: root.rightRuns.length === 0 && root.rightTitle.length > 0
        x: (root.offDev + (root.cols - (root.boxed ? 2 : 0)) * root.tm.cwDev) / root.tm.dpr - width
        value: root.rightTitle
        cells: Math.max(0, Math.floor(root.cols / 2) - 2)
        color: root.theme.textMuted
    }

    TermRuns {
        id: rightLine

        tm: root.tm
        visible: root.rightRuns.length > 0
        x: (root.offDev + (root.cols - (root.boxed ? 2 : 0)) * root.tm.cwDev) / root.tm.dpr - width
        runs: root.rightRuns
        cells: Math.max(0, Math.floor(root.cols / 2) - 2)
        defaultColor: root.theme.textMuted
    }

    TermRuns {
        id: bottomLine

        tm: root.tm
        visible: root.boxed && root.bottomRuns.length > 0
        x: (root.offDev + 2 * root.tm.cwDev) / root.tm.dpr
        y: root.tm.cy(root.rows - 1)
        runs: root.bottomRuns
        cells: Math.max(0, root.cols - 6)
        defaultColor: root.theme.textMuted
    }

    Item {
        id: body

        x: (root.offDev + (root.boxed ? 2 : 0) * root.tm.cwDev) / root.tm.dpr
        y: root.tm.cy(1)
        width: root.tm.cx(root.innerCols)
        height: root.tm.cy(root.contentRows)
    }

    // The harness checks that every edge is unbroken (tools/term_gaps.py).
    function report() {
        if (!root.theme.grid.harness || root.regionName.length === 0 || root.cols < 4 || !root.visible) {
            return;
        }
        const origin = root.mapToItem(null, 0, 0);
        console.info("SUNREACTOR_TERM_FRAME " + JSON.stringify({
            name: root.regionName, dpr: root.tm.dpr,
            ox: origin.x * root.tm.dpr, oy: origin.y * root.tm.dpr,
            left: root.leftDev, right: root.rightDev, top: root.topDev, bottom: root.bottomDev,
            stroke: root.strokeDev, radius: root.radiusDev, double: root.double, boxed: root.boxed,
            topGaps: root.gaps(root.titleCells, root.rightCells), bottomGaps: root.gaps(root.bottomCells, 0),
            color: String(root.lineColor),
        }));
    }
    Timer {
        id: reportTimer
        interval: 1500
        onTriggered: root.report()
    }
    onSegmentsChanged: if (root.theme.grid.harness) reportTimer.restart()
    Component.onCompleted: if (root.theme.grid.harness) reportTimer.restart()
}
