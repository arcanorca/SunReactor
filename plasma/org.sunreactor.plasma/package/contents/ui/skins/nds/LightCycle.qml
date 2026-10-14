/*
 * Nintendo DS top screen, brightness page: today's curve as a pixel line chart
 * with dither under it, the target as a chunky hero percentage in the bitmap
 * face, and a header strip with the display's name (a click cycles the
 * displays and All), the clock and the daemon's battery lamp. The hinge closes
 * the screen. Hover scrubs the curve (time -> target); it never sets brightness.
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel"
import "../pixel/PixelFormat.js" as PixelFormat
import "Nds.js" as Nds

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0 && curves[0].samples.length > 1

    property int selected: 0
    readonly property bool showAll: selected < 0 && curves.length > 1
    readonly property var current: curves.length > 0 ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property real now: client.nowEpochS
    readonly property var sun: available && curve.sun ? curve.sun : ({})
    readonly property real targetNow: current ? plot.valueAt(now) : -1
    readonly property real costNow: current && targetNow >= 0 ? plot.valueAt(now, true) - targetNow : 0
    property real scrubTime: 0
    readonly property var clock: PixelFormat.timeParts(client.nowEpochS)
    readonly property string title: showAll
        ? i18ndc(domain, "Show the curves of every display", "All")
        : (current ? PixelFormat.shortName(current.monitorId) : "")

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
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

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && targetNow >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now", "%1: %2% now",
                 current.monitorId, Math.round(targetNow))
        : ""

    NdsLid {
        id: surface

        width: parent.width
        theme: root.theme
        grid: root.theme.grid
        fillsWidth: true
        hinge: true
        lidHeight: Nds.TOP_H - Nds.HINGE
        regionName: "curve"

        // Header strip: the display's name, the clock and the battery lamp.
        Rectangle {
            x: 2
            y: 2
            width: surface.artWidth - 4
            height: 10
            color: root.theme.bg
            antialiasing: false
        }
        Rectangle {
            x: 2
            y: 11
            width: surface.artWidth - 4
            height: 1
            color: root.theme.borderInactive
            antialiasing: false
        }
        PixelLabel {
            id: titleLabel

            theme: root.theme
            x: 5
            y: 3
            face: "bitmap"
            capitals: true
            color: pstyle.ink
            text: root.title
            maximumArtWidth: Math.max(0, clockLabel.x - x - 8)
        }
        MouseArea {
            x: 2
            y: 2
            width: surface.artWidth - 4
            height: 10
            cursorShape: root.curves.length > 1 ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: root.cycle()
        }
        PixelLabel {
            id: clockLabel

            theme: root.theme
            x: surface.artWidth - 5 - artWidth
            y: 4
            face: "small"
            capitals: true
            color: pstyle.ink
            text: root.clock.hm + root.clock.meridiem
        }
        Repeater {
            model: Nds.battery([], clockLabel.x - 15, 4, root.client.isConnected ? 3 : 0,
                               { frame: root.theme.fg, lit: root.theme.stateColor(root.client.mode),
                                 faint: root.theme.borderInactive })
            delegate: Rectangle {
                required property var modelData

                x: modelData.x
                y: modelData.y
                width: modelData.w
                height: modelData.h
                color: modelData.c
                antialiasing: false
            }
        }

        // The hero: the target now (or the point the scrub cursor shows).
        readonly property bool scrubbing: scrub.containsMouse && root.current !== null
        readonly property real shownPct: scrubbing ? plot.valueAt(root.scrubTime)
                                                   : (root.showAll ? -1 : root.targetNow)

        BitmapText {
            id: hero

            theme: root.theme
            inSurface: true
            magnification: 2
            x: 5
            y: 15
            color: pstyle.ink
            text: surface.shownPct >= 0 ? Math.round(surface.shownPct) + "%" : "--%"
        }
        // The readout chip: "now" and the applied level, on white. The caption is
        // the tiny face (5 rows) so both lines fit the 18-row chip with a row of air
        // above and below; the small face's cell is 9 rows tall, so two of them
        // cannot stack inside a chip this size.
        readonly property int applied: root.current && !root.showAll ? root.appliedFor(root.current.monitorId) : -1
        Repeater {
            model: Nds.box([], hero.x + hero.artWidth + 6, 13, 52, 18, root.theme.bg, root.theme.borderInactive)
            delegate: Rectangle {
                required property var modelData

                x: modelData.x
                y: modelData.y
                width: modelData.w
                height: modelData.h
                color: modelData.c
                antialiasing: false
            }
        }
        PixelLabel {
            theme: root.theme
            x: hero.x + hero.artWidth + 9
            y: 15
            face: "tiny"
            capitals: true
            color: pstyle.dim
            text: surface.scrubbing ? PixelFormat.time(root.scrubTime)
                : i18ndc(root.domain, "The curve's brightness right now", "Now")
        }
        PixelLabel {
            theme: root.theme
            x: hero.x + hero.artWidth + 9
            y: 22
            face: "small"
            capitals: true
            color: pstyle.ink
            text: surface.scrubbing || surface.applied < 0 ? ""
                : i18ndc(root.domain, "Brightness the display has right now", "Set %1%", surface.applied)
        }

        // The chart card: white, with the curve, its dither and the hour axis.
        Repeater {
            model: Nds.box([], 3, 32, surface.artWidth - 6, 42, root.theme.bg, root.theme.borderInactive)
            delegate: Rectangle {
                required property var modelData

                x: modelData.x
                y: modelData.y
                width: modelData.w
                height: modelData.h
                color: modelData.c
                antialiasing: false
            }
        }

        PixelCurve {
            id: plot

            x: 5
            y: 34
            width: surface.artWidth - 10
            height: 33
            artScale: 1

            samples: root.showAll ? [] : (root.current ? root.current.samples : [])
            overlays: root.showAll ? root.curves.map(c => c.samples) : []
            overlayColors: [root.theme.fg, root.theme.accent, root.theme.textMuted, root.theme.warning]
            now: root.now
            applied: root.current && !root.showAll ? root.appliedFor(root.current.monitorId) : -1
            minPct: root.current && !root.showAll ? root.current.minPct : -1
            maxPct: root.current && !root.showAll ? root.current.maxPct : -1
            sunrise: root.sun.sunrise || 0
            solarNoon: root.sun.solarNoon || 0
            sunset: root.sun.sunset || 0
            t0: root.curves.length > 0 && root.curves[0].samples.length > 0 ? root.curves[0].samples[0].t : 0
            t1: root.curves.length > 0 && root.curves[0].samples.length > 0
                ? root.curves[0].samples[root.curves[0].samples.length - 1].t : 0

            paperColor: root.theme.bg
            lineColor: pstyle.ink
            lineWeight: 1
            areaColor: root.theme.accent
            areaPattern: "bayer4"
            areaLevel: 0.45
            costColor: root.theme.warning
            costPattern: "checker"
            bandColor: root.theme.textMuted
            nightColor: root.theme.borderInactive
            nightPattern: "sparse"
            tickColor: root.theme.textMuted
            sunColor: root.theme.warning
            markerColor: root.theme.secondaryAccent
            targetColor: root.theme.accent
            appliedColor: root.theme.fg

            Rectangle {
                visible: scrub.containsMouse
                x: Math.max(0, Math.min(plot.width - 1, Math.floor(scrub.mouseX)))
                width: 1
                height: plot.height
                antialiasing: false
                color: root.theme.secondaryAccent
            }

            MouseArea {
                id: scrub

                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
                onPositionChanged: event => root.scrubTime = plot.timeAt(Math.floor(event.x))
            }
        }

        Repeater {
            model: [0, 6, 12, 18, 24]

            delegate: PixelLabel {
                required property int modelData

                theme: root.theme
                face: "tiny"
                color: pstyle.dim
                text: PixelFormat.hour(modelData)
                y: 67
                x: Math.max(5, Math.min(surface.artWidth - 5 - artWidth,
                                        plot.xAt(plot.t0 + modelData * 3600) - Math.floor(artWidth / 2)))
            }
        }
    }
}
