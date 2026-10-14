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
    readonly property bool available: client.isConnected

    implicitHeight: bar.height

    Rectangle {
        id: bar

        width: root.width
        height: Math.round(root.style.gu * 1.9)
        radius: height / 2
        color: root.style.white
        border.width: 1
        border.color: root.style.line

        NdsStatusBar {
            anchors.fill: parent
            anchors.leftMargin: Math.round(root.style.gu * 0.9)
            anchors.rightMargin: Math.round(root.style.gu * 0.9)
            theme: root.theme
            style: root.style
            client: root.client
            showState: true
        }
    }
}
