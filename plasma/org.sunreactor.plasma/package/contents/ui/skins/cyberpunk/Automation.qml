/*
 * Automation, cyberpunk: the mode in its own titled panel, for a theme that does
 * not list the displays. With the displays the same controls close that panel
 * (ModeRows, drawn by Displays.qml) and this module stays away.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    /*! Drawn inside the Displays panel instead, when the theme lists both. */
    readonly property bool hosted: root.theme.modules.indexOf("displays") >= 0
    readonly property bool available: client.isConnected && !hosted

    spacing: 0

    CyberFrame {
        Layout.fillWidth: true
        visible: root.available
        style: root.style
        title: i18ndc(root.domain, "Panel title: automation mode", "Automation")
        contentSpacing: 0

        ModeRows {
            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
