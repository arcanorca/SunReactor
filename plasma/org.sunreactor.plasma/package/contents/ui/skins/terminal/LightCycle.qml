/*
 * Terminal skin of the light cycle (hero) module: the TUI's "Today's light
 * cycle" panel (src/tui/ui/light_cycle.rs) on the daemon's day_curve.
 *
 *     1 Mi   2 Len   3 All                       <- tabs with ≥2 displays
 *     ╭ Today's light cycle ────── Tue Oct 13 ╮
 *     │ ▼ Weather −5%      ━ Brightness target 39% │
 *     │100% ┈┈┈┈┈┈┈┈┈┈▄▆█▆▄┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈ │   area, eighth-cell tops
 *     │ 50% ┈┈┈┈┈┈▃▅██●██▅▃┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈ │   shaded by row
 *     │  0% ▁▁▁▁▁▁▁▁▄████│████████▄▁▁▁▁▁▁▁▁▁▁▁▁▁ │
 *     │     00:00   06:00   12:00   18:00   24:00 │   hour axis; its background
 *     ╰───────────────────────────────────────────╯   is the sky light (night→day)
 *
 * Absolute 0..100 % scale, three plot rows (six when the theme is comfortable).
 * The part of the area between pct and pct_no_weather (what the weather costs)
 * is tinted with `warning`. Hover scrubs (time -> target in the legend); it
 * never sets brightness. Blocks and lines are rectangles on the cell grid, text
 * is on the same grid.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Term.js" as Term

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves`; curves.length = "All". */
    property int selected: 0
    readonly property bool showAll: selected >= curves.length && curves.length > 1
    readonly property var current: curves.length > 0 ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null

    readonly property real t0: available ? curves[0].samples[0].t : 0
    readonly property real t1: available ? curves[0].samples[curves[0].samples.length - 1].t : 1
    readonly property real now: client.nowEpochS
    readonly property bool use12h: /a/i.test(Qt.locale().timeFormat(Locale.ShortFormat))

    TermMetrics {
        id: term
        theme: root.theme
    }

    function valueAt(samples, t, field) {
        if (!samples || samples.length === 0 || t < samples[0].t || t > samples[samples.length - 1].t) {
            return -1;
        }
        for (let i = 1; i < samples.length; ++i) {
            const b = samples[i];
            if (b.t >= t) {
                const a = samples[i - 1];
                const f = b.t === a.t ? 0 : (t - a.t) / (b.t - a.t);
                return a[field] + (b[field] - a[field]) * f;
            }
        }
        return samples[samples.length - 1][field];
    }

    // --- layout in cells -------------------------------------------------------
    readonly property bool tabs: curves.length > 1
    readonly property int chartRows: term.comfortable ? 6 : 3
    readonly property int gutter: frame.innerCols >= 34 ? 5 : 0
    readonly property int plotCols: Math.max(8, frame.innerCols - gutter - (gutter > 0 ? 1 : 0))
    readonly property int plotHDev: chartRows * term.chDev
    /*! Gutter labels and grid lines: the quarters when there are rows for them. */
    readonly property var labelPcts: chartRows >= 6 ? [100, 75, 50, 25, 0] : [100, 50, 0]
    readonly property var gridPcts: chartRows >= 6 ? [75, 50, 25] : [50]

    function colOf(t) {
        return Math.round(Math.max(0, Math.min(1, (t - t0) / Math.max(1, t1 - t0))) * (plotCols - 1));
    }
    function rowOf(pct) {
        // The cell the value falls in, on the same scale as fillDev (the whole plot
        // height is 0..100), so 75% and 25% sit on even rows.
        return Math.min(chartRows - 1, Math.floor(chartRows * (1 - Math.max(0, Math.min(100, pct)) / 100)));
    }
    /*! Height of a fill in device px, in eighths of a cell (▁..█), at least ▁. */
    function fillDev(pct) {
        const eighths = Math.max(1, Math.round(Math.max(0, Math.min(100, pct)) / 100 * chartRows * 8));
        return Math.round(eighths * term.chDev / 8);
    }

    /*! Per plot column: target, target without weather, fill heights, top row. */
    readonly property var columns: {
        const out = [];
        if (!available || !current) {
            return out;
        }
        for (let c = 0; c < plotCols; ++c) {
            const t = t0 + (c + 0.5) / plotCols * (t1 - t0);
            const v = valueAt(current.samples, t, "pct");
            const nw = valueAt(current.samples, t, "pctNoWeather");
            const fill = v >= 0 ? fillDev(v) : 0;
            const fillNw = nw >= 0 ? Math.max(fill, fillDev(nw)) : fill;
            // Row the area (with the weather share) reaches into, as in the TUI.
            const topRow = chartRows - Math.ceil(fillNw / term.chDev);
            out.push({ v: v, fill: fill, fillNw: fillNw, topRow: topRow });
        }
        return out;
    }

    readonly property int nowCol: colOf(now)
    readonly property bool nowInside: available && now >= t0 && now <= t1
    readonly property real targetNow: current && nowInside ? valueAt(current.samples, now, "pct") : -1
    readonly property real costNow: current && nowInside
        ? valueAt(current.samples, now, "pctNoWeather") - valueAt(current.samples, now, "pct") : 0

    readonly property var bounds: Term.phaseBounds(curve ? curve.sun : null, t0, t1)

    function shortTime(t) {
        const d = new Date(t * 1000);
        if (!use12h) {
            return Qt.formatTime(d, "HH:mm");
        }
        const h = d.getHours() % 12 === 0 ? 12 : d.getHours() % 12;
        return h + (d.getHours() < 12 ? "am" : "pm");
    }

    implicitHeight: (tabs ? term.cy(2) : 0) + frame.implicitHeight

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")

    TermTabs {
        id: switcher

        tm: term
        theme: root.theme
        visible: root.tabs
        x: term.inset(root.width)
        width: term.cx(term.cols(root.width))
        labels: root.curves.map(c => Term.shortName(c.monitorId))
            .concat([i18ndc(root.domain, "Show the curves of every display", "All")])
        current: Math.min(root.selected, root.curves.length)
        onActivated: index => root.selected = index
    }

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        y: root.tabs ? term.cy(2) : 0
        width: root.width
        title: i18nd(root.domain, "Today's light cycle")
        rightTitle: root.available ? Qt.formatDate(new Date(root.now * 1000), "ddd MMM d") : ""
        contentRows: 2 + root.chartRows
        focused: switcher.activeFocus
        regionName: "curve"

        // Legend: target now, or under the pointer; what the weather costs on the left.
        TermRuns {
            tm: term
            visible: !root.showAll && root.costNow >= 1 && !scrub.containsMouse
            runs: [{ t: "▼ ", c: root.theme.warning },
                   { t: i18ndc(root.domain, "Brightness the weather is taking away right now", "Weather −%1%",
                               Math.round(root.costNow)), c: root.theme.textMuted }]
            cells: Math.max(0, frame.innerCols - legend.usedCells - 2)
        }

        TermRuns {
            id: legend

            tm: term
            x: term.cx(frame.innerCols - usedCells)
            cells: frame.innerCols
            runs: {
                if (root.showAll) {
                    const colors = [root.theme.accent, root.theme.secondaryAccent, root.theme.textMuted];
                    let out = [];
                    root.curves.forEach((c, i) => {
                        out.push({ t: (i > 0 ? "  " : "") + "━ ", c: colors[Math.min(i, 2)] });
                        out.push({ t: Term.shortName(c.monitorId), c: root.theme.textMuted });
                    });
                    return out;
                }
                if (scrub.containsMouse && root.current) {
                    const t = root.t0 + (scrub.column + 0.5) / root.plotCols * (root.t1 - root.t0);
                    const pct = root.valueAt(root.current.samples, t, "pct");
                    return [{ t: "━ ", c: root.theme.accent },
                            { t: root.shortTime(t) + " ", c: root.theme.textMuted },
                            { t: i18ndc(root.domain, "Brightness the curve targets at the time under the pointer",
                                        "target"), c: root.theme.textMuted },
                            { t: " " + Math.round(pct) + "%", c: root.theme.fg, b: true }];
                }
                const out = [{ t: "━ ", c: root.theme.accent },
                             { t: i18nd(root.domain, "Brightness target"), c: root.theme.textMuted }];
                if (root.targetNow >= 0) {
                    out.push({ t: " " + Math.round(root.targetNow) + "%", c: root.theme.fg, b: true });
                }
                return out;
            }
        }

        // Percent labels in the gutter.
        Repeater {
            model: root.gutter > 0 ? root.labelPcts : []

            delegate: TermText {
                required property int modelData
                required property int index

                readonly property int row: root.rowOf(modelData)

                tm: term
                // A label whose row a higher label already took is skipped.
                visible: index === 0 || root.rowOf(root.labelPcts[index - 1]) !== row
                y: term.cy(1 + row)
                value: Term.padStart(modelData + "%", 4)
                color: root.theme.textMuted
            }
        }

        // The plot.
        Item {
            id: plot

            x: term.cx(root.gutter > 0 ? root.gutter : 0)
            y: term.cy(1)
            width: term.cx(root.plotCols)
            height: root.plotHDev / term.dpr
            visible: root.available

            // Row bands: the area's colour steps from the accent at the top
            // to the background at the bottom, one shade per row.
            Repeater {
                model: root.showAll ? 0 : root.chartRows

                delegate: Rectangle {
                    required property int index

                    y: term.cy(index)
                    width: plot.width
                    height: term.ch
                    antialiasing: false
                    color: {
                        const top = root.theme.mix(root.theme.accent, root.theme.bg, 0.05);
                        const bottom = root.theme.mix(root.theme.accent, root.theme.bg, 0.78);
                        return root.theme.mix(top, bottom, index / Math.max(1, root.chartRows - 1));
                    }
                }
            }

            // Per column: background down to the top of the area, then the
            // weather share in the warning tint.
            Repeater {
                model: root.showAll ? 0 : root.columns.length

                delegate: Item {
                    id: column

                    required property int index
                    readonly property var colData: root.columns[index]

                    x: term.cx(index)
                    width: term.cw
                    height: plot.height

                    Rectangle {
                        width: parent.width
                        height: (root.plotHDev - column.colData.fill) / term.dpr
                        color: root.theme.bg
                        antialiasing: false
                    }
                    Rectangle {
                        visible: column.colData.fillNw > column.colData.fill
                        y: (root.plotHDev - column.colData.fillNw) / term.dpr
                        width: parent.width
                        height: (column.colData.fillNw - column.colData.fill) / term.dpr
                        color: root.theme.mix(root.theme.warning, root.theme.bg, 0.6)
                        antialiasing: false
                    }
                }
            }

            // "All": each display's curve as a heavy line at its top.
            Repeater {
                model: root.showAll ? root.curves : []

                delegate: Item {
                    id: allCurve

                    required property var modelData
                    required property int index

                    anchors.fill: parent

                    Repeater {
                        model: root.plotCols

                        delegate: Rectangle {
                            required property int index

                            readonly property real v: root.valueAt(allCurve.modelData.samples,
                                root.t0 + (index + 0.5) / root.plotCols * (root.t1 - root.t0), "pct")

                            visible: v >= 0
                            x: term.cx(index)
                            y: (root.plotHDev - root.fillDev(v)) / term.dpr
                            width: term.cw
                            height: term.lwHeavy
                            antialiasing: false
                            color: [root.theme.accent, root.theme.secondaryAccent, root.theme.textMuted][Math.min(2, allCurve.index)]
                        }
                    }
                }
            }

            // Grid lines (┈), only where the area leaves the cell empty.
            Repeater {
                model: root.gridPcts

                delegate: TermText {
                    required property int modelData
                    readonly property int row: root.rowOf(modelData)

                    tm: term
                    y: term.cy(row)
                    color: root.theme.mix(root.theme.borderInactive, root.theme.bg, 0.45)
                    value: {
                        let s = "";
                        for (let c = 0; c < root.plotCols; ++c) {
                            const d = root.columns[c];
                            s += root.showAll || !d || d.topRow > row ? "┈" : " ";
                        }
                        return s;
                    }
                }
            }

            // Now: a thin column through the area, a dot riding on top of it.
            Rectangle {
                visible: root.nowInside
                x: term.lineX(root.nowCol)
                width: term.lw
                height: plot.height
                antialiasing: false
                color: root.theme.mix(root.theme.fg, root.theme.bg, 0.35)
            }
            Rectangle {
                readonly property var colData: root.columns[root.nowCol]
                visible: root.nowInside && !root.showAll && colData !== undefined
                x: term.lineX(root.nowCol)
                y: colData ? (root.plotHDev - colData.fill) / term.dpr : 0
                width: term.lw
                height: colData ? colData.fill / term.dpr : 0
                antialiasing: false
                color: root.theme.fg
            }
            TermText {
                tm: term
                readonly property var colData: root.columns[root.nowCol]
                visible: root.nowInside && !root.showAll && colData !== undefined && colData.topRow >= 1
                x: term.cx(root.nowCol)
                y: colData ? term.cy(colData.topRow - 1) : 0
                value: "●"
                strong: true
                color: root.theme.fg
            }

            // "Now 14:30" on the top row while the area stays clear of it.
            TermText {
                tm: term
                readonly property bool free: root.columns.every(d => d.topRow > 1)
                readonly property string label: " " + i18ndc(root.domain, "The current time on the chart", "Now %1",
                                                                root.shortTime(root.now)) + " "
                visible: root.nowInside && !root.showAll && free && term.len(label) + 2 < root.plotCols
                x: term.cx(Math.max(0, Math.min(root.plotCols - term.len(label), root.nowCol - Math.floor(term.len(label) / 2))))
                value: label
                strong: true
                color: root.theme.fg
                inverse: true
                inverseColor: root.theme.bg
            }

            // Scrub guide.
            Rectangle {
                visible: scrub.containsMouse
                x: term.cx(scrub.column) + Math.floor((term.cwDev - term.lwDev * 2) / 2) / term.dpr
                width: term.lwHeavy
                height: plot.height
                antialiasing: false
                color: root.theme.borderActive
            }

            MouseArea {
                id: scrub

                readonly property int column: Math.max(0, Math.min(root.plotCols - 1, Math.floor(mouseX / term.cw)))

                anchors.fill: parent
                hoverEnabled: true
                // Looking only: a click here must never change brightness.
                acceptedButtons: Qt.NoButton
            }
        }

        // Hour axis. Its background is the sky light of each column (night dark,
        // day light), so the day's phases read without a row of their own.
        Repeater {
            model: root.available && !root.showAll ? root.plotCols : 0

            delegate: Rectangle {
                required property int index
                readonly property real light: Term.skyLight((index + 0.5) / root.plotCols * (root.t1 - root.t0), root.bounds)

                x: plot.x + term.cx(index)
                y: term.cy(1 + root.chartRows)
                width: term.cw
                height: term.ch
                antialiasing: false
                color: root.theme.mix(root.theme.bg, root.theme.secondaryAccent, 0.05 + 0.2 * light)
            }
        }

        TermText {
            tm: term
            x: plot.x
            y: term.cy(1 + root.chartRows)
            color: root.theme.textMuted
            value: {
                const width = root.plotCols;
                const steps = [2, 3, 4, 6, 12];
                let step = 12;
                for (const s of steps) {
                    if (width * s / 24 >= (root.use12h ? 6 : 8)) {
                        step = s;
                        break;
                    }
                }
                const row = Array(width).fill(" ");
                let nextFree = 0;
                for (let hour = 0; hour <= 24; hour += step) {
                    const label = root.use12h
                        ? ((hour % 12 === 0 ? 12 : hour % 12) + (hour % 24 < 12 ? "am" : "pm"))
                        : String(hour).padStart(2, "0") + ":00";
                    const center = Math.round(hour / 24 * (width - 1));
                    const start = Math.min(Math.max(0, center - Math.floor(label.length / 2)), Math.max(0, width - label.length));
                    if (start < nextFree) {
                        continue;
                    }
                    for (let i = 0; i < label.length && start + i < width; ++i) {
                        row[start + i] = label[i];
                    }
                    nextFree = start + label.length + 1;
                }
                return row.join("");
            }
        }

    }
}
