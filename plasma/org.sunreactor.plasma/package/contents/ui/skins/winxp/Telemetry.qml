/*
 * Telemetry, Windows XP: the taskbar. The daemon state sits in the green start
 * pill on the left, the displays' applied brightness runs across the blue band,
 * and the clock sits in the sunken tray at the right end.
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

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected
    readonly property real barH: ui.snap(ui.sizeBody * 2.3)

    width: parent ? parent.width : 0
    implicitHeight: barH

    readonly property string stateText: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18ndc(domain, "Daemon state in the taskbar start pill", "OFFLINE");
        case SunReactorClient.Paused:
            return i18ndc(domain, "Daemon state in the taskbar start pill", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(domain, "Daemon state in the taskbar start pill", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(domain, "Daemon state in the taskbar start pill", "IDLE");
        default:
            return i18ndc(domain, "Daemon state in the taskbar start pill", "LIVE");
        }
    }

    function shortName(logicalId) {
        const head = String(logicalId).split(/[-_ .]/)[0] || String(logicalId);
        return head.slice(0, 4).toUpperCase();
    }

    readonly property string runText: {
        const parts = [];
        for (const monitor of client.monitors) {
            parts.push(monitor.appliedPercent >= 0
                ? i18ndc(domain, "Taskbar: short display name, then its applied brightness", "%1 %2%",
                         shortName(monitor.logicalId), monitor.appliedPercent)
                : i18ndc(domain, "Taskbar: short display name whose brightness is unknown", "%1 —",
                         shortName(monitor.logicalId)));
        }
        if (client.hasTemperature) {
            parts.push(i18ndc(domain, "Temperature in degrees Celsius, compact", "%1°C",
                              Math.round(client.temperatureC)));
        }
        return parts.join("  ·  ");
    }

    // The band.
    Rectangle {
        id: band

        anchors.fill: parent
        border.width: root.ui.line
        border.color: root.ui.frameDeep
        gradient: Gradient {
            GradientStop { position: 0; color: root.ui.frameLight }
            GradientStop { position: 1; color: root.ui.frame }
        }

        Rectangle {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: root.ui.line
            color: root.ui.capBlueTop
        }
    }

    // The green start pill: the state, LIVE when the daemon is following the sun.
    Rectangle {
        id: start

        x: root.ui.snap(root.ui.gap * 0.5)
        y: root.ui.snap((root.barH - height) / 2)
        height: root.ui.snap(root.barH - root.ui.gap)
        width: startLabel.implicitWidth + 2 * root.ui.pad + root.ui.gap
        radius: height / 2
        border.width: root.ui.line
        border.color: root.ui.frameDeep
        gradient: Gradient {
            GradientStop { position: 0; color: root.ui.greenSheen }
            GradientStop { position: 1; color: root.ui.green }
        }

        Text {
            id: startLabel

            anchors.centerIn: parent
            text: root.stateText
            textFormat: Text.PlainText
            font: Qt.font({ family: root.ui.sansBold, weight: Font.DemiBold, italic: true, pixelSize: root.ui.sizeBody })
            color: root.ui.white
        }
    }

    // The applied brightness of each display, and the temperature.
    Text {
        anchors.left: start.right
        anchors.right: tray.left
        anchors.leftMargin: root.ui.pad
        anchors.rightMargin: root.ui.pad
        anchors.verticalCenter: parent.verticalCenter
        text: root.runText
        textFormat: Text.PlainText
        font: root.ui.bodyFont
        color: root.ui.white
        elide: Text.ElideRight
    }

    // The sunken tray with the clock.
    Rectangle {
        id: tray

        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: clock.implicitWidth + 2 * root.ui.pad
        color: root.ui.frameLight
        border.width: root.ui.line
        border.color: root.ui.frameDeep

        Text {
            id: clock

            anchors.centerIn: parent
            visible: root.client.nowEpochS > 0
            text: visible ? Format.timeText(root.client.nowEpochS) : ""
            textFormat: Text.PlainText
            font: root.ui.figureFont
            color: root.ui.white
        }
    }
}
