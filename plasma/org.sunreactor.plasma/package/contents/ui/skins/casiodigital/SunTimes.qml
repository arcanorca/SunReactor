/*
 * Sun times, casiodigital: sunrise, solar noon and sunset in seven segments
 * under printed captions, and a slim track of the day with the sun's progress
 * lit in ink and the now marker in the backlight.
 *
 *   RISE          NOON          SET
 *   06:12         13:04         19:56
 *   ━━━━━━━━━━━━━━━━━━━━━━━━┃━━━━━━━━━━━━━━━
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Casio.js" as Casio

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CasioStyle style: CasioStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { label: i18nd(domain, "Rise"), epoch: client.sunriseEpochS },
        { label: i18nd(domain, "Noon"), epoch: client.solarNoonEpochS },
        { label: i18nd(domain, "Set"), epoch: client.sunsetEpochS }
    ]
    readonly property bool daylight: client.nowEpochS >= client.sunriseEpochS
        && client.nowEpochS < client.sunsetEpochS
    /*! Where the sun is on the track, 0 at sunrise and 1 at sunset (clamped). */
    readonly property real progress: client.sunsetEpochS > client.sunriseEpochS
        ? Math.max(0, Math.min(1, (client.nowEpochS - client.sunriseEpochS)
                                  / (client.sunsetEpochS - client.sunriseEpochS)))
        : 0
    /*! Solar noon along the track, 0..1. */
    readonly property real noonFraction: client.sunsetEpochS > client.sunriseEpochS
        ? Math.max(0, Math.min(1, (client.solarNoonEpochS - client.sunriseEpochS)
                                  / (client.sunsetEpochS - client.sunriseEpochS)))
        : 0.5

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.domain, "Sun")
        trailing: root.daylight ? i18nd(root.domain, "Day") : i18nd(root.domain, "Night")

        RowLayout {
            Layout.fillWidth: true
            spacing: 0

            Repeater {
                model: root.entries

                delegate: ColumnLayout {
                    id: entry

                    required property var modelData

                    readonly property var clock: Casio.clockParts(Format.timeText(modelData.epoch))

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: root.style.gap

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: entry.modelData.label
                        textFormat: Text.PlainText
                        font: root.style.cap(root.style.sizeCap)
                        color: root.style.inkMuted
                    }
                    RowLayout {
                        Layout.alignment: Qt.AlignHCenter
                        spacing: root.style.snap(root.style.gap * 0.5)

                        CasioDigits {
                            style: root.style
                            size: root.style.digitSmall
                            text: entry.clock.digits
                        }
                        Text {
                            Layout.alignment: Qt.AlignBottom
                            visible: entry.clock.suffix.length > 0
                            text: entry.clock.suffix
                            textFormat: Text.PlainText
                            font: root.style.cap(root.style.sizeCap)
                            color: root.style.inkMuted
                        }
                    }
                }
            }
        }

        // The track of the day: a ghost rail, the sun's progress in ink, ticks at
        // the three events and the now marker in the backlight.
        Item {
            id: track

            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.sizeCap * 1.1)
            Accessible.role: Accessible.Indicator
            Accessible.name: i18nd(root.domain, "Where the sun is between sunrise and sunset")

            readonly property real railY: height / 2 - root.style.line2 / 2
            readonly property real nowX: root.style.snap(root.progress * width)

            Rectangle {
                x: 0
                y: track.railY
                width: track.width
                height: root.style.line2
                color: root.style.inkFaint
            }
            Rectangle {
                x: 0
                y: track.railY
                width: track.nowX
                height: root.style.line2
                color: root.style.ink
            }

            Repeater {
                model: [0, root.noonFraction, 1]

                delegate: Rectangle {
                    required property real modelData

                    x: root.style.snap(modelData * (track.width - root.style.hairline))
                    y: track.height / 2 - height / 2
                    width: root.style.hairline * 2
                    height: track.height * 0.6
                    color: root.style.inkMuted
                }
            }

            Rectangle {
                visible: root.client.nowEpochS >= root.client.sunriseEpochS
                x: Math.max(0, Math.min(track.width - width, track.nowX - width / 2))
                y: 0
                width: root.style.line2 * 1.5
                height: track.height
                color: root.style.backlight
            }
        }
    }
}
