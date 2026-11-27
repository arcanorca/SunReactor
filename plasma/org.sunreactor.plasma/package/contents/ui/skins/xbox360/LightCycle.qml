/*
 * Xbox 360 Light Cycle: metallic Blade card featuring the Xbox 360 Guide Sphere
 * & Ring of Light next to the hero brightness target, display blade tabs, and a
 * glowing green solar policy curve. Hover scrubs time -> target without changing brightness.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../../modules/DayCurve.js" as DayCurve

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }

    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: (client.isConnected || client.statusStale) && curves.length > 0 && curves[0].samples.length > 1

    property int selected: 0
    readonly property var current: curves.length > 0 ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current ? current.samples : []
    readonly property real t0: samples.length > 0 ? samples[0].t : 0
    readonly property real t1: samples.length > 0 ? samples[samples.length - 1].t : 1
    readonly property var sun: curve && curve.sun ? curve.sun : ({})
    readonly property real now: client.nowEpochS
    readonly property bool nowInDay: now >= t0 && now <= t1
    readonly property real nowTarget: DayCurve.valueAt(samples, now, "pct")

    property bool hovering: false
    property real hoverT: 0
    readonly property real readTarget: hovering ? DayCurve.valueAt(samples, hoverT, "pct") : nowTarget
    readonly property int appliedNow: DayCurve.appliedFor(root.client.monitors, current ? current.monitorId : "")

    readonly property string rolAsset: {
        switch (client.mode) {
        case SunReactorClient.Offline: return root.style.asset("xbox_rol_offline.png");
        case SunReactorClient.Paused: return root.style.asset("xbox_rol_paused.png");
        case SunReactorClient.Manual: return root.style.asset("xbox_rol_manual.png");
        default: return root.style.asset("xbox_rol_live.png");
        }
    }

    function pctText(v) {
        return v >= 0 ? String(Math.round(v)) : "—";
    }

    spacing: 0

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && readTarget >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(readTarget))
        : ""

    XboxCard {
        id: card

        style: root.style
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 2 * card.pad

        ColumnLayout {
            id: inner

            anchors.fill: parent
            spacing: root.style.gap

            RowLayout {
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                Layout.preferredWidth: 0
                spacing: root.style.gap

                // Xbox 360 Guide Sphere + Ring of Light
                Item {
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: root.style.snap(root.style.unit * 2.25)
                    implicitHeight: implicitWidth

                    Image {
                        anchors.fill: parent
                        source: Qt.resolvedUrl(root.style.asset("xbox_guide_orb.png"))
                        smooth: true
                        mipmap: true
                    }
                    Image {
                        anchors.fill: parent
                        source: Qt.resolvedUrl(root.rolAsset)
                        smooth: true
                        mipmap: true
                    }
                }

                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: root.pctText(root.readTarget)
                    textFormat: Text.PlainText
                    font: root.style.heroFont
                    color: root.style.greenBright
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: "%"
                    textFormat: Text.PlainText
                    font: root.style.titleFont
                    color: root.style.green
                }

                ColumnLayout {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.leftMargin: root.style.gap
                    Layout.minimumWidth: 0
                    spacing: 0

                    Text {
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        text: root.hovering
                            ? Format.timeText(root.hoverT)
                            : i18ndc(root.domain, "Brightness the curve targets at the current time", "Target now")
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: root.style.ink
                    }
                    Text {
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        visible: root.appliedNow >= 0
                        opacity: root.hovering ? 0 : 1
                        text: i18ndc(root.domain, "What the daemon last wrote, in percent", "Applied %1", root.appliedNow)
                        textFormat: Text.PlainText
                        font: root.style.captionFont
                        color: root.style.muted
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                Repeater {
                    model: root.curves.length > 1 ? root.curves : []

                    delegate: XboxPill {
                        id: channel

                        required property var modelData
                        required property int index

                        Layout.alignment: Qt.AlignVCenter
                        style: root.style
                        label: DayCurve.shortName(modelData.monitorId)
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

            // Solar policy plot inside dark carbon inset
            Item {
                id: plot

                Layout.fillWidth: true
                Layout.preferredHeight: root.style.snap(root.style.unit * 3.1)

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

                Rectangle {
                    anchors.fill: parent
                    radius: 6
                    color: "#0e1419"
                    border.width: 1
                    border.color: root.style.isDark ? "#465462" : "#6b7885"
                }

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
                        color: root.style.shadeFill(modelData.shade)
                    }
                }

                Repeater {
                    model: [25, 50, 75]

                    delegate: Rectangle {
                        required property int modelData

                        y: plot.yAt(modelData)
                        width: plot.width
                        height: root.style.line
                        color: root.style.gridRule
                    }
                }

                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeWidth: -1
                        fillGradient: LinearGradient {
                            x1: 0; y1: 0
                            x2: 0; y2: plot.height
                            GradientStop { position: 0; color: root.style.curveAreaTop }
                            GradientStop { position: 1; color: root.style.curveAreaBottom }
                        }
                        PathPolyline { path: root.samples.length > 1 ? plot.areaPath() : [] }
                    }
                    ShapePath {
                        strokeColor: root.style.curveGlow
                        strokeWidth: root.style.stroke * 2.2
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        joinStyle: ShapePath.RoundJoin
                        PathPolyline { path: root.samples.length > 1 ? plot.linePath() : [] }
                    }
                    ShapePath {
                        strokeColor: root.style.curveStroke
                        strokeWidth: root.style.stroke
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        joinStyle: ShapePath.RoundJoin
                        PathPolyline { path: root.samples.length > 1 ? plot.linePath() : [] }
                    }
                }

                // Now / scrub marker
                Image {
                    visible: root.nowInDay && !root.hovering && root.nowTarget >= 0
                    width: root.style.snap(root.style.unit * 0.9)
                    height: width
                    x: Math.max(0, Math.min(plot.width - width, plot.xAt(root.now) - width / 2))
                    y: Math.max(0, Math.min(plot.height - height, plot.yAt(root.nowTarget) - height / 2))
                    source: Qt.resolvedUrl(root.style.asset("xbox_slider_thumb.png"))
                    smooth: true
                    mipmap: true
                }

                Rectangle {
                    visible: root.hovering && root.readTarget >= 0
                    width: root.style.snap(root.style.unit * 0.45)
                    height: width
                    radius: width / 2
                    x: plot.xAt(root.hoverT) - width / 2
                    y: plot.yAt(root.readTarget) - height / 2
                    color: root.style.curveStroke
                    border.width: 1
                    border.color: root.style.hoverMarkerEdge
                }

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.NoButton
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

            Item {
                id: axis

                Layout.fillWidth: true
                Layout.bottomMargin: root.style.gap * 0.25
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
                        color: root.style.muted
                    }
                }
            }
        }
    }
}
