/*
 * Telemetry, instrument family: the TUI status line as an instrument legend.
 *
 *   ● LIVE │ MI 39 │ LEN 48                         15 °C │ 14:30
 *
 * Lamp and state word in the daemon-state colour, one channel readout per
 * display (short caps name, tabular value, "—" when unknown), outside
 * temperature and the daemon's clock at the right. Emissive flavours show it
 * on a slim strip of display glass.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Curve.js" as Curve

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }

    readonly property string stateText: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18ndc(domain, "Daemon state in the status line", "OFFLINE");
        case SunReactorClient.Paused:
            return i18ndc(domain, "Daemon state in the status line", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(domain, "Daemon state in the status line", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(domain, "Daemon state in the status line", "IDLE");
        default:
            return i18ndc(domain, "Daemon state in the status line", "LIVE");
        }
    }

    function shortName(logicalId) {
        return Curve.shortName(logicalId);
    }

    implicitHeight: window.implicitHeight
    implicitWidth: window.implicitWidth

    Accessible.role: Accessible.StaticText
    Accessible.name: {
        const parts = [stateText];
        for (const monitor of client.monitors) {
            parts.push(shortName(monitor.logicalId) + " "
                       + (monitor.appliedPercent >= 0 ? monitor.appliedPercent + "%" : "—"));
        }
        if (client.hasTemperature) {
            parts.push(Math.round(client.temperatureC) + "°C");
        }
        parts.push(Format.timeText(client.nowEpochS));
        return parts.join(", ");
    }

    DisplayWindow {
        id: window

        width: parent.width
        style: root.style
        padding: glass ? root.style.snap(root.style.gap * 1.25) : 0

        Item {
            id: strip

            implicitHeight: root.style.snap(Math.max(dataMetrics.height, root.style.sizeLegend * 1.6))
            Layout.fillWidth: true

            TextMetrics {
                id: dataMetrics
                font: root.style.dataFont
                text: "00"
            }

            Glow {
                anchors.fill: line
                source: line
                strength: root.style.emissive ? root.style.glow * 0.6 : 0
            }

            Item {
                id: line

                anchors.fill: parent

                Row {
                    id: leftPart

                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, parent.width - rightPart.implicitWidth - root.style.gap2)
                    clip: true
                    spacing: root.style.gap2

                    Row {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: root.style.gap

                        Lamp {
                            anchors.verticalCenter: parent.verticalCenter
                            style: root.style
                            color: root.style.stateTone(root.client.mode)
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.stateText
                            textFormat: Text.PlainText
                            font: root.style.legendFont
                            color: root.style.stateTone(root.client.mode)
                        }
                    }

                    Repeater {
                        model: root.client.monitors

                        delegate: Row {
                            id: channel

                            required property var modelData

                            anchors.verticalCenter: parent.verticalCenter
                            spacing: root.style.gap2

                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                width: root.style.hairline
                                height: root.style.snap(root.style.sizeLegend * 1.2)
                                color: root.style.rule
                            }

                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: root.style.snap(root.style.gap * 0.75)

                                Text {
                                    anchors.baseline: value.baseline
                                    text: root.shortName(channel.modelData.logicalId)
                                    textFormat: Text.PlainText
                                    font: root.style.legendFont
                                    color: root.style.inkMuted
                                }

                                Text {
                                    id: value

                                    text: channel.modelData.appliedPercent >= 0
                                        ? String(channel.modelData.appliedPercent)
                                        : i18ndc(root.domain, "No value is known yet", "—")
                                    textFormat: Text.PlainText
                                    font: root.style.dataFont
                                    color: root.style.ink
                                }
                            }
                        }
                    }
                }

                Row {
                    id: rightPart

                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: root.style.gap2

                    Text {
                        visible: root.client.hasTemperature
                        anchors.verticalCenter: parent.verticalCenter
                        text: i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1°C",
                                     Math.round(root.client.temperatureC))
                        textFormat: Text.PlainText
                        font: root.style.dataFont
                        color: root.style.inkMuted
                    }

                    Rectangle {
                        visible: root.client.hasTemperature && clock.text.length > 0
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.style.hairline
                        height: root.style.snap(root.style.sizeLegend * 1.2)
                        color: root.style.rule
                    }

                    Text {
                        id: clock

                        anchors.verticalCenter: parent.verticalCenter
                        text: Format.timeText(root.client.nowEpochS)
                        textFormat: Text.PlainText
                        font: root.style.dataFont
                        color: root.style.ink
                    }
                }
            }
        }
    }
}
