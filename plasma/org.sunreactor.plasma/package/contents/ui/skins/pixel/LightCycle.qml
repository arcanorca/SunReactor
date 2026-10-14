/*
 * Pixel LightCycle (the hero): today's policy curve rasterised into the art
 * grid by PixelCurve - stepped line, patterned area, the weather-cost band
 * between pct_no_weather and pct, min/max limit lines, night shading, sun and
 * hour ticks, now marker with target and applied level - under a readout in
 * the hardware's digits. Display tabs when there are two or more (plus All).
 * Hover scrubs (time -> target); it never sets brightness.
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "PixelFormat.js" as PixelFormat
import "Sprites.js" as Sprites

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

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    /*! Tabs run display 0..n-1, then All (-1). */
    function moveTab(delta) {
        const order = curves.map((c, i) => i).concat([-1]);
        const i = Math.max(0, order.indexOf(selected));
        selected = order[Math.max(0, Math.min(order.length - 1, i + delta))];
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

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "curve"

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the day curve panel", "Today")
            contentHeight: tabs.height + readout.height + 2 + plot.height + 1 + axis.height

            // Display tabs.
            Row {
                id: tabs
                y: 0
                spacing: 3
                height: root.curves.length > 1 ? 13 : 0
                visible: root.curves.length > 1

                Repeater {
                    model: root.curves.length > 1 ? root.curves.length + 1 : 0

                    delegate: PixelButton {
                        id: tab
                        required property int index
                        readonly property bool isAll: index === root.curves.length
                        readonly property int value: isAll ? -1 : index

                        theme: root.theme
                        style: pstyle
                        variant: "tab"
                        checked: root.selected === value
                        text: isAll ? i18ndc(root.domain, "Show the curves of every display", "All")
                                    : PixelFormat.shortName(root.curves[index].monitorId)
                        accessibleName: isAll ? text : root.curves[index].monitorId
                        onClicked: root.selected = value
                        Keys.onLeftPressed: root.moveTab(-1)
                        Keys.onRightPressed: root.moveTab(1)
                    }
                }
            }

            // Readout: hero digits, then what they mean; weather cost on the right.
            Item {
                id: readout
                y: tabs.height
                width: panel.contentWidth
                height: hero.height

                readonly property bool scrubbing: scrub.containsMouse && root.current !== null
                readonly property real shownPct: scrubbing ? plot.valueAt(root.scrubTime)
                                                          : (root.showAll ? -1 : root.targetNow)

                PixelDigits {
                    id: hero
                    theme: root.theme
                    style: pstyle
                    size: 2
                    color: pstyle.ink
                    text: readout.shownPct >= 0 ? Math.round(readout.shownPct) + "%" : "--%"
                }
                PixelLabel {
                    theme: root.theme
                    x: hero.width + 5
                    y: hero.height - capHeight - 9
                    face: pstyle.bodyFace
                    capitals: true
                    color: pstyle.dim
                    stipple: pstyle.stippleDim
                    text: readout.scrubbing ? PixelFormat.time(root.scrubTime)
                        : i18ndc(root.domain, "The curve's brightness right now", "Now")
                }
                PixelLabel {
                    readonly property int applied: root.current && !root.showAll ? root.appliedFor(root.current.monitorId) : -1
                    theme: root.theme
                    x: hero.width + 5
                    y: hero.height - capHeight
                    face: pstyle.bodyFace
                    capitals: true
                    color: pstyle.dim
                    stipple: pstyle.stippleDim
                    maximumArtWidth: Math.max(0, cost.x - x - 3)
                    text: readout.scrubbing || applied < 0 ? ""
                        : i18ndc(root.domain, "Brightness the display has right now", "Set %1%", applied)
                }

                // What the weather takes away right now.
                Row {
                    id: cost
                    visible: !root.showAll && root.costNow >= 1 && !readout.scrubbing
                    x: panel.contentWidth - width
                    y: hero.height - 8
                    spacing: 2
                    PixelSprite {
                        source: Sprites.symbolAtlas()
                        frame: Sprites.symbol("drop")
                        frameWidth: 8
                        frameHeight: 8
                        colors: { const c = pstyle.spriteColors.slice(); c[7] = pstyle.cost; c[1] = pstyle.paper; c[8] = "transparent"; return c; }
                    }
                    PixelLabel {
                        theme: root.theme
                        y: 1
                        face: pstyle.bodyFace
                        color: pstyle.cost
                        text: i18ndc(root.domain, "Brightness the weather takes away, compact", "−%1%",
                                     Math.round(root.costNow))
                    }
                }
            }

            PixelCurve {
                id: plot
                y: readout.y + readout.height + 2
                width: panel.contentWidth
                height: 20
                artScale: 1

                samples: root.showAll ? [] : (root.current ? root.current.samples : [])
                overlays: root.showAll ? root.curves.map(c => c.samples) : []
                overlayColors: [pstyle.ink, pstyle.hot, pstyle.dim, pstyle.cost]
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

                paperColor: pstyle.paper
                lineColor: pstyle.kind === "neon" || pstyle.kind === "amiga" ? pstyle.hot : pstyle.ink
                lineWeight: pstyle.kind === "dots" ? 2 : 1
                areaColor: pstyle.areaColor
                areaPattern: pstyle.areaPattern
                areaLevel: 0.5
                costColor: pstyle.cost
                costPattern: pstyle.kind === "mac" || pstyle.kind === "dmg" ? "hatch" : "checker"
                bandColor: pstyle.dim
                nightColor: pstyle.ghost
                nightPattern: pstyle.kind === "dots" ? "none" : "sparse"
                tickColor: pstyle.dim
                sunColor: pstyle.kind === "mac" || pstyle.kind === "dmg" ? pstyle.ink
                    : (pstyle.kind === "amiga" ? root.theme.fg : root.theme.warning)
                markerColor: pstyle.kind === "amiga" ? root.theme.fg : pstyle.ink
                targetColor: pstyle.hot
                appliedColor: pstyle.kind === "amiga" ? root.theme.fg : pstyle.ink
                lattice: pstyle.kind === "dots" ? 2 : 1
                latticeColor: pstyle.kind === "dots" ? root.theme.borderInactive : "transparent"
                roundMarkers: pstyle.kind === "dots"

                Rectangle {
                    visible: scrub.containsMouse
                    x: Math.max(0, Math.min(plot.width - 1, Math.floor(scrub.mouseX)))
                    width: 1
                    height: plot.height
                    antialiasing: false
                    color: pstyle.hot
                }

                MouseArea {
                    id: scrub
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    onPositionChanged: event => root.scrubTime = plot.timeAt(Math.floor(event.x))
                }
            }

            // Hour axis.
            Item {
                id: axis
                y: plot.y + plot.height + 1
                width: panel.contentWidth
                height: 5

                Repeater {
                    model: [0, 6, 12, 18, 24]
                    delegate: PixelLabel {
                        required property int modelData
                        theme: root.theme
                        face: "tiny"
                        color: pstyle.dim
                        text: PixelFormat.hour(modelData)
                        x: Math.max(0, Math.min(axis.width - artWidth,
                                                plot.xAt(plot.t0 + modelData * 3600) - Math.floor(artWidth / 2)))
                    }
                }
            }
        }
    }
}
