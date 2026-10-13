/*
 * Telemetry, synthwave84: the status line above the tabs.
 *
 *   ● LIVE   MI 39   LEN 48                        15 °C   14:30
 *
 * A glowing lamp in the daemon's state colour and the state word, one channel
 * readout per display (short caps name in cyan, tabular value), the outside
 * temperature and the daemon's clock at the right. A neon hairline closes it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve
import "../instrument" as Instrument

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected

    readonly property color tone: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return style.red;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual:
            return style.yellow;
        case SunReactorClient.IdleDimmed:
            return style.cyan;
        default:
            return style.mint;
        }
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

    spacing: style.gap

    Accessible.role: Accessible.StaticText
    Accessible.name: {
        const parts = [stateText];
        for (const monitor of client.monitors) {
            parts.push(Curve.shortName(monitor.logicalId) + " "
                       + (monitor.appliedPercent >= 0 ? monitor.appliedPercent + "%" : "—"));
        }
        if (client.hasTemperature) {
            parts.push(Math.round(client.temperatureC) + "°C");
        }
        parts.push(Format.timeText(client.nowEpochS));
        return parts.join(", ");
    }

    // The margin above the status line: the popup's top edge is the frame.
    Item {
        Layout.preferredHeight: root.style.gap2
    }

    // A flow, not a row: with many displays the readouts wrap onto a second line
    // rather than push the clock off the strip.
    Flow {
        Layout.fillWidth: true
        spacing: root.style.gap2

        // The lamp and the state word.
        Item {
            id: lampSlot

            Layout.alignment: Qt.AlignVCenter
            implicitWidth: lamp.width + root.style.gap
            implicitHeight: lampLabel.implicitHeight

            Rectangle {
                id: lamp

                x: 0
                anchors.verticalCenter: parent.verticalCenter
                width: root.style.snap(root.style.sizeLabel * 0.62)
                height: width
                radius: width / 2
                color: root.tone
            }
            Instrument.Glow {
                anchors.fill: lamp
                source: lamp
                strength: root.style.glow
                tint: root.tone
            }
        }

        Text {
            id: lampLabel

            Layout.alignment: Qt.AlignVCenter
            text: root.stateText
            textFormat: Text.PlainText
            font: root.style.labelFont
            color: root.tone
        }

        // One readout per display.
        Repeater {
            model: root.client.isConnected ? root.client.monitors : []

            delegate: RowLayout {
                id: readout

                required property var modelData

                spacing: root.style.gap

                Text {
                    text: Curve.shortName(readout.modelData.logicalId)
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: root.style.cyan
                }
                Text {
                    text: readout.modelData.appliedPercent >= 0 ? readout.modelData.appliedPercent + "%" : "—"
                    textFormat: Text.PlainText
                    font: root.style.dataFont
                    color: root.style.ink
                }
            }
        }

        Text {
            visible: root.client.hasTemperature
            text: i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1 °C",
                         Math.round(root.client.temperatureC))
            textFormat: Text.PlainText
            font: root.style.dataFont
            color: root.style.ink
        }

        Text {
            text: Format.timeText(root.client.nowEpochS)
            textFormat: Text.PlainText
            font: root.style.dataFont
            color: root.style.pink
        }
    }

    // The neon hairline that closes the status line.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: root.style.hairline
        color: root.style.alpha(root.style.cyan, 0.4)
    }
}
