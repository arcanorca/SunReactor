/*
 * The status lamp: an 8 px dot standing in for the status line when the popup
 * hides it. FullRepresentation puts it at the right end of the tab row, left of
 * the theme button, so every tab family shows it. The colour is the daemon
 * state in the theme's palette: success while automatic (or dimmed while idle),
 * warning when set by hand, text-muted while paused, error when the daemon does
 * not answer. The tooltip and the accessible name say the state in words.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    readonly property color lampColor: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return theme.error;
        case SunReactorClient.Paused:
            return theme.textMuted;
        case SunReactorClient.Manual:
            return theme.warning;
        default:
            return theme.success;
        }
    }

    readonly property string stateText: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18nd(domain, "Not connected");
        case SunReactorClient.Paused:
            return i18nd(domain, "Paused");
        case SunReactorClient.Manual:
            return i18nd(domain, "Set by hand");
        case SunReactorClient.IdleDimmed:
            return i18nd(domain, "Dimmed while idle");
        default:
            return i18nd(domain, "Following the sun");
        }
    }

    implicitWidth: 8
    implicitHeight: 8

    Accessible.role: Accessible.Indicator
    Accessible.name: root.stateText

    Rectangle {
        anchors.fill: parent
        radius: root.theme.isVector ? width / 2 : 0
        antialiasing: radius > 0
        color: root.lampColor
    }

    HoverHandler {
        id: hover
    }

    QQC2.ToolTip.visible: hover.hovered
    QQC2.ToolTip.text: root.stateText
    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
}
