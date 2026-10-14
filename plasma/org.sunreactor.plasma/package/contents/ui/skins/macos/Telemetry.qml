/*
 * Telemetry, macos: the status line. It is hidden by default (the config's
 * hidden modules); the status lamp in the tab row stands for it then.
 *
 *   ● Live      MI 39%  ·  LEN 48%                 14 °   14:30
 *
 * The lamp is the daemon's state in the palette's traffic-light colours: green
 * automatic, yellow manual or paused, red offline. Nothing else in the skin is
 * coloured that way, so the lamp reads as status, not as window chrome.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve

RowLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected

    /*! The daemon's state as one word in sentence case. */
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
            return i18ndc(domain, "Daemon state in the status line", "Live");
        }
    }

    /*! "MI 39%  ·  LEN 48%": one readout per display, "—" where unknown. */
    readonly property string readouts: {
        const parts = [];
        for (const monitor of client.monitors) {
            parts.push(Curve.shortName(monitor.logicalId) + " "
                       + (monitor.appliedPercent >= 0 ? monitor.appliedPercent + "%" : "—"));
        }
        return parts.join("  ·  ");
    }

    spacing: root.style.gap

    Accessible.role: Accessible.StaticText
    Accessible.name: [stateText, readouts, client.hasTemperature ? Math.round(client.temperatureC) + "°C" : "",
                      Format.timeText(client.nowEpochS)].filter(part => part.length > 0).join(", ")

    Rectangle {
        Layout.alignment: Qt.AlignVCenter
        implicitWidth: root.style.snap(8)
        implicitHeight: implicitWidth
        radius: width / 2
        antialiasing: true
        color: root.theme.stateColor(root.client.mode)
    }

    Text {
        Layout.alignment: Qt.AlignVCenter
        text: root.stateText
        textFormat: Text.PlainText
        font: root.style.captionMedium
        color: root.style.ink
    }

    Text {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignVCenter
        text: root.readouts
        textFormat: Text.PlainText
        font: root.style.caption
        color: root.style.inkSecondary
        elide: Text.ElideRight
        horizontalAlignment: Text.AlignRight
    }

    Text {
        Layout.alignment: Qt.AlignVCenter
        visible: root.client.hasTemperature
        text: visible ? i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1 °C",
                               Math.round(root.client.temperatureC)) : ""
        textFormat: Text.PlainText
        font: root.style.caption
        color: root.style.ink
    }

    Text {
        Layout.alignment: Qt.AlignVCenter
        text: Format.timeText(root.client.nowEpochS)
        textFormat: Text.PlainText
        font: root.style.value
        color: root.style.inkSecondary
    }
}
