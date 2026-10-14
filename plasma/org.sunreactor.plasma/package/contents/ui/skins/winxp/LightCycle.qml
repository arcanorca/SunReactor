/*
 * Light cycle, Windows XP: today's brightness curve on graph paper inside a
 * sunken white client edge. One curve in Luna blue for the picked display, one
 * per display under All. The red needle is now and the dot on the curve is the
 * target now. The caption row holds the display picker and that target.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../instrument/Curve.js" as Curve

WinWindow {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    style: ui
    width: parent ? parent.width : 0

    readonly property string domain: settings.domain
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0 && curves[0].samples.length > 1
    title: i18ndc(domain, "Window title of the light cycle chart", "Light cycle")

    property int selected: 0
    readonly property bool showAll: selected < 0 && curves.length > 1
    readonly property var current: available ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property real t0: available ? curves[0].samples[0].t : 0
    readonly property real t1: available ? curves[0].samples[curves[0].samples.length - 1].t : 1
    readonly property real gridStart: Math.ceil(t0 / 7200) * 7200
    readonly property real now: client.nowEpochS
    readonly property real targetNow: current ? Curve.valueAt(current.samples, now, "pct") : -1
    readonly property var sun: curve && curve.sun ? curve.sun : ({})
    readonly property var curveColors: [ui.accent, ui.green, ui.orange, ui.danger]

    // The caption row: the display picker (when there are several), then the target now.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.ui.gap

        Repeater {
            model: root.curves.length > 1 ? root.curves.length + 1 : 0

            delegate: WinButton {
                id: pick

                required property int index

                readonly property bool isAll: index === root.curves.length
                readonly property int slot: isAll ? -1 : index

                style: root.ui
                text: isAll ? i18ndc(root.domain, "Light cycle selector: every display", "All")
                    : Curve.shortName(root.curves[index].monitorId)
                current: root.selected === slot
                onClicked: root.selected = slot
            }
        }
        Item {
            Layout.fillWidth: true
        }
        Text {
            visible: root.targetNow >= 0
            text: root.targetNow >= 0
                ? i18ndc(root.domain, "Light cycle caption: the brightness the curve targets now",
                         "Target now %1%", Math.round(root.targetNow))
                : ""
            textFormat: Text.PlainText
            font: root.ui.smallBoldFont
            color: root.ui.ink
        }
    }

    // The chart: a client edge, graph paper, the curves, the sun and now.
    Item {
        id: plot

        Layout.fillWidth: true
        implicitHeight: root.ui.snap(root.ui.sizeBody * 4.7)

        readonly property real inset: root.ui.snap(root.ui.gap * 0.5 + root.ui.line)

        Rectangle {
            anchors.fill: parent
            color: root.ui.white
            border.width: root.ui.line
            border.color: root.ui.clientEdge
        }

        Item {
            id: area

            x: plot.inset
            y: plot.inset
            width: plot.width - 2 * plot.inset
            height: plot.height - 2 * plot.inset
            clip: true

            function xAt(t) {
                return (t - root.t0) / Math.max(1, root.t1 - root.t0) * width;
            }
            function yAt(pct) {
                return (1 - Math.max(0, Math.min(100, pct)) / 100) * height;
            }

            // Graph paper: a line every two hours, and at 25, 50 and 75 percent.
            Repeater {
                model: root.available ? Math.max(0, Math.floor((root.t1 - root.gridStart) / 7200) + 1) : 0

                delegate: Rectangle {
                    required property int index

                    x: root.ui.snap(area.xAt(root.gridStart + index * 7200))
                    y: 0
                    width: root.ui.hairline
                    height: area.height
                    color: root.ui.gridline
                }
            }
            Repeater {
                model: [25, 50, 75]

                delegate: Rectangle {
                    required property int modelData

                    x: 0
                    y: root.ui.snap(area.yAt(modelData))
                    width: area.width
                    height: root.ui.hairline
                    color: root.ui.gridline
                }
            }

            // Sunrise, solar noon and sunset.
            Repeater {
                model: root.available ? [root.sun.sunrise, root.sun.solarNoon, root.sun.sunset].filter(t => t > 0) : []

                delegate: Rectangle {
                    required property real modelData

                    x: root.ui.snap(area.xAt(modelData))
                    y: 0
                    width: root.ui.hairline
                    height: area.height
                    color: root.ui.etchDark
                }
            }

            // The curves: one for the picked display, or one per display under All.
            Repeater {
                model: !root.available ? [] : (root.showAll ? root.curves : (root.current ? [root.current] : []))

                delegate: Shape {
                    id: trace

                    required property var modelData
                    required property int index

                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeColor: root.curveColors[trace.index % root.curveColors.length]
                        strokeWidth: root.ui.line * 2
                        fillColor: "transparent"
                        joinStyle: ShapePath.RoundJoin
                        capStyle: ShapePath.RoundCap
                        PathPolyline {
                            path: trace.modelData.samples.map(s => Qt.point(area.xAt(s.t), area.yAt(s.pct)))
                        }
                    }
                }
            }

            // Now: a red needle, and the dot where the picked curve is now.
            Rectangle {
                visible: root.available && root.now >= root.t0 && root.now <= root.t1
                x: root.ui.snap(area.xAt(root.now))
                y: 0
                width: root.ui.line
                height: area.height
                color: root.ui.danger
            }
            Rectangle {
                readonly property real dot: root.ui.snap(root.ui.sizeBody * 0.5)

                visible: root.targetNow >= 0 && root.now >= root.t0 && root.now <= root.t1
                x: area.xAt(root.now) - dot / 2
                y: area.yAt(root.targetNow) - dot / 2
                width: dot
                height: dot
                radius: dot / 2
                color: root.ui.white
                border.width: root.ui.line * 2
                border.color: root.ui.accent
            }
        }
    }
}
