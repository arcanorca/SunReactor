/*
 * Sun times, Wii style: the day as a pill track. The daylight is Wii blue,
 * the night the light grey; small sun discs mark sunrise, noon and sunset, and
 * a round white marker shows the time now. The times sit under the track.
 *
 *   ( ░░░░░░░░░ ◉ ●───────────────────● ░░░░░░░░░ )
 *   Rise 07:13            Noon 12:55            Set 18:38
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
    readonly property WiiStyle style: WiiStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    readonly property var curve: client.dayCurve && client.dayCurve.monitors && client.dayCurve.monitors.length > 0
        ? client.dayCurve.monitors[0] : null
    readonly property var samples: curve && curve.samples ? curve.samples : []
    readonly property real dayStart: samples.length > 0 ? samples[0].t : client.sunriseEpochS - 3600 * 3
    readonly property real dayEnd: samples.length > 0 ? samples[samples.length - 1].t : client.sunsetEpochS + 3600 * 3
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    spacing: style.rowGap

    Accessible.role: Accessible.StaticText
    Accessible.name: i18nd(domain, "Sun times")

    WiiTile {
        id: tile

        style: root.style
        pad: root.style.gap
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 2 * tile.pad

        ColumnLayout {
            id: inner

            anchors.fill: parent
            spacing: root.style.gap

            Item {
                id: track

                readonly property real markerD: root.style.snap(root.style.unit * 0.8)
                readonly property real trackH: root.style.snap(root.style.unit * 0.5)

                Layout.fillWidth: true
                implicitHeight: markerD

                function xAt(t) {
                    return (t - root.dayStart) / Math.max(1, root.dayEnd - root.dayStart) * width;
                }

                // The whole day: the pale track.
                Rectangle {
                    x: 0
                    width: track.width
                    height: track.trackH
                    y: (track.height - height) / 2
                    radius: height / 2
                    antialiasing: true
                    color: root.style.track
                }

                // Daylight: sunrise to sunset, Wii blue with a gloss on its top half.
                Item {
                    id: daylight

                    readonly property real from: track.xAt(root.client.sunriseEpochS)
                    readonly property real to: track.xAt(root.client.sunsetEpochS)

                    x: Math.max(0, from)
                    width: Math.max(track.trackH, Math.min(track.width, to) - x)
                    height: track.trackH
                    y: (track.height - height) / 2

                    Rectangle {
                        anchors.fill: parent
                        radius: height / 2
                        antialiasing: true
                        color: root.style.accent
                    }
                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        height: Math.round(parent.height / 2)
                        radius: height / 2
                        antialiasing: true
                        gradient: Gradient {
                            GradientStop { position: 0; color: root.style.gloss }
                            GradientStop { position: 1; color: root.style.glossClear }
                        }
                    }
                }

                // The sun markers: sunrise, noon, sunset.
                Repeater {
                    model: [root.client.sunriseEpochS, root.client.solarNoonEpochS, root.client.sunsetEpochS]

                    delegate: Rectangle {
                        required property real modelData

                        readonly property real at: track.xAt(modelData)

                        visible: modelData > 0 && at >= -track.markerD / 2 && at <= track.width + track.markerD / 2
                        x: at - track.markerD / 2
                        y: (track.height - track.markerD) / 2
                        width: track.markerD
                        height: track.markerD
                        radius: width / 2
                        antialiasing: true
                        color: root.style.sun
                        border.width: root.style.line2
                        border.color: root.style.paper
                    }
                }

                // Now: a round white marker with a blue ring.
                Rectangle {
                    readonly property real at: track.xAt(root.client.nowEpochS)

                    visible: root.client.nowEpochS >= root.dayStart && root.client.nowEpochS <= root.dayEnd
                    x: at - width / 2
                    y: (track.height - height) / 2
                    width: track.markerD * 0.7
                    height: width
                    radius: width / 2
                    antialiasing: true
                    color: root.style.paper
                    border.width: root.style.line2
                    border.color: root.style.accentLine
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                Text {
                    text: i18ndc(root.domain, "Sun times: sunrise, then the time", "Rise %1",
                                 Format.timeText(root.client.sunriseEpochS))
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.ink
                }
                Item {
                    Layout.fillWidth: true
                }
                Text {
                    text: i18ndc(root.domain, "Sun times: solar noon, then the time", "Noon %1",
                                 Format.timeText(root.client.solarNoonEpochS))
                    textFormat: Text.PlainText
                    font: root.style.captionFont
                    color: root.style.inkSoft
                }
                Item {
                    Layout.fillWidth: true
                }
                Text {
                    text: i18ndc(root.domain, "Sun times: sunset, then the time", "Set %1",
                                 Format.timeText(root.client.sunsetEpochS))
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.ink
                }
                Text {
                    visible: root.showElevation
                    text: i18ndc(root.domain, "Angle in degrees", "%1°", root.client.solarElevation.toFixed(1))
                    textFormat: Text.PlainText
                    font: root.style.captionFont
                    color: root.style.inkSoft
                }
            }
        }
    }
}
