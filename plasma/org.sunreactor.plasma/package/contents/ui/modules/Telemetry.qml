/*
 * Telemetry module: the TUI's status line in one row -
 * "● LIVE · MI 5% · LEN 9% · 15°C · 14:30". The dot is the daemon state in the
 * theme's success / warning / error colour. Pixel themes draw it in a
 * PixelSurface with the bitmap font; the others as text.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../Format.js" as Format

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected

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
        const head = String(logicalId).split(/[-_ .]/)[0] || String(logicalId);
        return head.slice(0, 4).toUpperCase();
    }

    readonly property string lineText: {
        const parts = [stateText];
        for (const monitor of client.monitors) {
            parts.push(monitor.appliedPercent >= 0
                ? i18ndc(domain, "Short display name, then its applied brightness", "%1 %2%",
                         shortName(monitor.logicalId), monitor.appliedPercent)
                : i18ndc(domain, "Short display name whose brightness is unknown", "%1 —",
                         shortName(monitor.logicalId)));
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

    implicitWidth: theme.isPixel ? pixelLine.implicitWidth : vectorLine.implicitWidth
    implicitHeight: theme.isPixel ? pixelLine.implicitHeight : vectorLine.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: lineText

    // --- pixel renderer ------------------------------------------------------
    PixelSurface {
        id: pixelLine

        visible: root.theme.isPixel
        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: glyphs.artHeight + 4
        regionName: "telemetry"

        Rectangle {
            width: pixelLine.artWidth
            height: pixelLine.artHeight
            color: root.theme.bg
        }

        Rectangle {
            x: 2
            y: Math.floor((pixelLine.artHeight - height) / 2)
            width: 4
            height: 4
            color: root.theme.stateColor(root.client.mode)
        }

        BitmapText {
            id: glyphs

            x: 9
            y: 2
            theme: root.theme
            inSurface: true
            text: root.lineText
            maximumArtWidth: Math.max(0, pixelLine.artWidth - 10)
        }
    }

    // --- vector / terminal renderer -------------------------------------------
    RowLayout {
        id: vectorLine

        visible: !root.theme.isPixel
        width: parent.width
        spacing: root.theme.spacing

        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: root.theme.spacing * 2
            implicitHeight: implicitWidth
            radius: root.theme.isVector ? width / 2 : 0
            color: root.theme.stateColor(root.client.mode)
        }

        PlasmaComponents3.Label {
            Layout.fillWidth: true
            text: root.lineText
            textFormat: Text.PlainText
            font.family: root.theme.digitsFont
            font.pixelSize: root.theme.fontSizeBody
            font.features: ({ "tnum": 1 })
            color: root.theme.fg
            elide: Text.ElideRight
        }
    }
}
