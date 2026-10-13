/*
 * The day curve as a smooth area chart, after Swift Charts: a 2 px accent line,
 * an area under it fading from 30% to nothing, soft grid hairlines, a point mark
 * with a white ring at now, and, while the pointer is over it, a scrub line and
 * a floating tooltip. The layers are static Shapes; only the hover layer moves,
 * and nothing animates. Reads the daemon's samples and never sets brightness.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve

Item {
    id: root

    required property MacStyle style
    /*! [{ t, pct, pctNoWeather }] in time order. */
    property var samples: []
    /*! Epoch seconds of now: where the point mark sits. */
    property real now: 0
    /*! [{ t, label }]: the labels under the plot, placed at their times. */
    property var marks: []

    implicitWidth: 160
    implicitHeight: 96

    readonly property real axisHeight: 14
    readonly property real plotHeight: Math.max(0, height - axisHeight)
    readonly property real t0: samples.length > 1 ? samples[0].t : 0
    readonly property real t1: samples.length > 1 ? samples[samples.length - 1].t : 1
    /*! The top of the y scale: at least 50%, at most 100%, fitted to the curve. */
    readonly property real yTop: {
        let high = 0;
        for (const s of samples) {
            high = Math.max(high, s.pct, s.pctNoWeather || 0);
        }
        return Math.min(100, Math.max(50, Math.ceil((high + 5) / 10) * 10));
    }

    function xOf(t) {
        return (t - t0) / Math.max(1, t1 - t0) * width;
    }
    function yOf(v) {
        return plotHeight - Math.max(0, Math.min(yTop, v)) / yTop * plotHeight;
    }

    readonly property var linePoints: samples.map(s => Qt.point(xOf(s.t), yOf(s.pct)))
    readonly property var areaPoints: linePoints.length > 0
        ? linePoints.concat([Qt.point(width, plotHeight), Qt.point(0, plotHeight)]) : []

    readonly property real nowValue: Curve.valueAt(samples, now, "pct")
    readonly property bool nowVisible: nowValue >= 0 && now >= t0 && now <= t1

    // --- hover --------------------------------------------------------------
    readonly property bool scrubbing: mouse.containsMouse && samples.length > 1
    readonly property real hoverTime: t0 + Math.max(0, Math.min(1, mouse.mouseX / Math.max(1, width))) * (t1 - t0)
    readonly property real hoverValue: Curve.valueAt(samples, hoverTime, "pct")

    // --- static layers --------------------------------------------------------
    // One horizontal hairline across the plot, `fraction` of the way down.
    component GridLine: ShapePath {
        required property real fraction

        strokeColor: root.style.gridline
        strokeWidth: root.style.hairline
        fillColor: "transparent"
        startX: 0
        startY: root.style.snap(root.plotHeight * fraction)
        PathLine {
            x: root.width
            y: root.style.snap(root.plotHeight * fraction)
        }
    }


    Shape {
        width: root.width
        height: root.plotHeight
        preferredRendererType: Shape.CurveRenderer

        // Soft hairlines at a quarter, half and three quarters of the scale.
        GridLine { fraction: 0.25 }
        GridLine { fraction: 0.5 }
        GridLine { fraction: 0.75 }

        ShapePath {
            strokeColor: root.style.gridline
            strokeWidth: root.style.hairline
            fillColor: "transparent"
            startX: 0
            startY: root.style.snap(root.plotHeight)
            PathLine {
                x: root.width
                y: root.style.snap(root.plotHeight)
            }
        }

        ShapePath {
            strokeColor: "transparent"
            strokeWidth: 0
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: 0
                y2: root.plotHeight
                GradientStop { position: 0; color: root.style.theme.alpha(root.style.accent, 0.30) }
                GradientStop { position: 1; color: root.style.theme.alpha(root.style.accent, 0) }
            }
            PathPolyline {
                path: root.areaPoints
            }
        }
    }

    Shape {
        width: root.width
        height: root.plotHeight
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.style.accent
            strokeWidth: 2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathPolyline {
                path: root.linePoints
            }
        }
    }

    // Times under the plot, each centred on its place and kept inside the chart.
    Repeater {
        model: root.marks

        delegate: Text {
            required property var modelData

            x: Math.max(0, Math.min(root.width - width, root.xOf(modelData.t) - width / 2))
            y: root.plotHeight + root.style.snap(2)
            text: modelData.label
            textFormat: Text.PlainText
            font: root.style.caption
            color: root.style.inkSecondary
        }
    }

    // --- the point at now: accent with a white ring ----------------------------
    Rectangle {
        visible: root.nowVisible
        x: root.style.snap(root.xOf(root.now) - 5)
        y: root.style.snap(root.yOf(root.nowValue) - 5)
        width: 10
        height: 10
        radius: 5
        antialiasing: true
        color: root.style.accent
        border.width: 2
        border.color: root.style.ink
    }

    // --- the hover layer ----------------------------------------------------------
    Rectangle {
        id: scrubLine

        visible: root.scrubbing && root.hoverValue >= 0
        x: root.style.snap(mouse.mouseX)
        y: 0
        width: root.style.hairline
        height: root.plotHeight
        color: root.style.theme.alpha(root.style.ink, 0.25)
    }

    Rectangle {
        visible: scrubLine.visible
        x: root.style.snap(mouse.mouseX - 4)
        y: root.style.snap(root.yOf(root.hoverValue) - 4)
        width: 8
        height: 8
        radius: 4
        antialiasing: true
        color: root.style.accent
        border.width: 2
        border.color: root.style.ink
    }

    Rectangle {
        id: tip

        visible: scrubLine.visible
        width: tipText.implicitWidth + 12
        height: tipText.implicitHeight + 6
        radius: 8
        antialiasing: true
        color: root.style.tooltip
        border.width: root.style.hairline
        border.color: root.style.tooltipEdge
        x: Math.max(0, Math.min(root.width - width, mouse.mouseX - width / 2))
        // Above the point, or below it when there is no room above.
        y: {
            const above = root.yOf(root.hoverValue) - height - 10;
            return above >= 0 ? above : root.yOf(root.hoverValue) + 10;
        }

        Text {
            id: tipText

            anchors.centerIn: parent
            text: Format.timeText(root.hoverTime) + "  " + Math.round(root.hoverValue) + "%"
            textFormat: Text.PlainText
            font: root.style.captionMedium
            color: root.style.ink
        }
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
    }
}
