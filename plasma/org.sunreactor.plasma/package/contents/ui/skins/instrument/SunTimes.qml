/*
 * Sun times, instrument family: the day as a 24-hour scale (night, twilight,
 * daylight, now) over three readouts; the next event is in the accent.
 *
 *   SUN ──────────────────────────────────────────────── DAY 11 H 25
 *   ┊┊┊┊┊┊░░▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░┊┊┊┊┊┊┊   ▲ now
 *   RISE 07:13          NOON 12:55          SET 18:38          ELEV 32.4°
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    readonly property var sun: client.dayCurve && client.dayCurve.sun ? client.dayCurve.sun : ({})
    /*! Local midnight of the day the sun times belong to, and its length. */
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
    readonly property real dayEnd: {
        const samples = client.dayCurve && client.dayCurve.monitors && client.dayCurve.monitors.length
            ? client.dayCurve.monitors[0].samples : [];
        return samples.length > 1 ? samples[samples.length - 1].t : dayStart + 86400;
    }
    readonly property var entries: [
        { label: i18ndc(domain, "Short legend: sunrise", "Rise"), epoch: client.sunriseEpochS },
        { label: i18ndc(domain, "Short legend: solar noon", "Noon"), epoch: client.solarNoonEpochS },
        { label: i18ndc(domain, "Short legend: sunset", "Set"), epoch: client.sunsetEpochS },
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

    spacing: style.rowGap

    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.domain, "Instrument legend over sunrise and sunset", "Sun")
        trailing: root.dayLength > 0
            ? i18ndc(root.domain, "Length of daylight, hours and minutes, compact", "Day %1 h %2",
                     Math.floor(root.dayLength / 3600), String(Math.round(root.dayLength % 3600 / 60)).padStart(2, "0"))
            : ""
    }

    // The day scale.
    Item {
        id: track

        Layout.fillWidth: true
        implicitHeight: root.style.snap(root.style.sizeLegend * 1.3)

        function xAt(t) {
            return root.style.snap((t - root.dayStart) / Math.max(1, root.dayEnd - root.dayStart)
                                   * (width - root.style.hairline));
        }
        readonly property real midY: root.style.snap(height / 2)

        Repeater {
            model: 25
            delegate: Rectangle {
                required property int index
                x: track.xAt(root.dayStart + index * 3600)
                y: track.midY - height / 2
                width: root.style.hairline
                height: root.style.snap(index % 6 === 0 ? track.height : track.height * 0.4)
                color: index % 6 === 0 ? root.style.rule : root.style.ruleFaint
            }
        }
        // Twilight: dawn to dusk.
        Rectangle {
            readonly property real from: root.sun.dawn > 0 ? root.sun.dawn : root.client.sunriseEpochS
            readonly property real to: root.sun.dusk > 0 ? root.sun.dusk : root.client.sunsetEpochS
            x: track.xAt(from)
            width: track.xAt(to) - x
            y: track.midY - height / 2
            height: root.style.line2
            color: root.style.ruleStrong
        }
        // Daylight.
        Rectangle {
            x: track.xAt(root.client.sunriseEpochS)
            width: track.xAt(root.client.sunsetEpochS) - x
            y: track.midY - height / 2
            height: root.style.snap(root.style.line2 * 1.5)
            color: root.style.accent2
        }
        // Now.
        Rectangle {
            visible: root.client.nowEpochS >= root.dayStart && root.client.nowEpochS <= root.dayEnd
            x: track.xAt(root.client.nowEpochS)
            y: 0
            width: root.style.line2
            height: track.height
            color: root.style.accent
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        Repeater {
            model: root.entries

            delegate: Row {
                id: entry

                required property var modelData
                required property int index
                readonly property bool isNext: modelData.epoch === root.nextEpoch

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                spacing: root.style.gap

                Text {
                    anchors.baseline: time.baseline
                    text: entry.modelData.label
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkMuted
                }
                Text {
                    id: time
                    text: Format.timeText(entry.modelData.epoch)
                    textFormat: Text.PlainText
                    font: root.style.dataFont
                    color: entry.isNext ? root.style.accent : root.style.ink
                }
            }
        }

        Row {
            visible: root.showElevation
            spacing: root.style.gap

            Text {
                anchors.baseline: elevation.baseline
                text: i18ndc(root.domain, "Short legend: the sun's height above the horizon", "Elev")
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkMuted
            }
            Text {
                id: elevation
                text: i18ndc(root.domain, "Angle in degrees", "%1°", root.client.solarElevation.toFixed(1))
                textFormat: Text.PlainText
                font: root.style.dataFont
                color: root.style.ink
            }
        }
    }
}
