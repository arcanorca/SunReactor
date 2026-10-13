/*
 * Light cycle, macos: today's target curve as a card, after the Daylight widget.
 *
 *   ☀ Daylight                                   [MI] [LEN]
 *   39%         ╱‾‾‾‾‾‾╲          The area under the curve fades out;
 *   Applied 39  │        ╲___     a dot with a white ring marks now.
 *   Weather −5  07:13      18:38
 *
 * Hovering the chart scrubs the day: the figure becomes the target at that time
 * and a tooltip floats over the curve. Several displays are picked from a small
 * segmented control. The curve is the daemon's day_curve; nothing here sets it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
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
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves` of the display on the chart. */
    property int selected: 0
    readonly property var current: curves.length > 0
        ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current && current.samples ? current.samples : []
    readonly property real now: client.nowEpochS
    readonly property var sun: available && curve.sun ? curve.sun : ({})

    readonly property real targetNow: current ? Curve.valueAt(samples, now, "pct") : -1
    readonly property int appliedNow: current ? appliedFor(current.monitorId) : -1

    // While the pointer is over the chart, the figures read the day at that time.
    readonly property bool scrubbing: chart.scrubbing
    readonly property real readTime: scrubbing ? chart.hoverTime : now
    readonly property real readTarget: scrubbing ? chart.hoverValue : targetNow
    readonly property real readClearSky: current ? Curve.valueAt(samples, readTime, "pctNoWeather") : -1
    readonly property real readCost: readTarget >= 0 && readClearSky >= 0 ? readClearSky - readTarget : 0

    /*! The marks under the chart: sunrise and sunset, when known. */
    readonly property var marks: [
        { t: sun.sunrise, label: Format.timeText(sun.sunrise) },
        { t: sun.sunset, label: Format.timeText(sun.sunset) }
    ].filter(mark => mark.t > 0)

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    function pctText(v) {
        return v >= 0 ? i18ndc(domain, "Percentage", "%1%", Math.round(v)) : "—";
    }

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Accessible name of the daylight curve card", "Daylight")

    Surface {
        Layout.fillWidth: true
        style: root.style
        padding: root.style.snap(10)

        // --- title row: the sun, the name, and the displays on the chart --------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap

            SunGlyph {
                Layout.alignment: Qt.AlignVCenter
                size: 13
                color: root.style.sun
            }

            Text {
                Layout.alignment: Qt.AlignVCenter
                text: i18nd(root.domain, "Daylight")
                textFormat: Text.PlainText
                font: root.style.captionMedium
                color: root.style.inkSecondary
            }

            Item {
                Layout.fillWidth: true
            }

            Segmented {
                visible: root.curves.length > 1
                Layout.alignment: Qt.AlignVCenter
                Layout.maximumWidth: root.width * 0.6
                Layout.preferredHeight: root.style.snap(20)
                style: root.style
                segments: root.curves.map((curve, i) => ({ id: String(i), text: Curve.shortName(curve.monitorId) }))
                current: String(root.selected)

                onPicked: id => root.selected = Number(id)
            }
        }

        // --- the figure and the chart ------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.snap(12)

            ColumnLayout {
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: root.style.snap(92)
                spacing: 0

                Text {
                    Layout.fillWidth: true
                    text: root.pctText(root.readTarget)
                    textFormat: Text.PlainText
                    font: root.style.hero
                    color: root.style.ink
                    elide: Text.ElideRight
                }
                Text {
                    Layout.fillWidth: true
                    text: root.scrubbing
                        ? Format.timeText(root.readTime)
                        : i18nd(root.domain, "Target now")
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: root.style.inkSecondary
                }
                Text {
                    Layout.fillWidth: true
                    text: i18ndc(root.domain, "Brightness the daemon last applied", "Applied %1",
                                 root.appliedNow >= 0 ? Math.round(root.appliedNow) : "—")
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: root.style.inkSecondary
                }
                Text {
                    Layout.fillWidth: true
                    text: i18ndc(root.domain, "Brightness the weather takes away, in percent points", "Weather %1",
                                 Math.round(root.readCost) >= 1 ? "−" + Math.round(root.readCost) : "0")
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: Math.round(root.readCost) >= 1 ? root.style.warning : root.style.inkSecondary
                }
            }

            CurveChart {
                id: chart

                Layout.fillWidth: true
                Layout.preferredHeight: root.style.snap(92)
                style: root.style
                samples: root.samples
                now: root.now
                marks: root.marks
            }
        }
    }
}
