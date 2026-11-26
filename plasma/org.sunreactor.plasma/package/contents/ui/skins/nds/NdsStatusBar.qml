/*
 * The Nintendo DS console's status bar: ascending pixel signal bars (proximity
 * to configured max brightness), optional state word (telemetry only), solar-
 * synced pixel battery (charges sunrise->sunset, drains to 0 before sunrise)
 * and clock on the right, sitting cleanly inside the sky screen.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../Format.js" as Format
import "NdsStatusRules.js" as StatusRules

RowLayout {
    id: root

    required property Theme theme
    required property NdsStyle style
    required property SunReactorClient client
    /*! Ink for the clock and status text. */
    property color ink: style.ink
    /*! Show the daemon's state word between the signal bars and the battery. */
    property bool showState: false

    readonly property string clock: Format.timeText(client.nowEpochS)
    readonly property int signalLevel: StatusRules.signalBars(
        client.isConnected || client.statusStale, client.monitors, client.globalPercent)
    readonly property int batteryLevel: StatusRules.solarBatteryLevel(
        client.isConnected || client.statusStale, client.nowEpochS, client.sunriseEpochS, client.sunsetEpochS, client.isDaylight)
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

    Image {
        Layout.alignment: Qt.AlignVCenter
        Layout.preferredWidth: 16
        Layout.preferredHeight: 12
        source: Qt.resolvedUrl(root.signalLevel < 0
            ? "assets/ds_signal_off.png"
            : ("assets/ds_signal_" + root.signalLevel + ".png"))
        fillMode: Image.PreserveAspectFit
        smooth: false
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

    Image {
        Layout.alignment: Qt.AlignVCenter
        Layout.preferredWidth: 20
        Layout.preferredHeight: 10
        source: Qt.resolvedUrl(root.batteryLevel < 0
            ? "assets/ds_battery_off.png"
            : ("assets/ds_battery_" + root.batteryLevel + ".png"))
        fillMode: Image.PreserveAspectFit
        smooth: false
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
