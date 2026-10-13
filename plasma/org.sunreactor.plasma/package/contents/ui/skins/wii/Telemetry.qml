/*
 * Telemetry, Wii style: the status line as a slim rounded bar. A blue lamp
 * and the daemon's state, each display's brightness, the outside temperature
 * and the clock, in dark slate. Hidden by default (the config's hiddenModules);
 * the status lamp of the tab row stands in for it.
 *
 *   (•) Automatic   MI 39   LEN 48                    15 °C   14:30
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property WiiStyle style: WiiStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected

    readonly property string stateText: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18ndc(domain, "Daemon state in the status line", "Offline");
        case SunReactorClient.Paused:
            return i18ndc(domain, "Daemon state in the status line", "Paused");
        case SunReactorClient.Manual:
            return i18ndc(domain, "Daemon state in the status line", "Manual");
        case SunReactorClient.IdleDimmed:
            return i18ndc(domain, "Daemon state in the status line", "Idle");
        default:
            return i18ndc(domain, "Daemon state in the status line", "Automatic");
        }
    }
    readonly property color lampColor: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return style.danger;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual:
            return style.sun;
        default:
            return style.accent;
        }
    }

    implicitWidth: bar.implicitWidth
    implicitHeight: bar.implicitHeight

    Rectangle {
        id: bar

        readonly property real barHeight: root.style.snap(root.style.unit * 1.7)

        anchors.left: parent.left
        anchors.right: parent.right
        implicitWidth: row.implicitWidth + 2 * height * 0.5
        implicitHeight: barHeight
        height: barHeight
        radius: height / 2
        antialiasing: true
        color: root.style.face
        border.width: root.style.line
        border.color: root.style.outline

        RowLayout {
            id: row

            anchors.fill: parent
            anchors.leftMargin: bar.height * 0.5
            anchors.rightMargin: bar.height * 0.5
            spacing: root.style.gap

            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: root.style.snap(root.style.unit * 0.5)
                implicitHeight: implicitWidth
                radius: width / 2
                antialiasing: true
                color: root.lampColor
            }
            Text {
                Layout.alignment: Qt.AlignVCenter
                text: root.stateText
                textFormat: Text.PlainText
                font: root.style.bodyBoldFont
                color: root.style.ink
            }

            Repeater {
                model: root.client.monitors

                delegate: RowLayout {
                    id: channel

                    required property var modelData

                    Layout.alignment: Qt.AlignVCenter
                    spacing: root.style.snap(root.style.gap * 0.5)

                    Text {
                        text: Curve.shortName(channel.modelData.logicalId)
                        textFormat: Text.PlainText
                        font: root.style.captionBoldFont
                        color: root.style.inkSoft
                    }
                    Text {
                        text: channel.modelData.percent >= 0 ? String(Math.round(channel.modelData.percent)) : "--"
                        textFormat: Text.PlainText
                        font: root.style.bodyBoldFont
                        color: root.style.ink
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }
            Text {
                Layout.alignment: Qt.AlignVCenter
                visible: root.client.hasTemperature
                text: i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1 °C",
                             Math.round(root.client.temperatureC))
                textFormat: Text.PlainText
                font: root.style.bodyFont
                color: root.style.ink
            }
            Text {
                Layout.alignment: Qt.AlignVCenter
                text: Format.timeText(root.client.nowEpochS)
                textFormat: Text.PlainText
                font: root.style.captionBoldFont
                color: root.style.inkSoft
            }
        }
    }
}
