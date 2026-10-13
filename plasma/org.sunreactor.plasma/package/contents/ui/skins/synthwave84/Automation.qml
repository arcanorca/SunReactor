/*
 * Automation, synthwave84: the mode in a neon card of its own, for a theme that
 * does not list the displays. With the displays, the same controls close the
 * Displays column (ModeRows, drawn by Displays.qml) and this module stays away.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    /*! Drawn inside the Displays column instead, when the theme lists both. */
    readonly property bool hosted: root.theme.modules.indexOf("displays") >= 0
    readonly property bool available: client.isConnected && !hosted

    spacing: 0

    NeonCard {
        Layout.fillWidth: true
        visible: root.available
        style: root.style
        neon: root.style.pink

        ModeRows {
            Layout.fillWidth: true
            theme: root.theme
            client: root.client
            settings: root.settings
        }
    }
}
