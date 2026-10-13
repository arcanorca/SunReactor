/*
 * Light cycle, Nothing: today's target as a dotted ring with the 5x7 percentage
 * inside it, the readings beside the ring, and the day below as a sparse dot plot.
 *
 *   TARGET                                   (MI) (LEN) (ALL)
 *   (dotted ring) APPLIED                              39
 *   ( 39%       ) WEATHER                              -5
 *   (  ring     ) RANGE                              1-50
 *   . . . . . . . . . . .     clear-sky dots: the weather cost
 *       o o o o ●             one dot per column, red at now
 *   . . . . . . . . . . .     dotted baseline
 *   00      06      12      18      00
 *
 * Hover scrubs the readings (time, clear-sky target, weather cost) and never
 * sets brightness. The display pills choose which day curve is drawn; ALL
 * overlays every display, the first one in white and the rest in grey.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

Rectangle {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property NothingStyle style: NothingStyle {
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
    /*! The curves drawn: one, or every display for ALL. */
    readonly property var plotted: available ? (showAll ? curves : [current]) : []

    readonly property real t0: current && current.samples.length > 0 ? current.samples[0].t : 0
    readonly property real t1: current && current.samples.length > 0
        ? current.samples[current.samples.length - 1].t : 1
    readonly property real now: client.nowEpochS
    readonly property bool nowInDay: available && now >= t0 && now <= t1

    /*! Dot rows of the day plot; the ring is `ringDiameter` dots across. */
    readonly property int plotRows: 12
    readonly property int ringDiameter: 21

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    readonly property bool scrubbing: hover.containsMouse && available && !showAll
    readonly property real readTime: scrubbing ? hover.time : now
    readonly property real readTarget: current ? style.valueAt(current.samples, readTime, "pct") : -1
    readonly property real readClear: current ? style.valueAt(current.samples, readTime, "pctNoWeather") : -1
    readonly property real readCost: readTarget >= 0 && readClear >= 0 ? readClear - readTarget : 0
    readonly property int appliedNow: current ? appliedFor(current.monitorId) : -1

    readonly property string caption: showAll
        ? i18ndc(root.domain, "Caption over the plot when every display is plotted", "Today")
        : (scrubbing
            ? i18ndc(root.domain, "Caption: the time under the pointer", "Target · %1", Format.timeText(readTime))
            : i18ndc(root.domain, "Caption: what the curve targets right now", "Target"))

    /*! Clock ticks every six local hours: { f: 0..1 along the day, label }. */
    readonly property var ticks: {
        const out = [];
        if (!available) {
            return out;
        }
        const span = Math.max(1, t1 - t0);
        for (let t = Math.ceil(t0 / 3600) * 3600; t <= t1; t += 3600) {
            const hour = new Date(t * 1000).getHours();
            if (hour % 6 === 0) {
                out.push({ f: (t - t0) / span, label: String(hour).padStart(2, "0") });
            }
        }
        return out;
    }

    implicitHeight: content.implicitHeight + 2 * style.pad
    radius: style.radius
    antialiasing: true
    color: style.surface
    border.width: style.hairline
    border.color: style.outline

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(root.domain, "Today's brightness curve")
    Accessible.description: current && readTarget >= 0
        ? i18ndc(root.domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(readTarget))
        : ""

    TextMetrics {
        id: tickMetrics

        font: root.style.label
        text: "00"
    }

    /*! One reading: a label flush left, its figure flush right. */
    component Reading: RowLayout {
        id: reading

        required property string caption
        required property string value

        Layout.fillWidth: true
        spacing: root.style.gap

        Text {
            Layout.fillWidth: true
            text: reading.caption
            textFormat: Text.PlainText
            font: root.style.label
            color: root.style.muted
            elide: Text.ElideRight
        }
        Text {
            text: reading.value
            textFormat: Text.PlainText
            font: root.style.figure
            color: root.style.ink
        }
    }

    ColumnLayout {
        id: content

        x: root.style.pad
        y: root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap

        // --- the caption and the display pills ------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap * 0.5

            Text {
                Layout.fillWidth: true
                text: root.caption
                textFormat: Text.PlainText
                font: root.style.label
                color: root.style.muted
                elide: Text.ElideRight
            }

            Repeater {
                model: root.curves.length > 1 ? root.curves.length + 1 : 0

                delegate: Pill {
                    id: channel

                    required property int index

                    readonly property bool isAll: index === root.curves.length
                    readonly property int value: isAll ? -1 : index

                    style: root.style
                    radio: true
                    active: root.selected === value
                    text: isAll ? i18ndc(root.domain, "Display selector: every display", "ALL")
                                : root.style.shortName(root.curves[index].monitorId)
                    accessibleName: isAll ? text : root.curves[index].monitorId
                    onClicked: root.selected = value
                }
            }
        }

        // --- the dial and the readings beside it ----------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap * 2

            Item {
                id: dial

                Layout.alignment: Qt.AlignVCenter
                width: ring.implicitWidth
                height: ring.implicitHeight

                // The dotted gauge: the lit arc is the target, the red head is the one signal.
                DotRing {
                    id: ring

                    diameter: root.ringDiameter
                    // Two dots per cell of circumference: an even dotted circle.
                    dots: 32
                    pitch: root.style.pitch
                    value: root.readTarget >= 0 && !root.showAll ? root.readTarget / 100 : 0
                    headAccent: true
                    litColor: root.style.ink
                    accentColor: root.style.accent
                    dimColor: root.style.dotGhost
                }

                // The target numeral, centred in the ring's hollow.
                DotText {
                    x: root.style.snap((dial.width - width) / 2)
                    y: root.style.snap((dial.height - height) / 2)
                    text: root.readTarget >= 0 && !root.showAll ? Math.round(root.readTarget) + "%" : "--%"
                    face: "small"
                    size: 1
                    letterSpacing: 1
                    pitch: root.style.pitch
                    litColor: root.style.ink
                    accentColor: root.style.accent
                    dimColor: root.style.textGhost
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                visible: !root.showAll
                spacing: root.style.gap * 0.9

                Reading {
                    caption: root.scrubbing
                        ? i18ndc(root.domain, "Readout: target with a clear sky, under the pointer", "CLEAR SKY")
                        : i18ndc(root.domain, "Readout: brightness the daemon last applied", "APPLIED")
                    value: root.scrubbing
                        ? (root.readClear >= 0 ? String(Math.round(root.readClear)) : "—")
                        : (root.appliedNow >= 0 ? String(root.appliedNow) : "—")
                }

                Reading {
                    caption: i18ndc(root.domain, "Readout: brightness the weather takes away", "WEATHER")
                    value: Math.round(root.readCost) >= 1 ? "−" + Math.round(root.readCost) : "0"
                }

                Reading {
                    caption: i18ndc(root.domain, "Readout: the display's automatic minimum and maximum", "RANGE")
                    value: root.current ? root.current.minPct + "–" + root.current.maxPct : "—"
                }
            }
        }

        // --- the day: a sparse dot plot ---------------------------------------------------
        Item {
            id: plot

            Layout.fillWidth: true
            Layout.topMargin: root.style.gap * 0.5
            implicitHeight: root.plotRows * root.style.pitch

            DotPlot {
                anchors.fill: parent
                style: root.style
                curves: root.plotted
                t0: root.t0
                t1: root.t1
                nowT: root.nowInDay ? root.now : -1
                rows: root.plotRows
            }

            // Scrub cursor: one hairline under the pointer, only while hovering.
            Rectangle {
                visible: root.scrubbing
                x: root.style.snap(hover.mouseX)
                y: 0
                width: root.style.hairline
                height: plot.height
                color: root.style.theme.alpha(root.style.ink, 0.5)
            }

            MouseArea {
                id: hover

                readonly property real time: root.t0 + Math.max(0, Math.min(1, mouseX / Math.max(1, width)))
                    * (root.t1 - root.t0)

                anchors.fill: parent
                hoverEnabled: true
                // Looking only: a click here must never change brightness.
                acceptedButtons: Qt.NoButton
            }
        }

        // --- hour ticks ----------------------------------------------------------------
        Item {
            Layout.fillWidth: true
            implicitHeight: tickMetrics.height

            Repeater {
                model: root.ticks

                delegate: Text {
                    required property var modelData

                    x: Math.max(0, Math.min(plot.width - implicitWidth,
                                            modelData.f * plot.width - implicitWidth / 2))
                    text: modelData.label
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                }
            }
        }
    }
}
