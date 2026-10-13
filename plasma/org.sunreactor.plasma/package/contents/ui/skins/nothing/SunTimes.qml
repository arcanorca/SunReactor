/*
 * Sun times, Nothing: the three turning points as mono times under their labels,
 * and the day as a dotted track. Lit dots are the day so far, the head is the one
 * red dot for now (between sunrise and sunset, the only red in the card). At night
 * the track is all unlit and no dot is red. Times are text, never dot numerals:
 * AM and PM would collide in the dot face.
 *
 *   RISE          NOON          SET
 *   7:12 AM       12:55 PM      6:33 PM
 *   ●●●●●●●●●●●●●●●●●●●●●●●●●●●●●○○○○○○○○○○○○○○○○○○○○○○○○
 *   sun 23.4° above the horizon
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

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { label: i18ndc(root.domain, "Short legend: sunrise", "RISE"), epoch: client.sunriseEpochS },
        { label: i18ndc(root.domain, "Short legend: solar noon", "NOON"), epoch: client.solarNoonEpochS },
        { label: i18ndc(root.domain, "Short legend: sunset", "SET"), epoch: client.sunsetEpochS },
    ]
    /*! Epoch of the next event still ahead today, or 0 once the day is done. */
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }
    /*! How far the day has run, 0 at sunrise and 1 at sunset. */
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
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    Layout.fillWidth: true
    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18nd(root.domain, "Sun times")

    // --- the three times ------------------------------------------------------------
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        Repeater {
            model: root.entries

            delegate: ColumnLayout {
                id: column

                required property var modelData
                required property int index
                readonly property bool isNext: modelData.epoch === root.nextEpoch
                /*! Rise flush left, noon centred, set flush right: the times never run together. */
                readonly property int align: index === 0 ? Text.AlignLeft
                    : (index === 1 ? Text.AlignHCenter : Text.AlignRight)

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                spacing: root.style.gap * 0.5

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: column.align
                    text: column.modelData.label
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                }

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: column.align
                    text: Format.timeText(column.modelData.epoch)
                    textFormat: Text.PlainText
                    font: root.style.figure
                    color: column.isNext ? root.style.ink : root.style.muted
                }
            }
        }
    }

    // --- the day as a dotted track ---------------------------------------------------
    Item {
        id: track

        Layout.fillWidth: true
        Layout.topMargin: root.style.gap * 0.5
        implicitHeight: bar.implicitHeight

        DotBar {
            id: bar

            anchors.left: parent.left
            anchors.right: parent.right
            cols: Math.max(12, Math.floor(track.width / root.style.pitch))
            rows: 1
            pitch: root.style.pitch
            value: root.dayProgress
            thumbAccent: root.daylight
            litColor: root.style.ink
            accentColor: root.style.accent
            dimColor: root.style.dotGhost
        }
    }

    Text {
        Layout.fillWidth: true
        visible: root.showElevation
        text: (root.client.solarElevation >= 0
            ? i18ndc(root.domain, "Placeholder is an angle in degrees",
                     "Sun %1° above the horizon", root.client.solarElevation.toFixed(1))
            : i18ndc(root.domain, "Placeholder is an angle in degrees",
                     "Sun %1° below the horizon", (-root.client.solarElevation).toFixed(1))).toLowerCase()
        textFormat: Text.PlainText
        font: root.style.sentence
        color: root.style.muted
        elide: Text.ElideRight
    }
}
