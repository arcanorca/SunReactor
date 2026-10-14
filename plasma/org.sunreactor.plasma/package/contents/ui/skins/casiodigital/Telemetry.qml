/*
 * Telemetry, casiodigital: the top of the popup, like the first line of a watch
 * face. The mode is three printed LCD flags (AUTO, MAN, PAUSE; the lit one is the
 * state), the clock in seven segments, and the temperature beside it.
 *
 *   [AUTO]  MAN  PAUSE                     14:32 PM   21 C
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
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected
    readonly property var clock: Casio.clockParts(Format.timeText(client.nowEpochS))
    readonly property string temperature: client.hasTemperature
        ? Casio.numberText(client.temperatureC, 3) : "---"

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style

        Flow {
            Layout.fillWidth: true
            spacing: root.style.gap

            CasioFlag {
                style: root.style
                text: "AUTO"
                lit: root.control.choice === "automatic"
            }
            CasioFlag {
                style: root.style
                text: "MAN"
                lit: root.control.choice === "manual"
            }
            CasioFlag {
                style: root.style
                text: "PAUSE"
                lit: root.control.choice === "paused"
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gapWide

            CasioDigits {
                style: root.style
                size: root.style.digitMedium
                text: root.clock.digits
                Accessible.name: root.clock.digits.trim()
            }
            Text {
                Layout.alignment: Qt.AlignBottom
                visible: root.clock.suffix.length > 0
                text: root.clock.suffix
                textFormat: Text.PlainText
                font: root.style.cap(root.style.sizeCap)
                color: root.style.inkMuted
            }

            Item {
                Layout.fillWidth: true
            }

            CasioDigits {
                style: root.style
                size: root.style.digitSmall
                text: root.temperature
            }
            Text {
                Layout.alignment: Qt.AlignBottom
                text: "°C"
                textFormat: Text.PlainText
                font: root.style.cap(root.style.sizeCap)
                color: root.style.inkMuted
            }
        }
    }
}
