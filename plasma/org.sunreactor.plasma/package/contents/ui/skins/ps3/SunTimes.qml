/*
 * Sun times, PlayStation 3: sunrise, solar noon and sunset as three XMB items,
 * and the day as a thin line under them: the part of the day gone is bright,
 * with the sun's place now as a glowing dot.
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

    // The PS3 side margin: the module's content keeps clear of the popup's edges.
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.leftMargin: root.style.sideMargin
    anchors.rightMargin: root.style.sideMargin

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { label: i18ndc(root.domain, "Short legend: sunrise", "Rise"), epoch: client.sunriseEpochS },
        { label: i18ndc(root.domain, "Short legend: solar noon", "Noon"), epoch: client.solarNoonEpochS },
        { label: i18ndc(root.domain, "Short legend: sunset", "Set"), epoch: client.sunsetEpochS },
    ]
    /*! How far the day has run: 0 at sunrise, 1 at sunset. */
    readonly property real dayProgress: {
        const rise = client.sunriseEpochS;
        const set = client.sunsetEpochS;
        if (rise <= 0 || set <= rise) {
            return 0;
        }
        return Math.max(0, Math.min(1, (client.nowEpochS - rise) / (set - rise)));
    }
    readonly property bool daylight: client.nowEpochS >= client.sunriseEpochS
        && client.nowEpochS < client.sunsetEpochS

    Layout.fillWidth: true
    spacing: style.gap * 0.5

    // The three times on one row: each a legend and its value, left, centre, right.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        Repeater {
            model: root.entries

            delegate: Item {
                id: entry

                required property var modelData
                required property int index

                Layout.fillWidth: true
                implicitHeight: pair.implicitHeight

                Row {
                    id: pair

                    x: entry.index === 0 ? 0 : (entry.index === 1 ? (entry.width - pair.width) / 2
                                                        : entry.width - pair.width)
                    spacing: root.style.gap

                    Text {
                        anchors.baseline: time.baseline
                        text: entry.modelData.label
                        textFormat: Text.PlainText
                        font: root.style.legendFont
                        color: root.style.inkSoft
                    }

                    Text {
                        id: time

                        text: Format.timeText(entry.modelData.epoch)
                        textFormat: Text.PlainText
                        font: root.style.valueFont
                        color: root.style.ink
                        lineHeightMode: Text.FixedHeight
                        lineHeight: root.style.sizeBody
                    }
                }
            }
        }
    }

    // The day as a thin line, sunrise to sunset; the sun's place is a bright dot.
    // The band is as tall as the dot, and a gap of room sits under it, so the
    // dot never falls below the module's clip.
    Item {
        id: track

        Layout.fillWidth: true
        Layout.bottomMargin: root.style.gap
        implicitHeight: root.style.snap(root.style.unit * 0.36)

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width
            height: root.style.line2
            radius: height / 2
            color: root.style.track
        }

        Rectangle {
            visible: root.daylight
            anchors.verticalCenter: parent.verticalCenter
            width: root.style.snap(track.width * root.dayProgress)
            height: root.style.line2
            radius: height / 2
            color: root.style.ink
        }

        Rectangle {
            visible: root.daylight
            x: root.style.snap(track.width * root.dayProgress) - width / 2
            anchors.verticalCenter: parent.verticalCenter
            width: root.style.snap(root.style.unit * 0.36)
            height: width
            radius: width / 2
            color: root.style.ink
        }
    }
}
