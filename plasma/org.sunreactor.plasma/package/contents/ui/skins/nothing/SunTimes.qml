/*
 * Sun times, Nothing: the three turning points as dot-matrix times, and the
 * day as a row of dots with one red dot for now (between sunrise and sunset,
 * the only red in the card). Lit dots are the day so far; the rest is the
 * matrix's ghost grid. At night the track is all unlit and no dot is red.
 *
 *   RISE          NOON          SET
 *   06:12         13:04         19:51
 *   ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪●▫▫▫▫▫▫▫▫▫▫▫▫▫
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
                readonly property bool isNext: modelData.epoch === root.nextEpoch

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                spacing: root.style.gap * 0.5

                Text {
                    text: column.modelData.label
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                }

                DotText {
                    text: Format.timeText(column.modelData.epoch)
                    face: "tiny"
                    size: 1
                    pitch: root.style.pitch
                    litColor: column.isNext ? root.style.ink : root.style.muted
                    accentColor: root.style.accent
                    dimColor: root.style.textGhost
                }
            }
        }
    }

    // --- the day as a row of dots ----------------------------------------------------
    Item {
        id: track

        Layout.fillWidth: true
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
        font: root.style.plain
        color: root.style.muted
        elide: Text.ElideRight
    }

    // Air below the card's last line, before the footer's hairline.
    Item {
        Layout.preferredHeight: root.style.gap * 0.5
    }
}
