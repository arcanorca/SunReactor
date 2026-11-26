/*
 * Xbox 360 Telemetry (Gamercard-style status header): Guide Sphere with Ring
 * of Light LEDs reflecting daemon mode + live brightness & weather summary.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../../components"

XboxCard {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property XboxStyle xboxStyle: XboxStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected || client.statusStale

    readonly property string rolAsset: {
        switch (client.mode) {
        case SunReactorClient.Offline: return root.xboxStyle.asset("xbox_rol_offline.png");
        case SunReactorClient.Paused: return root.xboxStyle.asset("xbox_rol_paused.png");
        case SunReactorClient.Manual: return root.xboxStyle.asset("xbox_rol_manual.png");
        default: return root.xboxStyle.asset("xbox_rol_live.png");
        }
    }

    readonly property string lineText: {
        const parts = [stateWords.label(client.mode)];
        for (const m of client.monitors) {
            const head = String(m.logicalId).split(/[-_ .]/)[0] || String(m.logicalId);
            parts.push(m.appliedPercent >= 0
                ? i18ndc(domain, "Short display name, then its applied brightness", "%1 %2%",
                         head.slice(0, 4).toUpperCase(), m.appliedPercent)
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

    style: root.xboxStyle
    pad: root.xboxStyle.gap
    implicitHeight: inner.implicitHeight + 2 * pad

    StateText {
        id: stateWords
    }

    Accessible.role: Accessible.StaticText
    Accessible.name: lineText

    RowLayout {
        id: inner

        anchors.fill: parent
        spacing: root.xboxStyle.gap

        Item {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: root.xboxStyle.snap(root.xboxStyle.unit * 1.4)
            implicitHeight: implicitWidth

            Image {
                anchors.fill: parent
                source: Qt.resolvedUrl(root.xboxStyle.asset("xbox_guide_orb.png"))
                smooth: true
                mipmap: true
            }
            Image {
                anchors.fill: parent
                source: Qt.resolvedUrl(root.rolAsset)
                smooth: true
                mipmap: true
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            text: root.lineText
            textFormat: Text.PlainText
            font: root.xboxStyle.captionBoldFont
            color: root.xboxStyle.ink
            elide: Text.ElideRight
        }
    }
}
