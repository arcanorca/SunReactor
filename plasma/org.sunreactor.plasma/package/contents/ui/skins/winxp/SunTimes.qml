/*
 * Sun times, Windows XP: its own window, showing the status bar. When the weather
 * module is listed, the Weather window hosts the status bar and this window stays
 * hidden.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

WinWindow {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    style: ui
    width: parent ? parent.width : 0

    /*! The Weather window draws the status bar when it is listed. */
    readonly property bool hosted: theme.modules.indexOf("weather") >= 0
    readonly property bool available: !hosted && bar.available
    title: i18ndc(settings.domain, "Window title of the sun times", "Sun")

    SunStatus {
        id: bar

        Layout.fillWidth: true
        theme: root.theme
        client: root.client
        settings: root.settings
    }
}
