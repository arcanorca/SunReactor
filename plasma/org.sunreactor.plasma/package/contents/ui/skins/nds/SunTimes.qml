/*
 * Nintendo DS touch screen, sun times: a white pill under the forecast with
 * sunrise, solar noon and sunset - each a small orange sun and its time in the
 * console's rounded digits - the next one in DS blue. Module contract:
 * modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: (client.isConnected || client.statusStale) && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { label: i18ndc(domain, "Sunrise, compact", "Rise"), epoch: client.sunriseEpochS },
        { label: i18ndc(domain, "Solar noon, compact", "Noon"), epoch: client.solarNoonEpochS },
        { label: i18ndc(domain, "Sunset, compact", "Set"), epoch: client.sunsetEpochS }
    ]
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }

    readonly property real pillH: Math.round(style.gu * 1.9)

    implicitHeight: pill.height

    Accessible.role: Accessible.StaticText
    Accessible.name: entries.map(e => e.label + " " + Format.timeText(e.epoch)).join(", ")

    NdsPanel {
        id: pill

        style: root.style
        kind: "panel"
        width: root.width
        height: root.pillH

        Row {
            anchors.fill: parent

            Repeater {
                model: root.entries

                delegate: Item {
                    id: column

                    required property var modelData

                    readonly property bool isNext: modelData.epoch === root.nextEpoch

                    width: pill.width / root.entries.length
                    height: pill.height

                    Accessible.role: Accessible.ListItem
                    Accessible.name: modelData.label + " " + Format.timeText(modelData.epoch)

                    Row {
                        anchors.centerIn: parent
                        spacing: Math.round(root.style.gu * 0.22)

                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            width: Math.round(root.style.gu * 0.4)
                            height: width
                            radius: 0
                            color: root.style.orange
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: column.modelData.label
                            textFormat: Text.PlainText
                            font.family: root.style.family
                            font.pixelSize: Math.round(root.style.gu * 0.58)
                            color: root.style.muted
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: Format.timeText(column.modelData.epoch)
                            textFormat: Text.PlainText
                            font.family: root.style.family
                            font.weight: Font.Bold
                            font.pixelSize: Math.round(root.style.gu * 0.76)
                            color: column.isNext ? root.style.blue : root.style.ink
                        }
                    }
                }
            }
        }
    }
}
