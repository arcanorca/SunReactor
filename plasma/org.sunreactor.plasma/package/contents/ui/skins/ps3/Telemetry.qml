/*
 * Telemetry, PlayStation 3: the status line, a small row of white text with a
 * status dot in front: green while following the sun, blue while the screen is
 * idle-dimmed, amber when set by hand or paused, red when the daemon is offline.
 * Hidden by default (the status lamp stands in for it).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

RowLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected
    readonly property XmbStyle style: XmbStyle {
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

    readonly property color dotColor: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return style.danger;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual:
            return style.warning;
        case SunReactorClient.IdleDimmed:
            return style.accent;
        default:
            return style.ok;
        }
    }

    readonly property string lineText: {
        const parts = [stateText];
        for (const monitor of client.monitors) {
            const head = String(monitor.logicalId).split(/[-_ .]/)[0] || String(monitor.logicalId);
            parts.push(monitor.appliedPercent >= 0
                ? i18ndc(domain, "Short display name, then its applied brightness", "%1 %2%",
                         head.slice(0, 4).toUpperCase(), monitor.appliedPercent)
                : i18ndc(domain, "Short display name whose brightness is unknown", "%1 —",
                         head.slice(0, 4).toUpperCase()));
        }
        if (client.hasTemperature) {
            parts.push(i18ndc(domain, "Temperature in degrees Celsius, compact", "%1°C",
                              Math.round(client.temperatureC)));
        }
        if (client.nowEpochS > 0) {
            parts.push(Format.timeText(client.nowEpochS));
        }
        return parts.join(" · ");
    }

    spacing: root.style.gap

    Accessible.role: Accessible.StaticText
    Accessible.name: lineText

    // The status dot with its halo: the halo is drawn first, under the dot.
    Item {
        Layout.alignment: Qt.AlignVCenter
        implicitWidth: root.style.snap(root.style.unit * 0.9)
        implicitHeight: implicitWidth

        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: root.style.alpha(root.dotColor, 0.22)
        }
        Rectangle {
            anchors.centerIn: parent
            width: root.style.snap(root.style.unit * 0.4)
            height: width
            radius: width / 2
            color: root.dotColor
        }
    }

    Text {
        Layout.fillWidth: true
        text: root.lineText
        textFormat: Text.PlainText
        font: root.style.legendFont
        color: root.style.alpha(root.style.ink, 0.85)
        elide: Text.ElideRight
    }
}
