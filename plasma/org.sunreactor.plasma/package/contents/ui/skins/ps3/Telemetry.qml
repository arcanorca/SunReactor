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
import "../../components"

Item {
    id: root

    // The PS3 side margin: the module's content keeps clear of the popup's edges.
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.leftMargin: root.style.sideMargin
    anchors.rightMargin: root.style.sideMargin

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected || client.statusStale
    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }

    readonly property string stateText: stateWords.label(client.mode)

    StateText {
        id: stateWords
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
            parts.push(Format.tempText(client.temperatureC, client.isFahrenheit));
        }
        if (client.nowEpochS > 0) {
            parts.push(Format.timeText(client.nowEpochS));
        }
        return parts.join(" · ");
    }

    Accessible.role: Accessible.StaticText
    Accessible.name: lineText

    implicitWidth: innerRow.implicitWidth + 2 * root.style.gap2
    implicitHeight: innerRow.implicitHeight + 2 * root.style.gap * 0.75

    BorderImage {
        id: capsule

        anchors.fill: parent
        source: Qt.resolvedUrl("assets/ps3_infobar.png")
        border { left: 12; top: 10; right: 12; bottom: 10 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: true
    }

    RowLayout {
        id: innerRow

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: root.style.gap2
        anchors.rightMargin: root.style.gap2
        spacing: root.style.gap

        // The status dot with its halo: the halo is drawn first, under the dot.
        Item {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: root.style.snap(root.style.unit * 0.9)
            implicitHeight: implicitWidth

            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: root.style.alpha(root.dotColor, 0.25)
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
            color: root.style.ink
            elide: Text.ElideRight
        }
    }
}
