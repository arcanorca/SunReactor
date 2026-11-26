/*
 * Nintendo DS status bar: a thin white pill above the tabs, the console's own top
 * bar - wifi bars (lit while the daemon answers), the daemon's state word, the
 * battery lamp and the clock. Hidden by default (the widget's hiddenModules).
 * Module contract: modules/Modules.js.
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
    readonly property bool available: client.isConnected || client.statusStale

    implicitHeight: bar.height

    NdsPanel {
        id: bar

        style: root.style
        kind: "panel"
        width: root.width
        height: Math.round(root.style.gu * 1.9)

        NdsStatusBar {
            anchors.fill: parent
            anchors.leftMargin: Math.round(root.style.gu * 0.7)
            anchors.rightMargin: Math.round(root.style.gu * 0.7)
            theme: root.theme
            style: root.style
            client: root.client
            showState: true
        }
    }
}
