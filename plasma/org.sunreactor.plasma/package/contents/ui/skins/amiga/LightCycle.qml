/*
 * Amiga LightCycle: "Today" in a Workbench window. The day curve is the shared
 * PixelCurve raster in Workbench pens: a black stepped line over a grey
 * ordered-dither area, the weather band in orange, the now marker in black.
 * Above it: the display gadgets (MI | LEN | ALL when there are several) and the
 * hero figure with its two captions, all measured so nothing overlaps. Hover
 * scrubs the time; it never sets brightness. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat

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
    property real scrubTime: 0

    function appliedFor(monitorId) {
        for (const monitor of root.client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(root.domain, "Today's brightness curve")
    Accessible.description: current && targetNow >= 0
        ? i18ndc(root.domain, "Display name, then the brightness the curve targets now", "%1: %2% now",
                 current.monitorId, Math.round(targetNow))
        : ""

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "curve"

        AmigaWindow {
            id: panel

            theme: root.theme
            style: tokens
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the day curve window", "Today")
            contentHeight: selector.height + readout.height + 2 + plot.height + 1 + axis.height

            // Display gadgets: one per display, then All.
            Row {
                id: selector

                spacing: 2
                height: root.curves.length > 1 ? 11 : 0
                visible: root.curves.length > 1

                Repeater {
                    model: root.curves.length > 1 ? root.curves.length + 1 : 0

                    delegate: AmigaGadget {
                        id: pick

                        required property int index
                        readonly property bool isAll: index === root.curves.length
                        readonly property int value: isAll ? -1 : index

                        theme: root.theme
                        style: tokens
                        text: isAll ? i18ndc(root.domain, "Show the curves of every display", "All")
                                    : PixelFormat.shortName(root.curves[index].monitorId)
                        lit: root.selected === value
                        onActivated: root.selected = value
                    }
                }
            }

            // The hero figure, and its two captions to the right of it.
            Item {
                id: readout

                readonly property bool scrubbing: scrub.containsMouse && root.current !== null
                readonly property real shownPct: scrubbing ? plot.valueAt(root.scrubTime)
                                                          : (root.showAll ? -1 : root.targetNow)
                readonly property int applied: root.current && !root.showAll ? root.appliedFor(root.current.monitorId) : -1

                y: selector.height
                width: panel.contentWidth
                height: 14

                AmigaText {
                    id: hero

                    pitch: 2
                    color: tokens.ink
                    text: readout.shownPct >= 0 ? Math.round(readout.shownPct) + "%" : "--%"
                }
                AmigaText {
                    id: caption

                    x: hero.artWidth + 6
                    y: 0
                    color: tokens.ink
                    maximumArtWidth: Math.max(0, readout.width - x)
                    text: readout.scrubbing ? PixelFormat.time(root.scrubTime)
                        : i18ndc(root.domain, "The curve's brightness right now", "Now")
                }
                AmigaText {
                    id: setLine

                    x: hero.artWidth + 6
                    y: 8
                    color: tokens.ink
                    maximumArtWidth: Math.max(0, readout.width - x)
                    visible: !readout.scrubbing && readout.applied >= 0
                    text: readout.applied >= 0
                        ? i18ndc(root.domain, "Brightness the display has right now", "Set %1%", readout.applied) : ""
                }
            }

            PixelCurve {
                id: plot

                y: readout.y + readout.height + 2
                width: panel.contentWidth
                height: 16
                artScale: 1

                samples: root.showAll ? [] : (root.current ? root.current.samples : [])
                overlays: root.showAll ? root.curves.map(c => c.samples) : []
                overlayColors: [tokens.ink, tokens.hot, tokens.dim, tokens.hot]
                now: root.now
                applied: root.current && !root.showAll ? readout.applied : -1
                minPct: root.current && !root.showAll ? root.current.minPct : -1
                maxPct: root.current && !root.showAll ? root.current.maxPct : -1
                sunrise: root.sun.sunrise || 0
                solarNoon: root.sun.solarNoon || 0
                sunset: root.sun.sunset || 0
                t0: root.curves.length > 0 && root.curves[0].samples.length > 0 ? root.curves[0].samples[0].t : 0
                t1: root.curves.length > 0 && root.curves[0].samples.length > 0
                    ? root.curves[0].samples[root.curves[0].samples.length - 1].t : 0

                paperColor: tokens.body
                lineColor: tokens.ink
                lineWeight: 1
                areaColor: tokens.face
                areaPattern: "checker"
                areaLevel: 0.5
                costColor: tokens.hot
                costPattern: "checker"
                bandColor: tokens.hot
                nightColor: tokens.face
                nightPattern: "sparse"
                tickColor: tokens.ink
                sunColor: tokens.hot
                markerColor: tokens.ink
                targetColor: tokens.hot
                appliedColor: tokens.ink
                lattice: 1
                latticeColor: "transparent"

                Rectangle {
                    visible: scrub.containsMouse
                    x: Math.max(0, Math.min(plot.width - 1, Math.floor(scrub.mouseX)))
                    width: 1
                    height: plot.height
                    antialiasing: false
                    color: tokens.hot
                }

                MouseArea {
                    id: scrub

                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    onPositionChanged: event => root.scrubTime = plot.timeAt(Math.floor(event.x))
                }
            }

            // Hour axis: the labels sit under their hours and never leave the window.
            Item {
                id: axis

                y: plot.y + plot.height + 1
                width: panel.contentWidth
                height: 8

                Repeater {
                    model: [0, 6, 12, 18, 24]

                    delegate: AmigaText {
                        required property int modelData

                        text: PixelFormat.hour(modelData)
                        color: tokens.ink
                        x: Math.max(0, Math.min(axis.width - artWidth,
                                                plot.xAt(plot.t0 + modelData * 3600) - Math.floor(artWidth / 2)))
                    }
                }
            }
        }
    }
}
