/*
 * Light cycle, Wii style (the hero): today's brightness policy on a channel
 * tile. The target now is the big rounded numeral; the curve is Wii blue over a
 * light blue area, the night is a pale band, and a round white "now" marker with
 * a blue ring carries a small sun. Hovering the chart reads any time of the day.
 * It never sets brightness.
 *
 *   ( 41 %  Target now )                       [MI] [LEN]
 *   ┌──────────────────────────────────────────────────────┐
 *   │ night ░ ╱‾‾‾‾‾‾‾‾‾‾╲  ◉ now               ░ night    │
 *   └──────────────────────────────────────────────────────┘
 *    00:00        06:00        12:00        18:00      24:00
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property WiiStyle style: WiiStyle {
        theme: root.theme
    }

    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves`: the display whose curve is drawn. */
    property int selected: 0
    readonly property var current: curves.length > 0
        ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current ? current.samples : []
    readonly property real t0: samples.length > 0 ? samples[0].t : 0
    readonly property real t1: samples.length > 0 ? samples[samples.length - 1].t : 1
    readonly property var sun: curve && curve.sun ? curve.sun : ({})
    readonly property real now: client.nowEpochS
    readonly property bool nowInDay: now >= t0 && now <= t1
    readonly property real nowTarget: Curve.valueAt(samples, now, "pct")

    /*! The time under the pointer; `hovering` false means "now". */
    property bool hovering: false
    property real hoverT: 0
    readonly property real readTarget: hovering ? Curve.valueAt(samples, hoverT, "pct") : nowTarget
    readonly property int appliedNow: appliedFor(current ? current.monitorId : "")

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    function pctText(v) {
        return v >= 0 ? String(Math.round(v)) : "—";
    }

    spacing: style.rowGap

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && readTarget >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(readTarget))
        : ""

    WiiTile {
        id: tile

        style: root.style
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 2 * tile.pad

        ColumnLayout {
            id: inner

            anchors.fill: parent
            spacing: root.style.gap

            // The hero: the target now, with the time it is read at and what is applied.
            RowLayout {
                Layout.fillWidth: true
                spacing: 0

                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: root.pctText(root.readTarget)
                    textFormat: Text.PlainText
                    font: root.style.heroFont
                    color: root.style.accentInk
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    Layout.leftMargin: root.style.gap
                    text: "%"
                    textFormat: Text.PlainText
                    font: root.style.titleFont
                    color: root.style.accentInk
                }
                ColumnLayout {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.leftMargin: root.style.gap2
                    spacing: 0

                    Text {
                        text: root.hovering
                            ? Format.timeText(root.hoverT)
                            : i18ndc(root.domain, "Brightness the curve targets at the current time", "Target now")
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: root.style.inkSoft
                    }
                    Text {
                        visible: root.appliedNow >= 0 && !root.hovering
                        text: i18ndc(root.domain, "What the daemon last wrote, in percent", "Applied %1", root.appliedNow)
                        textFormat: Text.PlainText
                        font: root.style.captionFont
                        color: root.style.inkSoft
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                // Which display's curve: one pill per display, when there are several.
                Repeater {
                    model: root.curves.length > 1 ? root.curves : []

                    delegate: WiiPill {
                        id: channel

                        required property var modelData
                        required property int index

                        Layout.alignment: Qt.AlignVCenter
                        style: root.style
                        label: Curve.shortName(modelData.monitorId)
                        variant: "outline"
                        current: index === root.selected
                        Accessible.role: Accessible.Button
                        Accessible.name: modelData.monitorId
                        Accessible.checkable: true
                        Accessible.checked: channel.current
                        Accessible.onPressAction: root.selected = channel.index

                        onClicked: root.selected = channel.index
                    }
                }
            }

            // The chart: night bands, the area and the curve, the now marker.
            Item {
                id: plot

                Layout.fillWidth: true
                Layout.preferredHeight: root.style.snap(root.style.unit * 3.2)

                function xAt(t) {
                    return (t - root.t0) / Math.max(1, root.t1 - root.t0) * width;
                }
                function yAt(pct) {
                    return (1 - Math.max(0, Math.min(100, pct)) / 100) * height;
                }
                function linePath() {
                    const out = [];
                    for (const s of root.samples) {
                        out.push(Qt.point(xAt(s.t), yAt(s.pct)));
                    }
                    return out;
                }
                function areaPath() {
                    const out = linePath();
                    if (out.length > 0) {
                        out.push(Qt.point(width, height));
                        out.push(Qt.point(0, height));
                    }
                    return out;
                }

                // Night is a pale band, twilight a lighter one.
                Repeater {
                    model: root.sun.sunrise > 0 && root.sun.sunset > 0 ? [
                        { from: root.t0, to: root.sun.dawn > 0 ? root.sun.dawn : root.sun.sunrise, shade: 1.0 },
                        { from: root.sun.dawn > 0 ? root.sun.dawn : root.sun.sunrise, to: root.sun.sunrise, shade: 0.5 },
                        { from: root.sun.sunset, to: root.sun.dusk > 0 ? root.sun.dusk : root.sun.sunset, shade: 0.5 },
                        { from: root.sun.dusk > 0 ? root.sun.dusk : root.sun.sunset, to: root.t1, shade: 1.0 },
                    ] : []

                    delegate: Rectangle {
                        required property var modelData

                        x: plot.xAt(modelData.from)
                        width: Math.max(0, plot.xAt(modelData.to) - x)
                        height: plot.height
                        color: root.style.alpha(root.style.ink, 0.07 * modelData.shade)
                    }
                }

                // Quarter gridlines, hairline.
                Repeater {
                    model: [25, 50, 75]

                    delegate: Rectangle {
                        required property int modelData

                        y: plot.yAt(modelData)
                        width: plot.width
                        height: root.style.line
                        color: root.style.alpha(root.style.ink, 0.08)
                    }
                }

                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeWidth: -1
                        fillColor: root.style.alpha(root.style.sky, 0.6)

                        PathPolyline {
                            path: root.samples.length > 1 ? plot.areaPath() : []
                        }
                    }
                    ShapePath {
                        strokeColor: root.style.accentLine
                        strokeWidth: root.style.stroke
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        joinStyle: ShapePath.RoundJoin

                        PathPolyline {
                            path: root.samples.length > 1 ? plot.linePath() : []
                        }
                    }
                }

                // The "now" marker: a round white disc, a blue ring, a small sun.
                Item {
                    id: nowMarker

                    readonly property real d: root.style.snap(root.style.unit * 1.2)

                    visible: root.nowInDay && !root.hovering && root.nowTarget >= 0
                    width: d
                    height: d
                    x: Math.max(0, Math.min(plot.width - d, plot.xAt(root.now) - d / 2))
                    y: Math.max(0, Math.min(plot.height - d, plot.yAt(root.nowTarget) - d / 2))

                    Rectangle {
                        anchors.fill: parent
                        radius: width / 2
                        antialiasing: true
                        color: root.style.paper
                        border.width: root.style.line2
                        border.color: root.style.accentLine
                    }
                    WeatherGlyph {
                        anchors.centerIn: parent
                        size: nowMarker.d * 0.62
                        condition: "clear"
                        night: false
                        glyphStyle: "solid"
                        palette: root.style.glyphPalette
                    }
                }

                // The hover read-out: a small dot on the curve at the pointer's time.
                Rectangle {
                    visible: root.hovering && root.readTarget >= 0
                    width: root.style.snap(root.style.unit * 0.5)
                    height: width
                    radius: width / 2
                    x: plot.xAt(root.hoverT) - width / 2
                    y: plot.yAt(root.readTarget) - height / 2
                    antialiasing: true
                    color: root.style.accentLine
                    border.width: root.style.line2
                    border.color: root.style.paper
                }

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onEntered: root.hovering = true
                    onExited: root.hovering = false
                    onPositionChanged: mouse => {
                        const f = Math.max(0, Math.min(1, mouse.x / Math.max(1, plot.width)));
                        root.hoverT = root.t0 + f * (root.t1 - root.t0);
                        root.hovering = true;
                    }
                }
            }

            // The time scale under the chart: five labels, the ends flush.
            Item {
                id: axis

                Layout.fillWidth: true
                implicitHeight: root.style.sizeCaption * 1.4

                Repeater {
                    model: [0, 0.25, 0.5, 0.75, 1]

                    delegate: Text {
                        required property real modelData
                        required property int index

                        y: 0
                        x: index === 0 ? 0 : (index === 4 ? axis.width - width : modelData * axis.width - width / 2)
                        text: Format.timeText(root.t0 + modelData * (root.t1 - root.t0))
                        textFormat: Text.PlainText
                        font: root.style.captionFont
                        color: root.style.inkSoft
                    }
                }
            }
        }
    }
}
