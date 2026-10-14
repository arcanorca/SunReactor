/*
 * Telemetry, cyberpunk: the daemon state as a slim strip above the tabs. A
 * lamp and the state word, the system and per-display readouts in brackets, a
 * tick ruler, then the outside temperature and the daemon's clock.
 *
 *   ■ LIVE  SYS.LUM [39]  MI.LUM [39]  ▏▏▏▏▏▏▏▏▏▏▏▏▏▏▏▏   15°C  [14:30]
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve
import "Geometry.js" as G

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected
    readonly property CyberStyle style: CyberStyle {
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
    /*! Override and offline are the red warnings; paused is the yellow alert. */
    readonly property color stateColor: {
        switch (client.mode) {
        case SunReactorClient.Offline:
        case SunReactorClient.Manual:
            return style.red;
        case SunReactorClient.Paused:
            return style.yellow;
        default:
            return style.cyan;
        }
    }

    /*! SYS.LUM, then one readout per display; "--" where nothing is known yet. */
    readonly property var channels: {
        const out = [{ label: "SYS.LUM", value: client.globalPercent >= 0 ? String(client.globalPercent) : "--" }];
        for (const monitor of client.monitors) {
            out.push({
                label: Curve.shortName(monitor.logicalId) + ".LUM",
                value: monitor.appliedPercent >= 0 ? String(monitor.appliedPercent) : "--"
            });
        }
        return out;
    }

    spacing: 0
    Accessible.role: Accessible.StaticText
    Accessible.name: {
        const parts = [stateText];
        for (const channel of channels) {
            parts.push(channel.label + " " + channel.value);
        }
        if (client.hasTemperature) {
            parts.push(Math.round(client.temperatureC) + "°C");
        }
        parts.push(Format.timeText(client.nowEpochS));
        return parts.join(", ");
    }

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        hatched: false
        visible: root.available

        // A flow, not a row: with many displays the readouts wrap onto a second line
        // rather than clip or push the clock off the strip.
        Flow {
            Layout.fillWidth: true
            spacing: root.style.gap2

            Row {
                Layout.alignment: Qt.AlignVCenter
                spacing: root.style.gap

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: root.style.snap(6)
                    height: width
                    color: root.stateColor
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.stateText
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: root.stateColor
                }
            }

            Repeater {
                model: root.channels
                delegate: Row {
                    id: readout
                    required property var modelData
                    Layout.alignment: Qt.AlignVCenter
                    spacing: root.style.gap
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: readout.modelData.label
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.cyan
                    }
                    Bracket {
                        anchors.verticalCenter: parent.verticalCenter
                        style: root.style
                        value: readout.modelData.value
                        font: root.style.techFont
                        color: root.style.ink
                    }
                }
            }
            // Hex-like tick ruler: a short run between the readouts and the clock.
            Item {
                id: ruler
                width: root.style.snap(root.style.unit * 1.2)
                height: root.style.snap(root.style.unit * 0.7)
                Layout.alignment: Qt.AlignVCenter
                Shape {
                    anchors.fill: parent
                    ShapePath {
                        strokeColor: root.style.rule
                        strokeWidth: root.style.hairline
                        fillColor: "transparent"
                        PathSvg { path: G.ticks(ruler.width, 8, 5, 2, 4) }
                    }
                }
            }
            Text {
                visible: root.client.hasTemperature
                Layout.alignment: Qt.AlignVCenter
                text: i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1°C",
                             Math.round(root.client.temperatureC))
                textFormat: Text.PlainText
                font: root.style.techFont
                color: root.style.muted
            }
            Bracket {
                Layout.alignment: Qt.AlignVCenter
                style: root.style
                value: Format.timeText(root.client.nowEpochS)
                font: root.style.techFont
                color: root.style.ink
            }
        }
    }
}
