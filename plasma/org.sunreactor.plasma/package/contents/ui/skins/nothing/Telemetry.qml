/*
 * Telemetry, Nothing: the status line above the tabs.
 *
 *   ● LIVE   MI 39   LEN 48                              15° 14:30
 *
 * The red lamp is the daemon's state (live, manual, paused: the accent; not
 * answering: grey). Each display reads its applied value in the tiny 3x5 dot
 * face; temperature and the daemon's clock sit at the right in the same face.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected
    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }

    readonly property bool offline: client.mode === SunReactorClient.Offline
    readonly property string stateText: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18ndc(root.domain, "Daemon state in the status line", "OFFLINE");
        case SunReactorClient.Paused:
            return i18ndc(root.domain, "Daemon state in the status line", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(root.domain, "Daemon state in the status line", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(root.domain, "Daemon state in the status line", "IDLE");
        default:
            return i18ndc(root.domain, "Daemon state in the status line", "LIVE");
        }
    }

    /*! Temperature and the daemon's clock, one string: "15° 2:30 PM" (or "15° 14:30"). */
    readonly property string clockText: (client.hasTemperature
        ? i18ndc(domain, "Temperature in degrees Celsius, compact", "%1°", Math.round(client.temperatureC)) + " "
        : "") + Format.timeText(client.nowEpochS)

    /*! Air above the status line, so the tiny digits clear the popup's edge. */
    readonly property real inset: style.gap

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight + inset

    Accessible.role: Accessible.StaticText
    Accessible.name: {
        const parts = [stateText];
        for (const monitor of client.monitors) {
            parts.push(style.shortName(monitor.logicalId) + " "
                       + (monitor.appliedPercent >= 0 ? monitor.appliedPercent + "%" : "—"));
        }
        if (client.hasTemperature) {
            parts.push(Math.round(client.temperatureC) + "°C");
        }
        parts.push(Format.timeText(client.nowEpochS));
        return parts.join(", ");
    }

    RowLayout {
        id: row

        x: root.style.gap
        y: root.inset
        width: parent.width - 2 * root.style.gap
        // Eight items on one line: tighter than the card gap.
        spacing: root.style.gap * 0.5

        Lamp {
            Layout.alignment: Qt.AlignVCenter
            tone: root.offline ? root.style.muted : root.style.accent
        }

        Text {
            Layout.alignment: Qt.AlignVCenter
            Layout.minimumWidth: implicitWidth
            text: root.stateText
            textFormat: Text.PlainText
            font: root.style.label
            color: root.offline ? root.style.muted : root.style.ink
        }

        Repeater {
            model: root.client.monitors

            delegate: RowLayout {
                id: channel

                required property var modelData

                Layout.alignment: Qt.AlignVCenter
                spacing: root.style.gap * 0.25

                Text {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.leftMargin: root.style.gap * 0.5
                    Layout.minimumWidth: implicitWidth
                    text: root.style.shortName(channel.modelData.logicalId)
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                }

                DotText {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.minimumWidth: implicitWidth
                    text: channel.modelData.appliedPercent >= 0
                        ? String(channel.modelData.appliedPercent)
                        : i18ndc(root.domain, "No value is known yet", "—")
                    face: "tiny"
                    size: 1
                    pitch: root.style.pitchFine
                    litColor: root.style.ink
                    accentColor: root.style.accent
                    dimColor: root.style.textGhost
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.minimumWidth: root.style.gap
        }

        // The clock, one run of tiny face at the right edge: "15° 2:30 PM" in the
        // locale's hour convention, the same as the other skins. Its unlit cells
        // are nearly black, so only the glyphs show.
        DotText {
            Layout.alignment: Qt.AlignVCenter
            Layout.minimumWidth: implicitWidth
            text: root.clockText
            face: "tiny"
            size: 1
            pitch: root.style.pitchFine
            litColor: root.style.ink
            accentColor: root.style.accent
            dimColor: root.style.theme.alpha(root.style.ink, 0.03)
        }
    }
}
