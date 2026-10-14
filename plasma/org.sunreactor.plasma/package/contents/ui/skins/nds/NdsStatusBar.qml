/*
 * The console's status bar: wifi bars (lit while the daemon answers), the
 * daemon's state word (telemetry only), the battery (lit by the state) and the
 * clock on the right. Drawn with shapes; the words are Nunito Bold.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../Format.js" as Format

RowLayout {
    id: root

    required property Theme theme
    required property NdsStyle style
    required property SunReactorClient client
    /*! Ink for the words and the battery outline. */
    property color ink: style.ink
    /*! Show the daemon's state word between the wifi bars and the battery. */
    property bool showState: false

    readonly property string clock: Format.timeText(client.nowEpochS)
    /*! Battery cells: from none (offline) to all three (live). */
    readonly property int lamp: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return 0;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual:
            return 2;
        case SunReactorClient.IdleDimmed:
            return 1;
        default:
            return 3;
        }
    }
    readonly property string stateWord: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18ndc("plasma_applet_org.sunreactor.plasma", "Daemon state in the status bar", "Offline");
        case SunReactorClient.Paused:
            return i18ndc("plasma_applet_org.sunreactor.plasma", "Daemon state in the status bar", "Paused");
        case SunReactorClient.Manual:
            return i18ndc("plasma_applet_org.sunreactor.plasma", "Daemon state in the status bar", "Manual");
        case SunReactorClient.IdleDimmed:
            return i18ndc("plasma_applet_org.sunreactor.plasma", "Daemon state in the status bar", "Idle");
        default:
            return i18ndc("plasma_applet_org.sunreactor.plasma", "Daemon state in the status bar", "Live");
        }
    }

    spacing: Math.round(style.gu * 0.45)
    implicitHeight: Math.round(style.gu * 1.2)

    Accessible.role: Accessible.StaticText
    Accessible.name: (showState ? stateWord + " " : "") + clock

    // Wifi: three rising bars on a common baseline.
    Item {
        Layout.alignment: Qt.AlignVCenter
        implicitWidth: Math.round(root.style.gu * 0.9)
        implicitHeight: Math.round(root.style.gu * 0.9)

        Repeater {
            model: 3

            delegate: Rectangle {
                required property int index

                x: index * 4
                y: parent.height - height
                width: 3
                height: Math.round(parent.height * (0.4 + 0.3 * index))
                radius: 1
                color: root.client.isConnected ? root.ink : root.style.line
            }
        }
    }

    Text {
        visible: root.showState
        Layout.alignment: Qt.AlignVCenter
        text: root.stateWord
        font.family: root.style.family
        font.weight: Font.Bold
        font.pixelSize: Math.round(root.style.gu * 0.62)
        color: root.theme.stateColor(root.client.mode)
    }

    Item { Layout.fillWidth: true }

    // Battery: a rounded outline, a nub, and three cells lit by the state.
    Item {
        Layout.alignment: Qt.AlignVCenter
        implicitWidth: Math.round(root.style.gu * 1.5)
        implicitHeight: Math.round(root.style.gu * 0.72)

        Rectangle {
            x: 0
            y: 0
            width: parent.width - 3
            height: parent.height
            radius: 3
            color: "transparent"
            border.width: 1.5
            border.color: root.ink
        }
        Rectangle {
            x: parent.width - 3
            y: Math.round(parent.height / 3)
            width: 2
            height: Math.round(parent.height / 3)
            color: root.ink
        }
        Repeater {
            model: 3

            delegate: Rectangle {
                required property int index

                x: 3 + index * Math.round((parent.width - 9 - 2) / 3)
                y: 3
                width: Math.round((parent.width - 9 - 2) / 3) - 2
                height: parent.height - 6
                radius: 1
                color: index < root.lamp ? root.theme.stateColor(root.client.mode) : root.style.line
            }
        }
    }

    Text {
        Layout.alignment: Qt.AlignVCenter
        text: root.clock
        font.family: root.style.family
        font.weight: Font.Bold
        font.pixelSize: Math.round(root.style.gu * 0.78)
        color: root.ink
    }
}
