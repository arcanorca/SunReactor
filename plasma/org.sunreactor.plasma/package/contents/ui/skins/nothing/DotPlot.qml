/*
 * The day curve, Nothing: a sparse dot plot. Every column carries one lit dot at
 * the curve's height; a dotted baseline runs under the day; the 'now' column is a
 * thin dotted line with the one red dot on the curve. For a single display the
 * weather cost is a second row of tiny dim dots at the clear-sky height, above
 * the curve. Nothing is filled: the plot is the curve, drawn as dots.
 *
 *   . . . . . . . . .      clear-sky row: tiny dim dots, the weather cost
 *       o o o ●             curve: one lit dot per column, red at now
 *   . . . . . . . . .      dotted baseline
 *
 * Each layer is one DotField written with setCell when its inputs change, never
 * per frame, so an idle plot costs nothing.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"

Item {
    id: root

    required property NothingStyle style
    /*! One entry per plotted display: { samples: [{ t, pct, pctNoWeather }] }; the first leads. */
    property var curves: []
    /*! The day, in epoch seconds. */
    property real t0: 0
    property real t1: 1
    /*! Epoch seconds of 'now'; -1 when the day does not contain it. */
    property real nowT: -1
    /*! Dot rows from the top (0%) to the baseline (0 target). */
    property int rows: 12

    readonly property real pitch: style.pitch
    readonly property int cols: Math.max(12, Math.floor(width / pitch))
    readonly property real span: Math.max(1, t1 - t0)
    readonly property int nowCol: nowT >= t0 && nowT <= t1
        ? Math.min(cols - 1, Math.floor((nowT - t0) / span * cols)) : -1
    readonly property int nowRow: {
        if (nowCol < 0 || curves.length === 0) {
            return -1;
        }
        const v = style.valueAt(curves[0].samples, nowT, "pct");
        return v < 0 ? -1 : rowFor(v);
    }
    /*! Everything the layers paint from: one change repaints each layer once. */
    readonly property var paintKey: [t0, t1, nowCol, nowRow, cols, rows, curves]

    implicitHeight: rows * pitch

    /*! Row of a percentage: 100 at the top row, 0 on the baseline. */
    function rowFor(pct) {
        const f = Math.max(0, Math.min(100, pct)) / 100;
        return Math.round((1 - f) * (rows - 1));
    }

    /*! Time at the middle of a column. */
    function timeOf(col) {
        return t0 + (col + 0.5) * span / cols;
    }

    onPaintKeyChanged: {
        paintGuides();
        paintCost();
    }

    Component.onCompleted: {
        paintGuides();
        paintCost();
    }

    // --- the guides: the dotted baseline and the dotted 'now' column --------------------

    DotField {
        id: guides

        cols: root.cols
        rows: root.rows
        pitch: root.pitch
        dimColor: "transparent"
        ghostColor: root.style.dotGhost
        onGridChanged: root.paintGuides()
    }

    function paintGuides() {
        guides.fill(DotField.Off);
        for (let c = 0; c < guides.cols; c += 2) {
            guides.setCell(c, guides.rows - 1, DotField.Ghost);
        }
        if (nowCol >= 0) {
            for (let r = 0; r < guides.rows; ++r) {
                guides.setCell(nowCol, r, DotField.Ghost);
            }
        }
    }

    // --- the weather cost: tiny dim dots at the clear-sky height, above the curve ------

    DotField {
        id: cost

        cols: root.cols
        rows: root.rows
        pitch: root.pitch
        dotScale: 0.4
        dimColor: "transparent"
        litColor: root.style.theme.alpha(root.style.ink, 0.5)
        onGridChanged: root.paintCost()
    }

    function paintCost() {
        cost.fill(DotField.Off);
        if (curves.length !== 1) {
            return;
        }
        const samples = curves[0].samples;
        for (let c = 0; c < cost.cols; ++c) {
            const t = timeOf(c);
            const clear = style.valueAt(samples, t, "pctNoWeather");
            const live = style.valueAt(samples, t, "pct");
            if (clear < 0 || live < 0) {
                continue;
            }
            const top = rowFor(clear);
            if (top < rowFor(live)) {
                cost.setCell(c, top, DotField.Lit);
            }
        }
    }

    // --- the curves: one lit dot per column; the lead is white, the others grey ---------

    Repeater {
        model: root.curves

        delegate: DotField {
            id: trace

            required property var modelData
            required property int index

            /*! The first display owns the red now dot. */
            readonly property bool lead: index === 0

            cols: root.cols
            rows: root.rows
            pitch: root.pitch
            dimColor: "transparent"
            litColor: lead ? root.style.ink : root.style.muted
            accentColor: root.style.accent
            onGridChanged: paint()

            function paint() {
                fill(DotField.Off);
                const samples = modelData.samples;
                for (let c = 0; c < root.cols; ++c) {
                    const v = root.style.valueAt(samples, root.timeOf(c), "pct");
                    if (v < 0) {
                        continue;
                    }
                    const isNow = lead && c === root.nowCol;
                    setCell(c, root.rowFor(v), isNow ? DotField.Accent : DotField.Lit);
                }
            }

            readonly property var paintInputs: root.paintKey

            Component.onCompleted: paint()
            onPaintInputsChanged: paint()
            onModelDataChanged: paint()
        }
    }
}
