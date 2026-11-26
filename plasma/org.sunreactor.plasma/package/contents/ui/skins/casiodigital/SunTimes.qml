/*
 * Sun times, casiodigital: sunrise, solar noon and sunset in seven segments
 * under printed captions, aligned over their ticks on the slim day track and
 * separated by faint LCD hairlines. Falls back to compact or stacked AM/PM
 * suffixes when 12-hour times would otherwise crowd each other.
 *
 *   RISE          NOON          SET
 *   06:12    |    13:04    |    19:56
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
    readonly property bool available: (client.isConnected || client.statusStale) && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    readonly property var riseClock: Casio.clockParts(Format.timeText(client.sunriseEpochS))
    readonly property var noonClock: Casio.clockParts(Format.timeText(client.solarNoonEpochS))
    readonly property var setClock: Casio.clockParts(Format.timeText(client.sunsetEpochS))

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

    // Measure 12-hour suffix fit independently of suffixMode to avoid layout loops.
    readonly property bool hasSuffix: riseClock.suffix.length > 0
        || noonClock.suffix.length > 0 || setClock.suffix.length > 0
    readonly property real minAir: style.snap(style.unit * 0.75)
    readonly property int compactCapSize: Math.max(7, Math.round(style.sizeCap * 0.78))
    readonly property int stackedCapSize: Math.max(7, Math.round(style.sizeCap * 0.82))
    readonly property real digitsSampleWidth: Casio.seven("12:55", style.digitSmall, style.dpr).width
    readonly property real inlineWidthSum: 3 * digitsSampleWidth
        + (hasSuffix ? 3 * (style.snap(style.gap * 0.5) + inlineSuffixMetrics.advanceWidth) : 0)
    readonly property real compactWidthSum: 3 * digitsSampleWidth
        + (hasSuffix ? 3 * (style.snap(style.gap * 0.25) + compactSuffixMetrics.advanceWidth) : 0)
    readonly property string suffixMode: {
        if (!hasSuffix || headerRow.width <= 0) {
            return "inline";
        }
        if (inlineWidthSum + 2 * minAir <= headerRow.width) {
            return "inline";
        }
        if (compactWidthSum + 2 * minAir <= headerRow.width) {
            return "compact";
        }
        return "stacked";
    }

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    TextMetrics {
        id: inlineSuffixMetrics

        font: root.style.cap(root.style.sizeCap)
        text: root.noonClock.suffix.length > 0 ? root.noonClock.suffix : "PM"
    }

    TextMetrics {
        id: compactSuffixMetrics

        font: root.style.cap(root.compactCapSize)
        text: root.noonClock.suffix.length > 0 ? root.noonClock.suffix : "PM"
    }

    component SunReadout: ColumnLayout {
        id: readout

        property string label: ""
        property var clock: ({ digits: "--:--", suffix: "" })

        readonly property bool stacked: root.suffixMode === "stacked" && clock.suffix.length > 0
        readonly property int suffixSize: root.suffixMode === "compact"
            ? root.compactCapSize
            : (stacked ? root.stackedCapSize : root.style.sizeCap)

        spacing: root.style.gap

        Text {
            Layout.alignment: Qt.AlignHCenter
            Layout.maximumWidth: Math.max(digitsBox.implicitWidth, root.style.snap(root.style.unit * 4.5))
            text: readout.label
            textFormat: Text.PlainText
            font: root.style.cap(root.style.sizeCap)
            color: root.style.inkMuted
            elide: Text.ElideRight
        }

        ColumnLayout {
            id: digitsBox

            Layout.alignment: Qt.AlignHCenter
            spacing: root.style.snap(root.style.gap * 0.35)

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: root.style.snap(root.style.gap * (root.suffixMode === "compact" ? 0.25 : 0.45))

                CasioDigits {
                    style: root.style
                    size: root.style.digitSmall
                    text: readout.clock.digits
                }

                Text {
                    Layout.alignment: Qt.AlignBottom
                    visible: readout.clock.suffix.length > 0 && !readout.stacked
                    text: readout.clock.suffix
                    textFormat: Text.PlainText
                    font: root.style.cap(readout.suffixSize)
                    color: root.style.inkMuted
                }
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                visible: readout.stacked
                text: readout.clock.suffix
                textFormat: Text.PlainText
                font: root.style.cap(readout.suffixSize)
                color: root.style.inkMuted
            }
        }
    }

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.domain, "Sun")
        trailing: root.daylight ? i18nd(root.domain, "Day") : i18nd(root.domain, "Night")

        Item {
            id: headerRow

            Layout.fillWidth: true
            implicitHeight: Math.max(riseReadout.implicitHeight, noonReadout.implicitHeight, setReadout.implicitHeight)

            readonly property real noonTickCenter: root.style.snap(
                root.noonFraction * (width - root.style.hairline) + root.style.hairline)
            readonly property real minX: riseReadout.width + root.style.gap
            readonly property real maxX: Math.max(minX, width - setReadout.width - root.style.gap - noonReadout.width)

            SunReadout {
                id: riseReadout

                x: 0
                y: 0
                label: i18nd(root.domain, "Rise")
                clock: root.riseClock
            }

            SunReadout {
                id: noonReadout

                x: root.style.snap(Math.max(headerRow.minX,
                    Math.min(headerRow.maxX, headerRow.noonTickCenter - width / 2)))
                y: 0
                label: i18nd(root.domain, "Noon")
                clock: root.noonClock
            }

            SunReadout {
                id: setReadout

                x: root.style.snap(Math.max(0, headerRow.width - width))
                y: 0
                label: i18nd(root.domain, "Set")
                clock: root.setClock
            }

            Rectangle {
                visible: noonReadout.x - riseReadout.width >= root.style.gapWide
                x: root.style.snap((riseReadout.width + noonReadout.x) / 2)
                y: root.style.snap((headerRow.height - height) / 2)
                width: root.style.hairline
                height: root.style.snap(headerRow.height * 0.82)
                color: root.style.inkFaint
            }

            Rectangle {
                visible: setReadout.x - (noonReadout.x + noonReadout.width) >= root.style.gapWide
                x: root.style.snap((noonReadout.x + noonReadout.width + setReadout.x) / 2)
                y: root.style.snap((headerRow.height - height) / 2)
                width: root.style.hairline
                height: root.style.snap(headerRow.height * 0.82)
                color: root.style.inkFaint
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

            readonly property real railY: root.style.snap(height / 2 - root.style.line2 / 2)
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
                    y: root.style.snap(track.height / 2 - height / 2)
                    width: root.style.hairline * 2
                    height: root.style.snap(track.height * 0.6)
                    color: root.style.inkMuted
                }
            }

            Rectangle {
                visible: root.client.nowEpochS >= root.client.sunriseEpochS
                x: root.style.snap(Math.max(0, Math.min(track.width - width, track.nowX - width / 2)))
                y: 0
                width: root.style.line2 * 1.5
                height: track.height
                color: root.style.backlight
            }
        }
    }
}
