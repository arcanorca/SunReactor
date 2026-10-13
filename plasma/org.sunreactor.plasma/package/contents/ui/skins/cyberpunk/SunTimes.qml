/*
 * Sun times, cyberpunk: the day as 24 slanted cells, one per hour, coloured by
 * phase (night ghost, twilight cyan, daylight yellow), with a red NOW cursor and
 * its tag. The sunrise, noon and sunset readouts sit under it in brackets; the
 * next event is in the accent.
 *
 *   SUN ───────────────────────────────────────────────  DAY 11H 25M
 *             [NOW 14:30]
 *   ▱▱▱▱▱▱▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱▱▱
 *   RISE [07:13]     NOON [12:55]     SET [18:38]
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Geometry.js" as G

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    readonly property var sun: client.dayCurve && client.dayCurve.sun ? client.dayCurve.sun : ({})
    /*! Local midnight of the day the sun times belong to, and its end. */
    readonly property real dayStart: {
        const samples = client.dayCurve && client.dayCurve.monitors && client.dayCurve.monitors.length
            ? client.dayCurve.monitors[0].samples : [];
        if (samples.length > 1) {
            return samples[0].t;
        }
        const d = new Date(client.sunriseEpochS * 1000);
        d.setHours(0, 0, 0, 0);
        return d.getTime() / 1000;
    }
    readonly property real dayEnd: dayStart + 86400

    /*! Twilight edges: dawn and dusk when the daemon sends them, else sunrise and sunset. */
    readonly property real dawn: sun.dawn > 0 ? sun.dawn : client.sunriseEpochS
    readonly property real dusk: sun.dusk > 0 ? sun.dusk : client.sunsetEpochS

    /*! 0 night, 1 twilight, 2 daylight. */
    function phaseAt(t) {
        if (t < dawn || t > dusk) {
            return 0;
        }
        if (t < client.sunriseEpochS || t > client.sunsetEpochS) {
            return 1;
        }
        return 2;
    }

    readonly property var entries: [
        { label: i18ndc(domain, "Short legend: sunrise", "RISE"), epoch: client.sunriseEpochS },
        { label: i18ndc(domain, "Short legend: solar noon", "NOON"), epoch: client.solarNoonEpochS },
        { label: i18ndc(domain, "Short legend: sunset", "SET"), epoch: client.sunsetEpochS }
    ]
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }
    readonly property real dayLength: client.sunsetEpochS - client.sunriseEpochS
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation
    readonly property bool nowInDay: client.nowEpochS >= dayStart && client.nowEpochS <= dayEnd

    spacing: style.rowGap

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        title: i18ndc(root.domain, "Panel title: sunrise and sunset", "Sun times")
        tag: root.dayLength > 0
            ? i18ndc(root.domain, "Length of daylight, hours and minutes, compact", "DAY %1H %2M",
                     Math.floor(root.dayLength / 3600), String(Math.round(root.dayLength % 3600 / 60)).padStart(2, "0"))
            : ""

        // The day scale: 24 cells and the NOW cursor with its tag above.
        Item {
            id: dayBar

            Layout.fillWidth: true
            implicitHeight: barTop + barH + root.style.snap(root.style.unit * 0.5)

            readonly property real barTop: root.style.snap(root.style.tabHeight * 0.8)
            readonly property real barH: root.style.snap(root.style.unit * 0.7)
            readonly property real pitch: width / 24
            readonly property real skew: root.style.snap(barH * 0.4)
            readonly property real cw: Math.max(1, pitch - skew - root.style.hairline * 2)

            function xAt(t) {
                return (t - root.dayStart) / Math.max(1, root.dayEnd - root.dayStart) * width;
            }
            function cellsFor(phase) {
                let d = "";
                for (let h = 0; h < 24; ++h) {
                    if (root.phaseAt(root.dayStart + (h + 0.5) * 3600) === phase) {
                        d += G.cells(h, h + 1, pitch, cw, barH, skew);
                    }
                }
                return d;
            }

            Shape {
                x: 0
                y: dayBar.barTop
                width: dayBar.width
                height: dayBar.barH
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.ghost
                    PathSvg { path: dayBar.cellsFor(0) }
                }
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.theme.alpha(root.style.cyan, 0.45)
                    PathSvg { path: dayBar.cellsFor(1) }
                }
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.yellow
                    PathSvg { path: dayBar.cellsFor(2) }
                }
            }

            Rectangle {
                visible: root.nowInDay
                x: dayBar.xAt(root.client.nowEpochS) - width / 2
                y: dayBar.barTop - root.style.gap
                width: root.style.trace
                height: dayBar.barH + root.style.gap * 2
                color: root.style.red
            }
            Rectangle {
                id: nowTag

                visible: root.nowInDay
                x: Math.max(0, Math.min(dayBar.width - width, dayBar.xAt(root.client.nowEpochS) - width / 2))
                y: 0
                width: nowText.implicitWidth + root.style.gap * 2
                height: nowText.implicitHeight + root.style.hairline * 4
                color: root.style.red

                Text {
                    id: nowText

                    anchors.centerIn: parent
                    text: i18ndc(root.domain, "Cursor tag: the current time", "NOW %1", Format.timeText(root.client.nowEpochS))
                    textFormat: Text.PlainText
                    font: root.style.tinyFont
                    color: root.style.onRed
                }
            }

            // Hour ruler under the cells, long ticks every six hours.
            Shape {
                x: 0
                y: dayBar.barTop + dayBar.barH + root.style.hairline * 2
                width: dayBar.width
                height: root.style.snap(root.style.unit * 0.4)
                ShapePath {
                    strokeColor: root.style.rule
                    strokeWidth: root.style.hairline
                    fillColor: "transparent"
                    PathSvg { path: G.ticks(dayBar.width, 24, 6, 2, 4) }
                }
            }
        }

        // Hour labels under the ruler.
        Item {
            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.sizeTiny * 1.4)

            Repeater {
                id: hours

                model: 5

                delegate: Text {
                    required property int index

                    x: Math.max(0, Math.min(parent.width - width, parent.width * index / 4 - width / 2))
                    y: 0
                    text: String(index * 6).padStart(2, "0")
                    textFormat: Text.PlainText
                    font: root.style.tinyFont
                    color: root.style.muted
                }
            }
        }

        // The readouts; the next event is in the accent.
        // A flow: the three times and the elevation wrap onto a second line rather than clip.
        Flow {
            Layout.fillWidth: true
            spacing: root.style.gap2

            Repeater {
                model: root.entries

                delegate: Row {
                    id: entry

                    required property var modelData
                    readonly property bool isNext: modelData.epoch === root.nextEpoch

                    spacing: root.style.gap

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: entry.modelData.label
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.muted
                    }
                    Bracket {
                        anchors.verticalCenter: parent.verticalCenter
                        style: root.style
                        value: entry.modelData.epoch > 0 ? Format.timeText(entry.modelData.epoch) : "--"
                        font: root.style.techFont
                        color: entry.isNext ? root.style.yellow : root.style.ink
                        bracketColor: entry.isNext ? root.style.yellow : root.style.muted
                    }
                }
            }

            Row {
                visible: root.showElevation
                spacing: root.style.gap

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: i18ndc(root.domain, "Short legend: the sun's height above the horizon", "ELEV")
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: root.style.muted
                }
                Bracket {
                    anchors.verticalCenter: parent.verticalCenter
                    style: root.style
                    value: root.client.hasSolarElevation ? root.client.solarElevation.toFixed(1) + "°" : "--"
                    font: root.style.techFont
                    color: root.style.cyan
                    bracketColor: root.style.muted
                }
            }
        }
    }
}
