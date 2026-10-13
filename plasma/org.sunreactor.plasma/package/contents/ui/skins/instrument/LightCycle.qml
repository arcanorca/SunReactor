/*
 * Light cycle, instrument family (the hero): today's policy curve from the
 * daemon's day_curve, read like a measurement instrument.
 *
 *   LIGHT CYCLE ─────────────────────────────────── [MI] [LEN] [ALL]
 *   TARGET · NOW                       APPLIED   39
 *   41 %                               WEATHER   −5
 *                                      RANGE   1–50
 *   ┌ graticule ─────────────────────────────────────────────┐ 100
 *   │ night │ curve, weather-cost band, limits, now cursor   │  50
 *   └───────────────────────────────────────────────────────┘   0
 *    00          06          12          18          24
 *
 * Hover scrubs (time -> target, clear-sky target, weather cost) and never
 * sets brightness. Rendering by flavour: line plot on a printed graticule
 * (Braun, ThinkPad, CRT), phosphor trace on a scope graticule with corner
 * readouts (oscilloscope), a segment bar-graph with ghost cells (VFD), a warm
 * neon trace behind smoked glass (Nixie). Bloom comes from widget.glow and is
 * applied to the static layers only; the hover cursor is drawn above them, so
 * scrubbing never re-renders a blur.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Curve.js" as Curve

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }

    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves`, or -1 for "All". */
    property int selected: 0
    readonly property bool showAll: selected < 0 && curves.length > 1
    readonly property var current: curves.length > 0
        ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null

    readonly property real t0: available ? curves[0].samples[0].t : 0
    readonly property real t1: available ? curves[0].samples[curves[0].samples.length - 1].t : 1
    // Derived from the curve itself, not from t0/t1: while the bindings above
    // update one after the other (t1 new, t0 still 0) a difference of t1 - t0 is
    // ~497 000 h, which made the tick scales instantiate that many items.
    readonly property real hours: available
        ? Math.max(1, Math.min(48, Math.round((curves[0].samples[curves[0].samples.length - 1].t
                                               - curves[0].samples[0].t) / 3600)))
        : 1
    readonly property real now: client.nowEpochS
    readonly property bool nowInDay: now >= t0 && now <= t1
    readonly property var sun: available && curve.sun ? curve.sun : ({})

    function valueAt(samples, t, field) {
        return Curve.valueAt(samples, t, field);
    }

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    function shortName(id) {
        return Curve.shortName(id);
    }

    function pctText(v) {
        return v >= 0 ? String(Math.round(v)) : i18ndc(domain, "No value is known yet", "—");
    }

    readonly property bool scrubbing: hover.active && current !== null && !showAll
    readonly property real readTime: scrubbing ? hover.time : now
    readonly property real readTarget: current ? valueAt(current.samples, readTime, "pct") : -1
    readonly property real readClearSky: current ? valueAt(current.samples, readTime, "pctNoWeather") : -1
    readonly property real readCost: readTarget >= 0 && readClearSky >= 0 ? readClearSky - readTarget : 0
    readonly property int appliedNow: current ? appliedFor(current.monitorId) : -1

    spacing: style.rowGap

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && readTarget >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(readTarget))
        : ""

    // --- legend with the channel selector ---------------------------------------
    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.domain, "Instrument legend over today's brightness curve", "Light cycle")

        Repeater {
            model: root.curves.length > 1 ? root.curves.length + 1 : 0

            delegate: Tag {
                id: channelTag

                required property int index
                readonly property bool isAll: index === root.curves.length
                readonly property int value: isAll ? -1 : index

                anchors.verticalCenter: parent ? parent.verticalCenter : undefined
                style: root.style
                interactive: true
                bare: true
                filled: root.selected === value
                tone: root.style.ink
                text: isAll ? i18ndc(root.domain, "Show the curves of every display", "All")
                            : root.shortName(root.curves[index].monitorId)
                Accessible.name: isAll ? text : root.curves[index].monitorId
                onClicked: root.selected = value
            }
        }
    }

    DisplayWindow {
        id: window

        Layout.fillWidth: true
        style: root.style
        spacing: root.style.rowGap

        // --- readouts -----------------------------------------------------------
        Item {
            id: readouts

            Layout.fillWidth: true
            implicitHeight: Math.max(primary.implicitHeight, secondary.implicitHeight)

            Glow {
                anchors.fill: secondary
                source: secondary
                strength: root.style.emissive ? root.style.glow * 0.5 : 0
            }

            Column {
                id: primary

                spacing: root.style.snap(root.style.gap * 0.75)

                Text {
                    text: root.showAll
                        ? i18ndc(root.domain, "Readout caption: every display is plotted", "Today · all displays")
                        : (root.scrubbing
                            ? i18ndc(root.domain, "Readout caption, then the time under the pointer", "Cursor · %1",
                                     Format.timeText(root.readTime))
                            : i18ndc(root.domain, "Readout caption: what the curve targets right now", "Target · now"))
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.scrubbing ? root.style.ink : root.style.inkMuted
                }

                Row {
                    spacing: root.style.snap(root.style.gap * 0.75)

                    Digits {
                        id: bigValue

                        style: root.style
                        text: root.showAll ? "--" : (root.readTarget >= 0 ? String(Math.round(root.readTarget)) : "--")
                        color: root.style.isBraun || root.style.isThinkpad ? root.style.ink : root.style.ink
                    }

                    Text {
                        anchors.baseline: bigValue.baseline
                        text: "%"
                        textFormat: Text.PlainText
                        font: root.style.legendFont
                        color: root.style.inkMuted
                    }
                }
            }

            GridLayout {
                id: secondary

                anchors.right: parent.right
                anchors.bottom: primary.bottom
                columns: 2
                columnSpacing: root.style.gap2
                rowSpacing: root.style.snap(root.style.gap * 0.5)
                visible: !root.showAll && root.current !== null

                Text {
                    Layout.alignment: Qt.AlignRight | Qt.AlignBaseline
                    text: root.scrubbing
                        ? i18ndc(root.domain, "Readout: target with a clear sky", "Clear sky")
                        : i18ndc(root.domain, "Readout: brightness the daemon last applied", "Applied")
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkMuted
                }
                Text {
                    Layout.alignment: Qt.AlignRight | Qt.AlignBaseline
                    Layout.minimumWidth: rangeMetrics.width
                    horizontalAlignment: Text.AlignRight
                    text: root.scrubbing ? root.pctText(root.readClearSky) : root.pctText(root.appliedNow)
                    textFormat: Text.PlainText
                    font: root.style.dataFont
                    color: root.style.ink
                }

                Text {
                    Layout.alignment: Qt.AlignRight | Qt.AlignBaseline
                    text: i18ndc(root.domain, "Readout: brightness the weather takes away", "Weather")
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkMuted
                }
                Text {
                    Layout.alignment: Qt.AlignRight | Qt.AlignBaseline
                    horizontalAlignment: Text.AlignRight
                    text: Math.round(root.readCost) >= 1
                        ? i18ndc(root.domain, "Brightness the weather takes away, in percent points", "−%1",
                                 Math.round(root.readCost))
                        : "0"
                    textFormat: Text.PlainText
                    font: root.style.dataFont
                    color: Math.round(root.readCost) >= 1 ? root.style.warning : root.style.inkMuted
                }

                Text {
                    Layout.alignment: Qt.AlignRight | Qt.AlignBaseline
                    text: i18ndc(root.domain, "Readout: the display's automatic minimum and maximum", "Range")
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkMuted
                }
                Text {
                    Layout.alignment: Qt.AlignRight | Qt.AlignBaseline
                    horizontalAlignment: Text.AlignRight
                    text: root.current ? i18ndc(root.domain, "Minimum to maximum percent", "%1–%2",
                                                root.current.minPct, root.current.maxPct) : ""
                    textFormat: Text.PlainText
                    font: root.style.dataFont
                    color: root.style.ink
                }

                TextMetrics {
                    id: rangeMetrics
                    font: root.style.dataFont
                    text: "100–100"
                }
            }
        }

        // --- plot ---------------------------------------------------------------
        Item {
            id: plotBox

            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.unit * (root.theme.compact ? 3.8 : 4.2))

            TextMetrics {
                id: gutterMetrics
                font: root.style.dataSmallFont
                text: "100"
            }

            readonly property real gutter: root.style.snap(gutterMetrics.width + root.style.gap * 1.5)

            Item {
                id: plot

                width: root.style.snap(plotBox.width - plotBox.gutter)
                height: plotBox.height

                function xAt(t) {
                    return (t - root.t0) / Math.max(1, root.t1 - root.t0) * (width - root.style.hairline);
                }
                function yAt(pct) {
                    return (1 - Math.max(0, Math.min(100, pct)) / 100) * (height - root.style.hairline);
                }
                function sx(t) {
                    return root.style.snap(xAt(t));
                }
                function sy(pct) {
                    return root.style.snap(yAt(pct));
                }
                function polyline(samples, field) {
                    return samples.map(s => Qt.point(xAt(s.t), yAt(s[field])));
                }

                Loader {
                    anchors.fill: parent
                    sourceComponent: root.style.isVfd ? barsPlot : linePlot
                }

                // Now cursor (above the glow layers, but static between polls).
                Rectangle {
                    visible: root.nowInDay
                    x: plot.sx(root.now)
                    y: 0
                    width: root.style.line
                    height: plot.height
                    color: root.style.theme.alpha(root.style.ink, root.style.isVfd ? 0.0 : 0.55)
                }
                Rectangle {
                    // Applied level when it differs from the curve (override, lag).
                    visible: root.nowInDay && !root.showAll && root.appliedNow >= 0 && !root.style.isVfd
                        && Math.abs(root.appliedNow - root.valueAt(root.current.samples, root.now, "pct")) >= 1
                    width: root.style.snap(9)
                    height: width
                    radius: root.style.isThinkpad ? 0 : width / 2
                    antialiasing: true
                    x: plot.sx(root.now) - width / 2 + root.style.line / 2
                    y: plot.yAt(root.appliedNow) - height / 2
                    color: "transparent"
                    border.width: root.style.line
                    border.color: root.style.ink
                }
                Rectangle {
                    visible: root.nowInDay && !root.showAll && root.current !== null && !root.style.isVfd
                    readonly property real level: root.current ? root.valueAt(root.current.samples, root.now, "pct") : 0
                    width: root.style.snap(root.style.isThinkpad ? 8 : 7)
                    height: width
                    radius: width / 2
                    antialiasing: true
                    x: plot.sx(root.now) - width / 2 + root.style.line / 2
                    y: plot.yAt(level) - height / 2
                    color: root.style.accent
                    border.width: root.style.hairline
                    border.color: root.style.isScope || root.style.isCrt ? root.style.accent : root.style.panel
                }

                // Hover cursor: never inside a glow source.
                Rectangle {
                    visible: root.scrubbing
                    x: root.style.snap(hover.mouseX)
                    y: 0
                    width: root.style.hairline
                    height: plot.height
                    color: root.style.theme.borderActive
                }
                Rectangle {
                    visible: root.scrubbing && root.readTarget >= 0
                    x: root.style.snap(hover.mouseX)
                    y: plot.sy(root.readTarget)
                    width: plot.width - x
                    height: root.style.hairline
                    color: root.style.theme.alpha(root.style.theme.borderActive, 0.6)
                }
                Rectangle {
                    visible: root.scrubbing && root.readTarget >= 0
                    width: root.style.snap(5)
                    height: width
                    x: root.style.snap(hover.mouseX) - width / 2 + root.style.hairline / 2
                    y: plot.yAt(root.readTarget) - height / 2
                    color: root.style.theme.borderActive
                }

                MouseArea {
                    id: hover

                    readonly property bool active: containsMouse && root.available
                    readonly property real time: root.t0 + Math.max(0, Math.min(1, mouseX / Math.max(1, width)))
                        * (root.t1 - root.t0)

                    anchors.fill: parent
                    hoverEnabled: true
                    // Looking only: a click here must never change brightness.
                    acceptedButtons: Qt.NoButton
                }
            }

            // Y axis in the gutter: 0 / 50 / 100, plus the cursor value tag.
            Repeater {
                model: [0, 50, 100]

                delegate: Text {
                    required property int modelData
                    x: plot.width + root.style.gap
                    y: Math.max(0, Math.min(plotBox.height - height, plot.yAt(modelData) - height / 2))
                    text: String(modelData)
                    textFormat: Text.PlainText
                    font: root.style.dataSmallFont
                    color: root.style.inkFaint
                }
            }
        }

        TickScale {
            Layout.fillWidth: true
            Layout.rightMargin: plotBox.gutter
            Layout.topMargin: -root.style.rowGap + root.style.hairline * 2
            style: root.style
            from: 0
            to: root.hours
            minor: 1
            major: 6
            labelEvery: 6
            label: v => String(v).padStart(2, "0")
            minorColor: root.style.ruleFaint
            tickColor: root.style.rule
        }
    }

    // --- line plot (Braun, ThinkPad, CRT, scope, nixie) --------------------------
    Component {
        id: linePlot

        Item {
            id: lp

            readonly property bool scope: root.style.isScope

            // Day and night: night darker, twilight half way (printed panels).
            Repeater {
                model: !lp.scope && root.sun.sunrise > 0 && root.sun.sunset > 0
                    ? [
                        { from: root.t0, to: root.sun.dawn > 0 ? root.sun.dawn : root.sun.sunrise, a: 1.0 },
                        { from: root.sun.dawn > 0 ? root.sun.dawn : root.sun.sunrise, to: root.sun.sunrise, a: 0.5 },
                        { from: root.sun.sunset, to: root.sun.dusk > 0 ? root.sun.dusk : root.sun.sunset, a: 0.5 },
                        { from: root.sun.dusk > 0 ? root.sun.dusk : root.sun.sunset, to: root.t1, a: 1.0 },
                    ]
                    : []

                delegate: Rectangle {
                    required property var modelData
                    x: plot.sx(modelData.from)
                    width: Math.max(0, plot.sx(modelData.to) - x)
                    y: 0
                    height: plot.height
                    color: root.style.theme.alpha(root.style.ink, (root.style.emissive ? 0.025 : 0.035) * modelData.a)
                }
            }

            // Graticule: printed grid, or a scope's 8 x 8 divisions with a
            // finely ticked centre cross.
            Repeater {
                model: lp.scope ? 9 : 5
                delegate: Rectangle {
                    required property int index
                    readonly property real pct: lp.scope ? index * 12.5 : index * 25
                    x: 0
                    y: plot.sy(pct)
                    width: plot.width
                    height: root.style.hairline
                    color: (pct === 0 || (lp.scope && pct === 100)) ? root.style.rule
                        : (pct === 50 && lp.scope ? root.style.rule : root.style.ruleFaint)
                }
            }
            Repeater {
                model: lp.scope ? 9 : Math.floor(root.hours / 3) + 1
                delegate: Rectangle {
                    required property int index
                    readonly property real hour: lp.scope ? index * root.hours / 8 : index * 3
                    x: plot.sx(root.t0 + hour * 3600)
                    y: 0
                    width: root.style.hairline
                    height: plot.height
                    color: lp.scope ? ((index === 0 || index === 8 || index === 4) ? root.style.rule : root.style.ruleFaint)
                        : (index % 2 === 0 ? root.style.ruleFaint : root.style.theme.alpha(root.style.ruleFaint, 0.5))
                }
            }
            Repeater {
                // Scope centre cross: five minor ticks per division.
                model: lp.scope ? 41 : 0
                delegate: Item {
                    required property int index
                    Rectangle {
                        x: plot.sx(root.t0 + index / 40 * (root.t1 - root.t0))
                        y: plot.sy(50) - height / 2
                        width: root.style.hairline
                        height: root.style.snap(index % 5 === 0 ? 0 : 4)
                        color: root.style.rule
                    }
                    Rectangle {
                        x: plot.sx(root.t0 + (root.t1 - root.t0) / 2) - width / 2
                        y: plot.sy(index * 2.5)
                        width: root.style.snap(index % 5 === 0 ? 0 : 4)
                        height: root.style.hairline
                        color: root.style.rule
                    }
                }
            }

            // Limits band of the selected display.
            Rectangle {
                visible: !root.showAll && root.current !== null
                x: 0
                width: plot.width
                y: root.current ? plot.sy(root.current.maxPct) : 0
                height: root.current ? plot.sy(Math.max(0, root.current.minPct)) - y : 0
                color: root.style.theme.alpha(root.style.ink, 0.025)
            }
            Repeater {
                model: !root.showAll && root.current
                    ? [{ pct: root.current.maxPct, label: i18ndc(root.domain, "Plot label: automatic maximum", "Max") },
                       { pct: root.current.minPct, label: i18ndc(root.domain, "Plot label: automatic minimum", "Min") }]
                    : []
                delegate: Item {
                    id: limit
                    required property var modelData
                    readonly property bool roomAbove: plot.sy(modelData.pct) > limitText.implicitHeight + 1
                    Rectangle {
                        x: 0
                        y: plot.sy(limit.modelData.pct)
                        width: plot.width
                        height: root.style.hairline
                        color: root.style.theme.alpha(root.style.accent2, 0.45)
                    }
                    Text {
                        id: limitText
                        visible: limit.modelData.pct > 6 || limit.modelData.label.length === 0
                        x: root.style.gap * 0.75
                        y: limit.roomAbove ? plot.sy(limit.modelData.pct) - implicitHeight
                                           : plot.sy(limit.modelData.pct) + root.style.hairline * 2
                        text: limit.modelData.label + " " + limit.modelData.pct
                        textFormat: Text.PlainText
                        font: root.style.legendFont
                        color: root.style.theme.alpha(root.style.accent2, 0.85)
                    }
                }
            }

            // Sunrise, solar noon, sunset: ticks hanging from the top edge.
            Repeater {
                model: [root.sun.sunrise, root.sun.solarNoon, root.sun.sunset].filter(t => t > 0)
                delegate: Rectangle {
                    required property real modelData
                    required property int index
                    x: plot.sx(modelData)
                    y: 0
                    width: root.style.line
                    height: root.style.snap(index === 1 ? 4 : 6)
                    color: root.style.accent2
                }
            }

            // What the weather costs: the band between the clear-sky curve
            // and the target, plus the clear-sky curve as a dashed line.
            Shape {
                anchors.fill: parent
                visible: !root.showAll && root.current !== null
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.theme.alpha(root.style.warning, root.style.emissive ? 0.16 : 0.22)
                    PathPolyline {
                        path: root.current
                            ? plot.polyline(root.current.samples, "pctNoWeather")
                                .concat(plot.polyline(root.current.samples, "pct").reverse())
                            : []
                    }
                }
                ShapePath {
                    strokeColor: root.style.theme.alpha(root.style.warning, 0.75)
                    strokeWidth: root.style.hairline
                    strokeStyle: ShapePath.DashLine
                    dashPattern: [3, 3]
                    fillColor: "transparent"
                    PathPolyline {
                        path: root.current ? plot.polyline(root.current.samples, "pctNoWeather") : []
                    }
                }
            }

            Glow {
                anchors.fill: traces
                source: traces
                strength: root.style.emissive ? root.style.glow : 0
            }

            Item {
                id: traces

                anchors.fill: parent

                Repeater {
                    model: root.showAll ? root.curves : (root.current ? [root.current] : [])

                    delegate: Shape {
                        id: trace

                        required property var modelData
                        required property int index

                        anchors.fill: parent
                        preferredRendererType: Shape.CurveRenderer

                        ShapePath {
                            strokeColor: !root.showAll || trace.index === 0 ? root.style.accent
                                : (trace.index === 1 ? root.style.accent2 : root.style.inkMuted)
                            strokeWidth: root.style.line2
                            fillColor: "transparent"
                            joinStyle: ShapePath.RoundJoin
                            capStyle: ShapePath.RoundCap
                            PathPolyline {
                                path: plot.polyline(trace.modelData.samples, "pct")
                            }
                        }
                    }
                }
            }

            // Scope corner readouts: channel, scale per division, clock.
            Repeater {
                model: lp.scope ? 4 : 0
                delegate: Text {
                    required property int index
                    readonly property bool atRight: index % 2 === 1
                    readonly property bool atBottom: index >= 2
                    x: atRight ? plot.width - implicitWidth - root.style.gap : root.style.gap
                    y: atBottom ? plot.height - implicitHeight - root.style.hairline * 2 : root.style.hairline * 2
                    text: {
                        switch (index) {
                        case 0:
                            return root.showAll ? i18ndc(root.domain, "Scope readout: every channel", "ALL CH")
                                : i18ndc(root.domain, "Scope readout: channel number and display", "CH%1 %2",
                                         root.selected + 1, root.shortName(root.current ? root.current.monitorId : ""));
                        case 1:
                            return i18ndc(root.domain, "Scope readout: vertical scale", "12.5%/DIV");
                        case 2:
                            return i18ndc(root.domain, "Scope readout: horizontal scale in hours", "%1H/DIV",
                                          (root.hours / 8).toFixed(0));
                        default:
                            return Format.timeText(root.now);
                        }
                    }
                    textFormat: Text.PlainText
                    font: root.style.dataSmallFont
                    color: root.style.inkMuted
                }
            }
        }
    }

    // --- segment bar graph (VFD) --------------------------------------------------
    Component {
        id: barsPlot

        Item {
            id: bars

            readonly property int columns: Math.max(24, Math.min(48, Math.floor(plot.width / root.style.snap(6))))
            readonly property int rows: 20
            readonly property real pitchX: plot.width / columns
            readonly property real pitchY: plot.height / rows
            readonly property real cellW: Math.max(root.style.hairline, root.style.floorSnap(pitchX * 0.7))
            readonly property real cellH: Math.max(root.style.hairline, root.style.floorSnap(pitchY * 0.62))
            readonly property int nowColumn: root.nowInDay
                ? Math.min(columns - 1, Math.floor((root.now - root.t0) / (root.t1 - root.t0) * columns)) : -1

            function cell(c, r) {
                const x = root.style.snap(c * pitchX + (pitchX - cellW) / 2);
                const y = root.style.snap(plot.height - (r + 1) * pitchY + (pitchY - cellH) / 2);
                return "M" + x + " " + y + " h" + cellW + " v" + cellH + " h" + (-cellW) + " Z ";
            }
            function level(c, field, samples) {
                const t = root.t0 + (c + 0.5) / columns * (root.t1 - root.t0);
                const v = root.valueAt(samples, t, field);
                return v < 0 ? 0 : Math.round(v / 100 * rows);
            }

            // Paths: unlit, lit, weather-dimmed and now-column cells.
            readonly property var paths: {
                const out = { ghost: "", lit: "", cost: "", now: "", limit: "" };
                const samples = root.current ? root.current.samples : [];
                for (let c = 0; c < columns; ++c) {
                    const lit = level(c, "pct", samples);
                    const clear = Math.max(lit, level(c, "pctNoWeather", samples));
                    for (let r = 0; r < rows; ++r) {
                        const p = cell(c, r);
                        if (r < lit) {
                            if (c === nowColumn) {
                                out.now += p;
                            } else {
                                out.lit += p;
                            }
                        } else if (r < clear) {
                            out.cost += p;
                        } else {
                            out.ghost += p;
                        }
                    }
                }
                return out;
            }

            Shape {
                anchors.fill: parent
                antialiasing: false
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.ghost
                    PathSvg { path: bars.paths.ghost }
                }
            }

            // Limits: a lit tick at the left and right of the max and min rows.
            Repeater {
                model: !root.showAll && root.current ? [root.current.maxPct, root.current.minPct] : []
                delegate: Rectangle {
                    required property real modelData
                    x: plot.width + root.style.hairline * 2
                    y: root.style.snap(plot.height - Math.round(modelData / 100 * bars.rows) * bars.pitchY)
                    width: root.style.snap(3)
                    height: root.style.line
                    color: root.style.accent2
                }
            }

            Glow {
                anchors.fill: litCells
                source: litCells
                strength: root.style.glow
            }

            Shape {
                id: litCells

                anchors.fill: parent
                antialiasing: false
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.theme.alpha(root.style.warning, 0.55)
                    PathSvg { path: bars.paths.cost }
                }
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.ink
                    PathSvg { path: bars.paths.lit }
                }
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.accent
                    PathSvg { path: bars.paths.now }
                }
            }

            // Sun events: dots under the columns at sunrise and sunset.
            Repeater {
                model: [root.sun.sunrise, root.sun.solarNoon, root.sun.sunset].filter(t => t > 0)
                delegate: Rectangle {
                    required property real modelData
                    x: root.style.snap(plot.xAt(modelData) - width / 2)
                    y: 0
                    width: root.style.line2
                    height: root.style.line2
                    color: root.style.accent2
                }
            }
        }
    }
}
