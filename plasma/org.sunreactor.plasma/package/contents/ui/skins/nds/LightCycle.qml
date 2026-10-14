/*
 * Nintendo DS top screen, brightness page: the day's curve on a white card over
 * the sky wallpaper, the target as a large rounded percentage, the display's
 * name (a click cycles the displays and All) and the console's status bar. The
 * hinge sits under the screen. Hover scrubs the curve (time -> target); it never
 * sets brightness. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../instrument/Curve.js" as Curve
import "../../Format.js" as Format

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0 && curves[0].samples.length > 1

    property int selected: 0
    readonly property bool showAll: selected < 0 && curves.length > 1
    readonly property var current: curves.length > 0 ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current && !showAll ? current.samples : []
    readonly property var sun: available && curve.sun ? curve.sun : ({})
    readonly property real now: client.nowEpochS
    readonly property real t0: available ? curves[0].samples[0].t : 0
    readonly property real t1: available ? curves[0].samples[curves[0].samples.length - 1].t : 1
    readonly property real targetNow: samples.length > 1 ? Curve.valueAt(samples, now, "pct") : -1
    property real scrubTime: -1
    readonly property bool scrubbing: scrubTime >= 0 && samples.length > 1
    readonly property real heroValue: scrubbing ? Curve.valueAt(samples, scrubTime, "pct") : targetNow
    readonly property int applied: current && !showAll ? appliedFor(current.monitorId) : -1
    readonly property string title: showAll
        ? i18ndc(domain, "Show the curves of every display", "All")
        : (current ? String(current.monitorId) : "")
    readonly property string heroText: heroValue >= 0 ? Math.round(heroValue) + "%" : "--%"

    // Glass layout, in grid units from the top of the glass.
    readonly property real gu: style.gu
    readonly property real pad: Math.round(gu * 0.75)
    readonly property real glassW: root.width - 2 * style.bezel
    readonly property real cardTop: Math.round(gu * 4.75)
    readonly property real cardH: Math.round(gu * 3.5)
    readonly property real glassH: Math.round(cardTop + cardH + gu * 0.4)
    // The plot inside the card: an inset, and a row of hour labels at its foot.
    readonly property real plotX0: pad + gu * 0.6
    readonly property real plotX1: glassW - pad - gu * 0.6
    readonly property real plotY0: cardTop + gu * 0.45
    readonly property real plotY1: cardTop + cardH - gu * 0.95

    readonly property var linePath: samples.length > 1 ? polyline(samples, "pct") : []
    readonly property var clearPath: samples.length > 1 ? polyline(samples, "pctNoWeather") : []
    readonly property var areaPath: samples.length > 1
        ? polyline(samples, "pct").concat([
            Qt.point(xAt(samples[samples.length - 1].t), plotY1),
            Qt.point(xAt(samples[0].t), plotY1)])
        : []
    readonly property real riseX: samples.length > 1 && sun.sunrise > 0
        ? Math.max(plotX0, Math.min(plotX1, xAt(sun.sunrise))) : plotX0
    readonly property real setX: samples.length > 1 && sun.sunset > 0
        ? Math.max(plotX0, Math.min(plotX1, xAt(sun.sunset))) : plotX1

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    function xAt(t) {
        return plotX0 + (t - t0) / Math.max(1, t1 - t0) * (plotX1 - plotX0);
    }

    function timeAt(x) {
        const t = t0 + (x - plotX0) / Math.max(1, plotX1 - plotX0) * (t1 - t0);
        return Math.max(t0, Math.min(t1, t));
    }

    function yAt(pct) {
        return plotY1 - Math.max(0, Math.min(100, pct)) / 100 * (plotY1 - plotY0);
    }

    function polyline(list, field) {
        return list.map(s => Qt.point(xAt(s.t), yAt(s[field])));
    }

    /*! Local midnight of the curve's first day, plus `hour` hours (epoch seconds). */
    function hourTime(hour) {
        const day = new Date(t0 * 1000);
        day.setHours(0, 0, 0, 0);
        return day.getTime() / 1000 + hour * 3600;
    }

    /*! The header click: displays 0..n-1, then All (-1), wrapping. */
    function cycle() {
        if (curves.length < 2) {
            return;
        }
        const order = curves.map((c, i) => i).concat([-1]);
        const i = Math.max(0, order.indexOf(selected));
        selected = order[(i + 1) % order.length];
    }

    implicitHeight: screen.implicitHeight

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && targetNow >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now", "%1: %2% now",
                 current.monitorId, Math.round(targetNow))
        : ""

    NdsScreen {
        id: screen

        width: parent.width
        theme: root.theme
        style: root.style
        wallpaper: "sky"
        hinge: true
        glassHeight: root.glassH

        NdsStatusBar {
            x: root.pad
            y: Math.round(root.gu * 0.35)
            width: root.glassW - 2 * root.pad
            theme: root.theme
            style: root.style
            client: root.client
        }

        // The display's name; a click cycles through the displays and All.
        Text {
            id: titleText

            x: root.pad
            y: Math.round(root.gu * 1.5)
            width: root.glassW - 2 * root.pad
            elide: Text.ElideRight
            text: root.title
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 0.9)
            color: root.style.ink
        }
        MouseArea {
            x: root.pad
            y: Math.round(root.gu * 1.45)
            width: root.glassW - 2 * root.pad
            height: Math.round(root.gu * 1.2)
            cursorShape: root.curves.length > 1 ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: root.cycle()
        }

        // The target now (or the point the scrub cursor shows).
        Text {
            id: hero

            x: root.pad
            y: Math.round(root.gu * 2.6)
            text: root.showAll ? "--%" : root.heroText
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 1.75)
            color: root.style.ink
        }
        Text {
            x: hero.x + hero.implicitWidth + Math.round(root.gu * 0.6)
            y: Math.round(root.gu * 2.85)
            text: root.scrubbing ? Format.timeText(root.scrubTime)
                : i18ndc(root.domain, "The curve's brightness right now", "Now")
            font.family: root.style.family
            font.pixelSize: Math.round(root.gu * 0.66)
            color: root.style.muted
        }
        Text {
            x: hero.x + hero.implicitWidth + Math.round(root.gu * 0.6)
            y: Math.round(root.gu * 3.7)
            visible: !root.scrubbing && root.applied >= 0
            text: i18ndc(root.domain, "Brightness the display has right now", "Set %1%", root.applied)
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 0.8)
            color: root.style.blue
        }

        // The chart card: white, with the curve, the night and the hour axis.
        Rectangle {
            id: card

            x: root.pad
            y: root.cardTop
            width: root.glassW - 2 * root.pad
            height: root.cardH
            radius: 10
            color: root.style.white
            border.width: 1
            border.color: root.style.line
        }
        Rectangle {
            visible: root.samples.length > 1 && root.sun.sunrise > 0
            x: root.plotX0
            y: root.plotY0
            width: Math.max(0, root.riseX - root.plotX0)
            height: root.plotY1 - root.plotY0
            color: root.style.alpha(root.style.ink, 0.07)
        }
        Rectangle {
            visible: root.samples.length > 1 && root.sun.sunset > 0
            x: root.setX
            y: root.plotY0
            width: Math.max(0, root.plotX1 - root.setX)
            height: root.plotY1 - root.plotY0
            color: root.style.alpha(root.style.ink, 0.07)
        }

        Shape {
            x: 0
            y: 0
            width: root.glassW
            height: root.glassH
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "transparent"
                strokeWidth: 0
                fillColor: root.style.alpha(root.style.blue, 0.2)
                PathPolyline { path: root.areaPath }
            }
            ShapePath {
                strokeColor: root.style.muted
                strokeWidth: 1.5
                fillColor: "transparent"
                strokeStyle: ShapePath.DashLine
                dashPattern: [4, 4]
                PathPolyline { path: root.clearPath }
            }
            ShapePath {
                strokeColor: root.style.blue
                strokeWidth: 2.5
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin
                PathPolyline { path: root.linePath }
            }
        }

        // Now (or the scrub point): a line down the plot and a dot on the curve.
        Rectangle {
            visible: root.samples.length > 1
            x: Math.round(root.xAt(root.scrubbing ? root.scrubTime : root.now))
            y: root.plotY0
            width: 1
            height: root.plotY1 - root.plotY0
            color: root.style.alpha(root.style.ink, 0.35)
        }
        Rectangle {
            visible: root.samples.length > 1 && root.heroValue >= 0
            x: Math.round(root.xAt(root.scrubbing ? root.scrubTime : root.now)) - 5
            y: Math.round(root.yAt(root.heroValue)) - 5
            width: 10
            height: 10
            radius: 5
            color: root.style.white
            border.width: 2
            border.color: root.style.blue
        }

        Repeater {
            model: [0, 6, 12, 18, 24]

            delegate: Text {
                required property int modelData

                readonly property real at: root.hourTime(modelData)

                visible: root.samples.length > 1 && at >= root.t0 && at <= root.t1
                x: Math.round(root.xAt(at) - implicitWidth / 2)
                y: Math.round(root.cardTop + root.cardH - root.gu * 0.9)
                text: String(modelData)
                font.family: root.style.family
                font.weight: Font.Bold
                font.pixelSize: Math.round(root.gu * 0.6)
                color: root.style.muted
            }
        }

        MouseArea {
            x: root.pad
            y: root.cardTop
            width: card.width
            height: card.height
            hoverEnabled: true
            acceptedButtons: Qt.NoButton
            onPositionChanged: mouse => root.scrubTime = root.timeAt(mouse.x + x)
            onExited: root.scrubTime = -1
        }
    }
}
