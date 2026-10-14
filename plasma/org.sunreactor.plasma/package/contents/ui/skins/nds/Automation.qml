/*
 * Nintendo DS automation: the mode as three touch buttons - AUTO, MANUAL, PAUSE -
 * with the daemon's sentence in a speech bubble under them (ModeRows). On the DS the
 * controls live on the touch screen, under the display panels, so Displays.qml draws
 * them there and this module stays away while the theme lists the displays. Without
 * displays it draws them in a strip of its own. The rules are in
 * modules/ModeControl.qml. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    /*! Drawn inside the touch screen by Displays.qml when the theme lists the displays. */
    readonly property bool hosted: root.theme.modules.indexOf("displays") >= 0
    readonly property bool available: client.isConnected && !hosted

    implicitHeight: tray.implicitHeight

    Loader {
        id: tray

        width: parent.width
        active: root.available
        sourceComponent: ModeRows {
            theme: root.theme
            client: root.client
            settings: root.settings
            style: root.style
        }
    }
}
