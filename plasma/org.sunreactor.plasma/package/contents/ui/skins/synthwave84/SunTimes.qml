/*
 * Sun times, synthwave84: the day as a sun arc. A cyan arc from sunrise to
 * sunset, a pink sun on it at this moment, and the three times underneath
 * (rise, noon, set), the next one lit in pink. Optional: the sun's height.
 *
 *            ●  (the sun, now)
 *        ╭───┴───╮
 *   ─────╯       ╰─────────  horizon
 *   RISE 06:12  NOON 13:05  SET 19:46
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../Format.js" as Format
import "../instrument" as Instrument

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    readonly property var events: [
        { label: i18ndc(domain, "Short legend: sunrise", "Rise"), epoch: client.sunriseEpochS },
        { label: i18ndc(domain, "Short legend: solar noon", "Noon"), epoch: client.solarNoonEpochS },
        { label: i18ndc(domain, "Short legend: sunset", "Set"), epoch: client.sunsetEpochS }
    ]
    /*! Index of the next event, or -1 once the day has gone by. */
    readonly property int nextIndex: {
        for (let i = 0; i < events.length; ++i) {
            if (events[i].epoch > client.nowEpochS) {
                return i;
            }
        }
        return -1;
    }
    /*! Where the sun is along the arc: 0 at sunrise, 1 at sunset; -1 outside the day. */
    readonly property real progress: client.sunsetEpochS > client.sunriseEpochS
        && client.nowEpochS >= client.sunriseEpochS && client.nowEpochS <= client.sunsetEpochS
        ? (client.nowEpochS - client.sunriseEpochS) / (client.sunsetEpochS - client.sunriseEpochS) : -1

    readonly property string dayText: {
        const length = client.sunsetEpochS - client.sunriseEpochS;
        const hours = Math.floor(length / 3600);
        const minutes = Math.round((length % 3600) / 60);
        return i18ndc(domain, "Length of the day in hours and minutes", "%1 h %2 min", hours, minutes);
    }

    spacing: style.rowGap

    Accessible.role: Accessible.StaticText
    Accessible.name: available
        ? i18nd(domain, "Sun times") + ": " + events.map(e => e.label + " " + Format.timeText(e.epoch)).join(", ")
        : ""

    Heading {
        style: root.style
        text: i18ndc(root.domain, "Synthwave heading of the sun times", "Sun times")
    }

    NeonCard {
        Layout.fillWidth: true
        style: root.style
        neon: root.style.cyan

        // The arc: sunrise on the left, sunset on the right, the horizon under it.
        Item {
            id: arc

            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.unit * 2.1)

            readonly property real inset: root.style.unit * 0.9
            readonly property real baseY: height - root.style.unit * 0.25
            readonly property real radiusX: (width - 2 * inset) / 2
            readonly property real radiusY: Math.max(1, baseY - root.style.unit * 0.3)
            readonly property real centreX: width / 2
            readonly property real sunSize: root.style.snap(root.style.unit * 0.62)

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: root.style.cyan
                    strokeWidth: root.style.line2
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathSvg {
                        path: "M " + arc.inset + " " + arc.baseY
                            + " A " + arc.radiusX + " " + arc.radiusY + " 0 0 1 " + (arc.width - arc.inset) + " " + arc.baseY
                    }
                }
            }

            // The horizon.
            Rectangle {
                x: 0
                y: Math.round(arc.baseY)
                width: arc.width
                height: root.style.hairline
                color: root.style.alpha(root.style.lilac, 0.9)
            }

            // The sun on the arc, now.
            Instrument.Glow {
                anchors.fill: sunDot
                source: sunDot
                strength: root.style.glow
                tint: root.style.pink
            }
            Rectangle {
                id: sunDot

                visible: root.progress >= 0
                width: arc.sunSize
                height: width
                radius: width / 2
                color: root.style.pink
                x: root.progress >= 0
                    ? arc.centreX + arc.radiusX * Math.cos(Math.PI * (1 - root.progress)) - width / 2 : 0
                y: root.progress >= 0
                    ? arc.baseY - arc.radiusY * Math.sin(Math.PI * (1 - root.progress)) - height / 2 : 0
            }
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 3
            columnSpacing: root.style.gap2
            uniformCellWidths: true

            Repeater {
                model: root.available ? root.events : []

                // The legend over the time, as the three columns of the day's times.
                delegate: ColumnLayout {
                    id: event

                    required property var modelData
                    required property int index

                    Layout.fillWidth: true
                    spacing: 0

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: event.modelData.label
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: event.index === root.nextIndex ? root.style.pink : root.style.cyan
                    }
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: Format.timeText(event.modelData.epoch)
                        textFormat: Text.PlainText
                        font: root.style.dataFont
                        color: root.style.ink
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap2

            Text {
                text: root.dayText
                textFormat: Text.PlainText
                font: root.style.dataSmallFont
                color: root.style.muted
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                visible: root.settings.showSolarElevation && root.client.hasSolarElevation
                text: i18ndc(root.domain, "Sun height above the horizon in degrees", "Sun %1°",
                             root.client.solarElevation.toFixed(1))
                textFormat: Text.PlainText
                font: root.style.dataSmallFont
                color: root.style.muted
            }
        }
    }
}
