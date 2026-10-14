/*
 * Light cycle, PlayStation 3: today's policy curve from the daemon's day_curve,
 * as a smooth glowing white line with a soft translucent area under it, over
 * the wave backdrop. The target sits large and light beside its legend; the
 * "now" point is a bright dot with a halo. Hovering scrubs the curve (time and
 * target under the pointer) and never sets brightness.
 *
 *   TARGET · NOW                            APPLIED
 *   41%                                     39%   LEN MI
 *   ╱‾‾‾‾‾‾‾‾‾╲___________          ● (now)
 *   00        06        12        18
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Plot.js" as Plot

ColumnLayout {
    id: root

    // The PS3 side margin: the module's content keeps clear of the popup's edges.
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.leftMargin: root.style.sideMargin
    anchors.rightMargin: root.style.sideMargin

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }

    readonly property var curves: client.dayCurve && client.dayCurve.monitors ? client.dayCurve.monitors : []
    /*! Index into `curves` of the plotted display. */
    property int selected: 0
    readonly property var current: curves.length > 0
        ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current && current.samples ? current.samples : []
    readonly property real t0: samples.length > 0 ? samples[0].t : 0
    readonly property real t1: samples.length > 0 ? samples[samples.length - 1].t : 1
    readonly property real span: Math.max(1, t1 - t0)
    readonly property bool available: client.isConnected && samples.length > 1

    readonly property real now: client.nowEpochS
    readonly property bool nowInDay: now >= t0 && now <= t1
    readonly property real nowPct: Plot.valueAt(samples, now, "pct")
    readonly property int appliedNow: appliedFor(current ? current.monitorId : "")

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    // What the readout shows: the time under the pointer while scrubbing, else now.
    readonly property bool scrubbing: hover.containsMouse
    readonly property real readTime: scrubbing ? hover.time : now
    readonly property real readTarget: Plot.valueAt(samples, readTime, "pct")

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && nowPct >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(nowPct))
        : ""

    spacing: style.gap

    // --- readouts -------------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        ColumnLayout {
            spacing: 0

            Text {
                text: root.scrubbing
                    ? i18ndc(root.domain, "Readout caption, then the time under the pointer", "Cursor · %1",
                             Format.timeText(root.readTime))
                    : i18ndc(root.domain, "Readout caption: what the curve targets right now", "Target · now")
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkSoft
            }

            RowLayout {
                spacing: root.style.gap

                Text {
                    id: hero

                    text: root.readTarget >= 0 ? String(Math.round(root.readTarget)) : "--"
                    textFormat: Text.PlainText
                    font: root.style.heroFont
                    color: root.style.ink
                }

                Text {
                    Layout.alignment: Qt.AlignBottom
                    Layout.bottomMargin: root.style.snap(root.style.sizeHero * 0.12)
                    text: "%"
                    textFormat: Text.PlainText
                    font: root.style.unitFont
                    color: root.style.inkSoft
                }
            }
        }

        Item {
            Layout.fillWidth: true
        }

        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            spacing: 0

            Text {
                Layout.alignment: Qt.AlignRight
                text: i18ndc(root.domain, "Readout caption: the brightness applied now", "Applied")
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkSoft
            }

            Text {
                Layout.alignment: Qt.AlignRight
                text: root.appliedNow >= 0 ? root.appliedNow + "%" : "—"
                textFormat: Text.PlainText
                font: root.style.valueFont
                color: root.style.ink
            }

            // The displays: a short name each, the plotted one bright.
            RowLayout {
                Layout.alignment: Qt.AlignRight
                visible: root.curves.length > 1
                spacing: root.style.gap2

                Repeater {
                    model: root.curves.length

                    delegate: XmbLabel {
                        id: channel

                        required property int index

                        style: root.style
                        text: Plot.shortName(root.curves[index].monitorId)
                        selected: root.selected === index
                        textFont: root.style.legendFont
                        Accessible.name: root.curves[index].monitorId
                        onActivated: root.selected = index
                        onMoved: delta => {
                            const count = root.curves.length;
                            const next = channel.index + delta;
                            if (next >= 0 && next < count) {
                                root.selected = next;
                            }
                        }
                    }
                }
            }
        }
    }

    // --- the curve --------------------------------------------------------------

    Item {
        id: plotBox

        Layout.fillWidth: true
        implicitHeight: plot.height + root.style.sizeLegend + root.style.gap2

        Item {
            id: plot

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: root.style.snap(root.style.unit * 4.8)

            /*! The smooth curve and its area, in plot coordinates. */
            readonly property string curveD: Plot.smoothPath(
                Plot.points(root.samples, root.t0, root.span, width, height))
            readonly property string areaD: Plot.areaPath(
                Plot.points(root.samples, root.t0, root.span, width, height), height)

            // The area under the curve: a soft fade from white to nothing.
            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: "transparent"
                    fillGradient: LinearGradient {
                        x1: 0
                        y1: 0
                        x2: 0
                        y2: plot.height
                        GradientStop { position: 0; color: root.style.alpha(root.style.ink, 0.22) }
                        GradientStop { position: 1; color: root.style.alpha(root.style.ink, 0) }
                    }
                    PathSvg { path: plot.areaD }
                }
            }

            // The curve: a wide soft under-stroke for the glow, then the crisp line.
            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    fillColor: "transparent"
                    strokeColor: root.style.alpha(root.style.ink, 0.22)
                    strokeWidth: root.style.snap(root.style.line2 * 3)
                    capStyle: ShapePath.RoundCap
                    joinStyle: ShapePath.RoundJoin
                    PathSvg { path: plot.curveD }
                }
                ShapePath {
                    fillColor: "transparent"
                    strokeColor: root.style.ink
                    strokeWidth: root.style.line2
                    capStyle: ShapePath.RoundCap
                    joinStyle: ShapePath.RoundJoin
                    PathSvg { path: plot.curveD }
                }
            }

            // The baseline: a hairline.
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: root.style.hairline
                color: root.style.rule
            }

            Repeater {
                model: Plot.hourTicks(root.t0, root.span, plot.width)

                delegate: Text {
                    required property var modelData

                    x: modelData.x < 4 ? 0
                        : (modelData.x > plot.width - 4 ? plot.width - implicitWidth
                           : root.style.snap(modelData.x - implicitWidth / 2))
                    y: plot.height + root.style.gap
                    text: modelData.label
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkDim
                }
            }

            // The now point: a bright dot with a soft halo.
            Item {
                id: nowDot

                readonly property real halo: root.style.snap(root.style.unit * 0.9)
                readonly property real core: root.style.snap(root.style.unit * 0.22)

                visible: root.nowInDay && root.nowPct >= 0
                x: root.style.snap((root.now - root.t0) / root.span * plot.width)
                y: root.style.snap(plot.height * (1 - Math.max(0, Math.min(100, root.nowPct)) / 100))
                width: 0
                height: 0

                Rectangle {
                    x: -nowDot.halo
                    y: -nowDot.halo
                    width: nowDot.halo * 2
                    height: width
                    radius: width / 2
                    color: root.style.alpha(root.style.ink, 0.10)
                }
                Rectangle {
                    x: -nowDot.halo / 2
                    y: -nowDot.halo / 2
                    width: nowDot.halo
                    height: width
                    radius: width / 2
                    color: root.style.alpha(root.style.ink, 0.24)
                }
                Rectangle {
                    x: -nowDot.core
                    y: -nowDot.core
                    width: nowDot.core * 2
                    height: width
                    radius: width / 2
                    color: root.style.ink
                }
            }

            // Hover scrub: a hairline and a dot where the pointer is on the curve.
            Rectangle {
                visible: root.scrubbing && root.readTarget >= 0
                x: root.style.snap(hover.mouseX)
                y: 0
                width: root.style.line
                height: plot.height
                color: root.style.alpha(root.style.ink, 0.35)
            }
            Rectangle {
                visible: root.scrubbing && root.readTarget >= 0
                x: root.style.snap(hover.mouseX) - width / 2
                y: root.style.snap(plot.height * (1 - Math.max(0, Math.min(100, root.readTarget)) / 100)) - height / 2
                width: root.style.snap(root.style.unit * 0.3)
                height: width
                radius: width / 2
                color: root.style.ink
            }

            MouseArea {
                id: hover

                property real time: root.now

                anchors.fill: parent
                hoverEnabled: true
                function scrubTo() {
                    const f = Math.max(0, Math.min(1, mouseX / Math.max(1, plot.width)));
                    time = root.t0 + f * root.span;
                }

                onPositionChanged: scrubTo()
                onEntered: scrubTo()
            }
        }
    }
}
