/*
 * The sun times on one line, cyberpunk, without a frame: rise, noon and set in
 * brackets, the next one in the accent. The day's length goes in the panel's tag
 * when SunTimes.qml frames the line; the weather panel shows the line alone.
 *
 *   RISE [07:13]    NOON [12:55]    SET [18:38]
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
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property real dayLength: client.sunsetEpochS - client.sunriseEpochS
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation
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

    spacing: 0

    Flow {
        Layout.fillWidth: true
        spacing: root.style.gap

        Repeater {
            model: root.available ? root.entries : []

            delegate: Row {
                id: entry

                required property var modelData
                readonly property bool isNext: modelData.epoch === root.nextEpoch

                spacing: root.style.gap * 0.5

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: entry.modelData.label
                    textFormat: Text.PlainText
                    font: root.style.tinyFont
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
            visible: root.showElevation && root.available
            spacing: root.style.gap * 0.5

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
