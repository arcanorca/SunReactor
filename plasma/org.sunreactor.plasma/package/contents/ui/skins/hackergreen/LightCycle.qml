/*
 * hackergreen light cycle, the brightness page's hero: the daily brightness
 * target as a phosphor chart. The area is bright green columns in eighth-cell
 * steps, capped by a bright line. Above the area each column carries a static
 * falling-code texture (digits and symbols, low contrast), frozen as if the rain
 * stopped mid-fall. What the weather takes away is tinted amber (warning). The
 * hero percentage is 2.2x the body size, the largest text in the skin.
 *
 *   [ BRIGHTNESS ]                                  TUE 13 OCT
 *   39%    mi-monitor target now
 *          ▼ weather −12%
 *    100% 0000::..  :..000
 *     50% ...
 *      0% ..
 *          00:00    06:00    12:00    18:00    24:00
 *          NIGHT  DAWN  MORNING   AFTERNOON  DUSK  NIGHT
 *
 * Hovering the chart scrubs: the hero and the legend show the target at the
 * pointer's time. Hover never changes brightness; the chart is only looked at.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../instrument"
import "../terminal"
import "../terminal/Term.js" as Term
import "Hg.js" as Hg

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
    HgCrt {
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

    // --- layout in cells ---------------------------------------------------------
    readonly property bool tabs: curves.length > 1
    readonly property int chartRows: term.comfortable ? 9 : 7
    /*! Gutter of percent labels, "100% " (five cells). */
    readonly property int gutter: 5
    readonly property int cols: term.cols(root.width)
    readonly property int plotCols: Math.max(8, cols - gutter - 1)
    readonly property int plotHDev: chartRows * term.chDev
    readonly property real plotX: term.cx(gutter)

    function colOf(t) {
        return Math.round(Math.max(0, Math.min(1, (t - t0) / Math.max(1, t1 - t0))) * (plotCols - 1));
    }
    function rowOf(pct) {
        return Math.min(chartRows - 1, Math.floor(chartRows * (1 - Math.max(0, Math.min(100, pct)) / 100)));
    }
    /*! Height of a fill in device px, in eighths of a cell, at least one eighth. */
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
            out.push({ v: v, fill: fill, fillNw: fillNw, topRow: chartRows - Math.ceil(fillNw / term.chDev) });
        }
        return out;
    }

    /*! The static code texture, one string per chart row. */
    readonly property var rain: Hg.rain(plotCols, chartRows)
    readonly property color rainColor: root.theme.mix(root.theme.bg, root.theme.textMuted, 0.34)

    readonly property int nowCol: colOf(now)
    readonly property bool nowInside: available && now >= t0 && now <= t1
    readonly property real targetNow: current && nowInside ? valueAt(current.samples, now, "pct") : -1
    readonly property real costNow: current && nowInside
        ? valueAt(current.samples, now, "pctNoWeather") - valueAt(current.samples, now, "pct") : 0
    /*! All curves at this moment, averaged, for the "All" hero. */
    readonly property real meanNow: {
        let sum = 0;
        let n = 0;
        for (const c of curves) {
            const v = valueAt(c.samples, now, "pct");
            if (v >= 0) {
                sum += v;
                n += 1;
            }
        }
        return n > 0 ? sum / n : -1;
    }

    readonly property var bounds: Term.phaseBounds(curve ? curve.sun : null, t0, t1)
    readonly property int currentPhase: Term.phaseAt(now - t0, bounds)

    function shortTime(t) {
        const d = new Date(t * 1000);
        if (!use12h) {
            return Qt.formatTime(d, "HH:mm");
        }
        const h = d.getHours() % 12 === 0 ? 12 : d.getHours() % 12;
        return h + (d.getHours() < 12 ? "am" : "pm");
    }

    // --- scrub: the hovered time, when the chart is pointed at ---------------------
    readonly property real scrubT: t0 + (scrub.column + 0.5) / plotCols * (t1 - t0)
    readonly property real scrubPct: current ? valueAt(current.samples, scrubT, "pct") : -1
    readonly property bool scrubbing: scrub.containsMouse && !showAll && current !== null && scrubPct >= 0

    readonly property string heroText: {
        let v = targetNow;
        if (scrubbing) {
            v = scrubPct;
        } else if (showAll) {
            v = meanNow;
        }
        return v >= 0 ? Math.round(v) + "%" : "—";
    }

    readonly property var legendTop: {
        const t = root.theme;
        if (root.showAll) {
            return [{ t: i18ndc(root.domain, "Every display on the chart", "all displays"), c: t.fg, b: true }];
        }
        const name = root.current ? root.current.monitorId : "";
        const when = root.scrubbing ? root.shortTime(root.scrubT)
            : i18ndc(root.domain, "Brightness target now", "target now");
        return [{ t: name + " ", c: t.fg, b: true }, { t: when, c: t.textMuted }];
    }

    readonly property var legendBottom: {
        const t = root.theme;
        if (root.showAll) {
            const colours = [t.accent, t.secondaryAccent, t.textMuted];
            const out = [];
            root.curves.forEach((c, i) => {
                out.push({ t: (i > 0 ? "  " : "") + "━ ", c: colours[Math.min(i, 2)] });
                out.push({ t: c.monitorId, c: t.textMuted });
            });
            return out;
        }
        if (root.scrubbing) {
            return [{ t: i18ndc(root.domain, "Target at the time under the pointer", "target "), c: t.textMuted },
                    { t: Math.round(root.scrubPct) + "%", c: t.fg, b: true }];
        }
        if (root.costNow >= 1) {
            return [{ t: "▼ ", c: t.warning },
                    { t: i18ndc(root.domain, "Brightness the weather is taking away right now", "weather −%1%",
                                Math.round(root.costNow)), c: t.warning }];
        }
        return [{ t: i18ndc(root.domain, "The weather takes nothing away right now", "no weather cost"), c: t.textMuted }];
    }

    /*! Hour labels on one row: every step hours, centred, dropped when they would collide. */
    function axisText() {
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

    /*! Phase names placed under their spans; a name that would collide is left out. */
    function phaseRow() {
        if (!root.available) {
            return [];
        }
        const names = [i18ndc(root.domain, "Phase of the day", "NIGHT"), i18ndc(root.domain, "Phase of the day", "DAWN"),
                       i18ndc(root.domain, "Phase of the day", "MORNING"), i18ndc(root.domain, "Phase of the day", "AFTERNOON"),
                       i18ndc(root.domain, "Phase of the day", "DUSK"), i18ndc(root.domain, "Phase of the day", "NIGHT")];
        const width = root.plotCols;
        const day = Math.max(1, root.t1 - root.t0);
        const col = s => Math.round(Math.max(0, Math.min(1, s / day)) * (width - 1));
        const occupied = Array(width).fill(false);
        const placed = [];
        for (const index of [2, 3, 0, 5, 1, 4]) {
            const label = names[index];
            const length = Term.len(label);
            const center = Math.floor((col(root.bounds[index]) + col(root.bounds[index + 1])) / 2);
            const start = Math.min(Math.max(0, center - Math.floor(length / 2)), Math.max(0, width - length));
            const lo = Math.max(0, start - 1);
            const hi = Math.min(width, start + length + 1);
            if (length > width || occupied.slice(lo, hi).some(v => v)) {
                continue;
            }
            for (let i = start; i < start + length; ++i) {
                occupied[i] = true;
            }
            const current = index === root.currentPhase
                || (root.currentPhase === 5 && index === 0 && root.now - root.t0 < root.bounds[1]);
            placed.push({ at: start, t: label, current: current });
        }
        return placed;
    }

    implicitHeight: (tabs ? term.cy(2) : 0) + section.implicitHeight

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")

    TermTabs {
        id: switcher

        tm: term
        theme: root.theme
        visible: root.tabs
        width: term.cx(term.cols(root.width))
        labels: root.curves.map(c => Term.shortName(c.monitorId))
            .concat([i18ndc(root.domain, "Show the curves of every display", "All")])
        current: Math.min(root.selected, root.curves.length)
        onActivated: index => root.selected = index
    }

    HgSection {
        id: section

        tm: term
        theme: root.theme
        y: root.tabs ? term.cy(2) : 0
        width: root.width
        title: i18ndc(root.domain, "Section title of the light cycle", "BRIGHTNESS")
        bracket: true
        metaRuns: root.available
            ? [{ t: Qt.formatDate(new Date(root.now * 1000), "ddd d MMM").toUpperCase(), c: root.theme.textMuted }]
            : []
        bodyHeight: term.cy(4 + root.chartRows)

        // Everything here is static between daemon updates: the glow is rendered
        // from it and does not repaint on its own.
        Glow {
            anchors.fill: lit
            source: lit
            strength: root.theme.glow
        }

        Item {
            id: lit

            anchors.fill: parent
            visible: root.available

            // Hero: the target as a large number, 2.2x the body size.
            Text {
                id: hero

                x: term.cx(1)
                y: 0
                height: term.cy(2)
                text: root.heroText
                textFormat: Text.PlainText
                color: root.theme.fg
                font.family: term.family
                font.pixelSize: Math.round(term.pixelSize * 2.2)
                font.bold: true
                font.kerning: false
                verticalAlignment: Text.AlignVCenter
            }

            TermRuns {
                tm: term
                x: hero.x + hero.width + term.cx(2)
                y: term.cy(0)
                cells: Math.max(0, root.cols - Math.ceil(hero.width / term.cw) - 3)
                defaultColor: root.theme.fg
                runs: root.legendTop
            }

            TermRuns {
                tm: term
                x: hero.x + hero.width + term.cx(2)
                y: term.cy(1)
                cells: Math.max(0, root.cols - Math.ceil(hero.width / term.cw) - 3)
                defaultColor: root.theme.textMuted
                runs: root.legendBottom
            }

            // Percent labels in the gutter, a label skipped when a higher one took its row.
            Repeater {
                model: [100, 75, 50, 25, 0]

                delegate: TermText {
                    required property int modelData
                    required property int index

                    readonly property int row: root.rowOf(modelData)

                    tm: term
                    visible: index === 0 || root.rowOf([100, 75, 50, 25, 0][index - 1]) !== row
                    y: term.cy(2 + row)
                    value: Term.padStart(modelData + "%", root.gutter - 1)
                    color: root.theme.textMuted
                }
            }

            // The code texture: under the columns, so the area hides it. The NOW label
            // sits over it on purpose (an opaque capsule), so the texture is not audited.
            Repeater {
                model: root.showAll ? 0 : root.chartRows

                delegate: TermText {
                    required property int index
                    property bool auditIgnore: true

                    tm: term
                    x: root.plotX
                    y: term.cy(2 + index)
                    value: root.rain[index] || ""
                    color: root.rainColor
                }
            }

            // Per column: the weather share, the area, and its bright cap.
            Repeater {
                model: root.showAll ? 0 : root.columns.length

                delegate: Item {
                    id: column

                    required property int index
                    readonly property var colData: root.columns[index]

                    x: root.plotX + term.cx(index)
                    y: term.cy(2)
                    width: term.cw
                    height: term.cy(root.chartRows)

                    Rectangle {
                        visible: column.colData.fillNw > column.colData.fill
                        y: (root.plotHDev - column.colData.fillNw) / term.dpr
                        width: parent.width
                        height: (column.colData.fillNw - column.colData.fill) / term.dpr
                        color: root.theme.mix(root.theme.warning, root.theme.bg, 0.5)
                        antialiasing: false
                    }
                    Rectangle {
                        visible: column.colData.fill > 0
                        y: (root.plotHDev - column.colData.fill) / term.dpr
                        width: parent.width
                        height: column.colData.fill / term.dpr
                        color: root.theme.accent
                        antialiasing: false
                    }
                    Rectangle {
                        visible: column.colData.fill > 0
                        y: (root.plotHDev - column.colData.fill) / term.dpr
                        width: parent.width
                        height: term.lwHeavy
                        color: root.theme.fg
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
                            x: root.plotX + term.cx(index)
                            y: term.cy(2) + (root.plotHDev - root.fillDev(v)) / term.dpr
                            width: term.cw
                            height: term.lwHeavy
                            antialiasing: false
                            color: [root.theme.accent, root.theme.secondaryAccent, root.theme.textMuted][Math.min(2, allCurve.index)]
                        }
                    }
                }
            }

            // Now: a thin column through the chart, and its label on the top row when free.
            Rectangle {
                visible: root.nowInside && !root.showAll
                x: root.plotX + term.lineX(root.nowCol)
                y: term.cy(2)
                width: term.lw
                height: term.cy(root.chartRows)
                antialiasing: false
                color: root.theme.mix(root.theme.fg, root.theme.bg, 0.4)
            }
            TermText {
                tm: term
                readonly property bool free: root.columns.every(d => d.topRow > 1)
                readonly property string label: " " + i18ndc(root.domain, "The current time on the chart", "NOW %1",
                                                              root.shortTime(root.now)) + " "
                visible: root.nowInside && !root.showAll && free && term.len(label) + 2 < root.plotCols
                x: root.plotX + term.cx(Math.max(0, Math.min(root.plotCols - term.len(label), root.nowCol - Math.floor(term.len(label) / 2))))
                y: term.cy(2)
                value: label
                strong: true
                color: root.theme.bg
                inverse: true
                inverseColor: root.theme.fg
            }

            // Hour axis, then the phases of the day.
            TermText {
                tm: term
                x: root.plotX
                y: term.cy(2 + root.chartRows)
                value: root.available ? root.axisText() : ""
                color: root.theme.textMuted
            }

            Repeater {
                model: root.phaseRow()

                delegate: TermText {
                    required property var modelData

                    tm: term
                    x: root.plotX + term.cx(modelData.at)
                    y: term.cy(3 + root.chartRows)
                    value: modelData.t
                    strong: modelData.current
                    color: modelData.current ? root.theme.fg : root.theme.textMuted
                }
            }

            // Scrub: the pointer's column, looked at only.
            Rectangle {
                visible: scrub.containsMouse && !root.showAll
                x: root.plotX + term.cx(scrub.column) + Math.floor((term.cwDev - term.lwDev * 2) / 2) / term.dpr
                y: term.cy(2)
                width: term.lwHeavy
                height: term.cy(root.chartRows)
                antialiasing: false
                color: root.theme.borderActive
            }

            MouseArea {
                id: scrub

                readonly property int column: Math.max(0, Math.min(root.plotCols - 1, Math.floor(mouseX / term.cw)))

                x: root.plotX
                y: term.cy(2)
                width: term.cx(root.plotCols)
                height: term.cy(root.chartRows)
                hoverEnabled: true
                // Looking only: a click here must never change brightness.
                acceptedButtons: Qt.NoButton
            }
        }
    }
}
