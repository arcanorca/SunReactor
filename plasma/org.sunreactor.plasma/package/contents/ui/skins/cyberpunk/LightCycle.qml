/*
 * Light cycle, cyberpunk (the hero): today's policy curve from the daemon's
 * day_curve, drawn as a HUD plot.
 *
 *   24 H WINDOW ──────────────────────────────── LIGHT CYCLE
 *   TARGET · NOW                                  [MI] [LEN] [ALL]
 *   39%     TARGET [39]  APPLIED [39]  WEATHER [−5]  RANGE [1–50]
 *   ┌ dark grid, night shade, cyan weather band ──────┐ 100
 *   │ yellow curve, red NOW cursor and tag            │  50
 *   └─────────────────────────────────────────────────┘   0
 *    00      06      12      18      24
 *
 * Hover scrubs the readouts and never sets brightness. The glitch on the hero
 * figure is static; nothing animates.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument" as Instrument
import "../instrument/Curve.js" as Curve
import "Geometry.js" as G

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property CyberStyle style: CyberStyle {
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
    readonly property real hours: available
        ? Math.max(1, Math.min(48, Math.round((t1 - t0) / 3600))) : 1
    readonly property real now: client.nowEpochS
    readonly property bool nowInDay: available && now >= t0 && now <= t1
    readonly property var sun: available && curve.sun ? curve.sun : ({})
    readonly property bool sunKnown: available && sun.sunrise > 0 && sun.sunset > 0

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
        return v >= 0 ? String(Math.round(v)) : "--";
    }

    readonly property bool scrubbing: hover.active && current !== null && !showAll
    readonly property real readTime: scrubbing ? hover.time : now
    readonly property real readTarget: current ? valueAt(current.samples, readTime, "pct") : -1
    readonly property real readClearSky: current ? valueAt(current.samples, readTime, "pctNoWeather") : -1
    readonly property real readCost: readTarget >= 0 && readClearSky >= 0 ? readClearSky - readTarget : 0
    readonly property int appliedNow: current ? appliedFor(current.monitorId) : -1
    readonly property real nowLevel: current && nowInDay ? valueAt(current.samples, now, "pct") : -1

    spacing: style.rowGap

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && readTarget >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(readTarget))
        : ""

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        title: i18ndc(root.domain, "Panel title: today's brightness curve", "Light cycle")
        tag: root.available ? i18ndc(root.domain, "Plot window in hours", "%1 H WINDOW", root.hours) : ""

        // The caption (what the readouts describe) and the channel chips: one line; the chips
        // wrap onto a second line when a display count leaves no room for them.
        Flow {
            Layout.fillWidth: true
            spacing: root.style.gap

            Text {
                text: root.showAll
                    ? i18ndc(root.domain, "Readout caption: every display is plotted", "TODAY · ALL DISPLAYS")
                    : (root.scrubbing
                        ? i18ndc(root.domain, "Readout caption, then the time under the pointer", "CURSOR · %1",
                                 Format.timeText(root.readTime))
                        : i18ndc(root.domain, "Readout caption: what the curve targets right now", "TARGET · NOW"))
                textFormat: Text.PlainText
                font: root.style.labelFont
                color: root.scrubbing ? root.style.ink : root.style.muted
            }

            Repeater {
                model: root.curves.length > 1 ? root.curves.length + 1 : 0

                delegate: ChamferButton {
                    id: chip

                    required property int index
                    readonly property bool isAll: index === root.curves.length
                    readonly property int value: isAll ? -1 : index

                    style: root.style
                    height: root.style.tabHeight
                    font: root.style.techFont
                    text: isAll ? i18ndc(root.domain, "Channel chip: every display", "ALL")
                        : root.shortName(root.curves[index].monitorId)
                    active: root.selected === value
                    Accessible.name: isAll ? text : root.curves[index].monitorId

                    onClicked: root.selected = value
                }
            }
        }

        // The hero figure, and beside it one row of four bracket readouts.
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap2

            Row {
                spacing: root.style.gap * 0.5

                GlitchValue {
                    id: hero

                    style: root.style
                    text: root.showAll || root.readTarget < 0 ? "--" : String(Math.round(root.readTarget))
                }
                Text {
                    // Sits on the figure's baseline: its bottom is the figure's bottom
                    // less the difference of the two descents (a Row cannot anchor to
                    // the figure's inner text, which is not a sibling).
                    anchors.bottom: hero.bottom
                    anchors.bottomMargin: root.style.snap(root.style.sizeHero * 0.12)
                    text: "%"
                    textFormat: Text.PlainText
                    font: root.style.bodyFont
                    color: root.style.muted
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Row {
                id: readouts

                visible: !root.showAll && root.current !== null
                Layout.alignment: Qt.AlignVCenter
                spacing: root.style.gap

                Column {
                    spacing: root.style.snap(root.style.unit * 0.2)

                    Text {
                        text: i18ndc(root.domain, "Label before the automatic target value", "TARGET")
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.muted
                    }
                    Bracket {
                        style: root.style
                        value: root.pctText(root.readTarget)
                        font: root.style.techFont
                        color: root.style.ink
                    }
                }
                Column {
                    spacing: root.style.snap(root.style.unit * 0.2)

                    Text {
                        text: root.scrubbing
                            ? i18ndc(root.domain, "Readout: target with a clear sky", "CLEAR SKY")
                            : i18ndc(root.domain, "Readout: brightness the daemon last applied", "APPLIED")
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.muted
                    }
                    Bracket {
                        style: root.style
                        value: root.scrubbing ? root.pctText(root.readClearSky) : root.pctText(root.appliedNow)
                        font: root.style.techFont
                        color: root.style.ink
                    }
                }
                Column {
                    spacing: root.style.snap(root.style.unit * 0.2)

                    Text {
                        text: i18ndc(root.domain, "Readout: brightness the weather takes away", "WEATHER")
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.muted
                    }
                    Bracket {
                        style: root.style
                        value: Math.round(root.readCost) >= 1 ? "−" + Math.round(root.readCost) : "0"
                        font: root.style.techFont
                        color: root.style.cyan
                    }
                }
                Column {
                    spacing: root.style.snap(root.style.unit * 0.2)

                    Text {
                        text: i18ndc(root.domain, "Readout: the display's automatic minimum and maximum", "RANGE")
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.muted
                    }
                    Bracket {
                        style: root.style
                        value: root.current ? root.current.minPct + "–" + root.current.maxPct : "--"
                        font: root.style.techFont
                        color: root.style.ink
                    }
                }
            }
        }

        // The plot: a dark grid with corner brackets, the axis ruler under it.
        Item {
            id: plotBox

            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.unit * 3.2)

            readonly property real axisH: root.style.snap(root.style.sizeTiny * 2.2)
            readonly property real gutter: root.style.snap(gutterMetrics.width + root.style.gap * 2)

            TextMetrics {
                id: gutterMetrics

                font: root.style.techFont
                text: "100"
            }

            Item {
                id: plot

                width: root.style.snap(plotBox.width - plotBox.gutter)
                height: root.style.snap(plotBox.height - plotBox.axisH)

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

                Rectangle {
                    anchors.fill: parent
                    color: root.style.plot
                }

                // Graticule: quarters, and every third hour.
                Repeater {
                    model: 5

                    delegate: Rectangle {
                        required property int index

                        x: 0
                        y: plot.sy(index * 25)
                        width: plot.width
                        height: root.style.hairline
                        color: root.style.ruleFaint
                    }
                }
                Repeater {
                    model: Math.floor(root.hours / 3) + 1

                    delegate: Rectangle {
                        required property int index

                        x: plot.sx(root.t0 + index * 3 * 3600)
                        y: 0
                        width: root.style.hairline
                        height: plot.height
                        color: root.style.ruleFaint
                    }
                }

                // Night: the hours before sunrise and after sunset are shaded.
                Repeater {
                    model: root.sunKnown
                        ? [{ from: root.t0, to: root.sun.sunrise }, { from: root.sun.sunset, to: root.t1 }]
                        : []

                    delegate: Rectangle {
                        required property var modelData

                        x: plot.sx(modelData.from)
                        width: Math.max(0, plot.sx(modelData.to) - x)
                        y: 0
                        height: plot.height
                        color: Qt.rgba(0, 0, 0, 0.3)
                    }
                }

                // Automatic limits of the selected display.
                Repeater {
                    model: !root.showAll && root.current ? [root.current.maxPct, root.current.minPct] : []

                    delegate: Rectangle {
                        required property real modelData

                        x: 0
                        y: plot.sy(modelData)
                        width: plot.width
                        height: root.style.hairline
                        color: root.style.theme.alpha(root.style.muted, 0.6)
                    }
                }

                // What the weather costs: the cyan band between the clear-sky curve and the target,
                // and the clear-sky curve as a dashed line.
                Shape {
                    anchors.fill: parent
                    visible: !root.showAll && root.current !== null
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeWidth: -1
                        fillColor: root.style.theme.alpha(root.style.cyan, 0.22)
                        PathPolyline {
                            path: root.current
                                ? plot.polyline(root.current.samples, "pctNoWeather")
                                    .concat(plot.polyline(root.current.samples, "pct").reverse())
                                : []
                        }
                    }
                    ShapePath {
                        strokeColor: root.style.theme.alpha(root.style.cyan, 0.85)
                        strokeWidth: root.style.hairline
                        fillColor: "transparent"
                        strokeStyle: ShapePath.DashLine
                        dashPattern: [3, 3]
                        PathPolyline {
                            path: root.current ? plot.polyline(root.current.samples, "pctNoWeather") : []
                        }
                    }
                }

                // The neon curve, with its bloom behind it.
                Instrument.Glow {
                    anchors.fill: traces
                    source: traces
                    strength: root.style.glow
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
                                strokeColor: trace.index === 0 ? root.style.yellow
                                    : (trace.index === 1 ? root.style.cyan : root.style.muted)
                                strokeWidth: root.style.trace
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

                // The applied-now marker on the curve.
                Rectangle {
                    visible: root.nowLevel >= 0
                    x: plot.sx(root.now) - width / 2
                    y: plot.sy(root.nowLevel) - height / 2
                    width: root.style.snap(7)
                    height: width
                    rotation: 45
                    color: root.style.yellow
                    border.width: root.style.hairline
                    border.color: root.style.plot
                }

                // The NOW cursor, red, with its tag at the top.
                Rectangle {
                    visible: root.nowInDay
                    x: plot.sx(root.now) - width / 2
                    y: 0
                    width: root.style.trace
                    height: plot.height
                    color: root.style.red
                }
                Rectangle {
                    visible: root.nowInDay
                    x: Math.max(0, Math.min(plot.width - width, plot.sx(root.now) - width / 2))
                    y: 0
                    width: nowText.implicitWidth + root.style.gap * 2
                    height: nowText.implicitHeight + root.style.hairline * 4
                    color: root.style.red

                    Text {
                        id: nowText

                        anchors.centerIn: parent
                        text: i18ndc(root.domain, "Cursor tag: the current time", "NOW %1", Format.timeText(root.now))
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.onRed
                    }
                }

                // The hover cursor: never inside a glow source.
                Rectangle {
                    visible: root.scrubbing
                    x: root.style.snap(hover.mouseX)
                    y: 0
                    width: root.style.hairline
                    height: plot.height
                    color: root.style.ink
                }
                Rectangle {
                    visible: root.scrubbing && root.readTarget >= 0
                    x: root.style.snap(hover.mouseX) - width / 2
                    y: plot.sy(root.readTarget) - height / 2
                    width: root.style.snap(5)
                    height: width
                    rotation: 45
                    color: root.style.ink
                }

                // Corner brackets of the plot.
                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer
                    ShapePath {
                        strokeColor: root.style.yellow
                        strokeWidth: root.style.outline
                        fillColor: "transparent"
                        PathSvg {
                            path: G.corners(plot.width, plot.height, root.style.snap(root.style.unit * 0.9),
                                            root.style.outline / 2)
                        }
                    }
                }

                MouseArea {
                    id: hover

                    readonly property bool active: containsMouse && root.available
                    readonly property real time: root.t0
                        + Math.max(0, Math.min(1, mouseX / Math.max(1, width))) * (root.t1 - root.t0)

                    anchors.fill: parent
                    hoverEnabled: true
                    // Looking only: a click here must never change brightness.
                    acceptedButtons: Qt.NoButton
                }
            }

            // Y axis in the gutter.
            Repeater {
                model: [0, 50, 100]

                delegate: Text {
                    required property int modelData

                    x: plot.width + root.style.gap
                    y: Math.max(0, Math.min(plotBox.height - height, plot.yAt(modelData) - height / 2))
                    text: String(modelData)
                    textFormat: Text.PlainText
                    font: root.style.tinyFont
                    color: root.style.muted
                }
            }

            // X axis: a tick ruler, long ticks every six hours, labels under them.
            Shape {
                x: 0
                y: plot.height
                width: plot.width
                height: plotBox.axisH
                ShapePath {
                    strokeColor: root.style.rule
                    strokeWidth: root.style.hairline
                    fillColor: "transparent"
                    PathSvg { path: G.ticks(plot.width, root.hours, 6, 2, 4) }
                }
            }
            Repeater {
                model: Math.floor(root.hours / 6) + 1

                delegate: Text {
                    required property int index

                    x: Math.max(0, Math.min(plotBox.width - width, plot.sx(root.t0 + index * 6 * 3600) - width / 2))
                    y: plot.height + root.style.hairline * 4
                    text: String(index * 6).padStart(2, "0")
                    textFormat: Text.PlainText
                    font: root.style.tinyFont
                    color: root.style.muted
                }
            }
        }
    }
}
