/*
 * Screenshot harness containment (tools/shot.sh installs it into a private
 * XDG_DATA_HOME, never into the user's). It holds a single applet and lets it
 * fill the window over Plasma's popup background, so in the planar form factor
 * the full representation renders exactly as it would inside the popup.
 */

import QtQuick
import org.kde.plasma.plasmoid
import org.kde.ksvg as KSvg

ContainmentItem {
    id: root

    function adopt(applet) {
        const item = root.itemFor(applet);
        if (!item) {
            return;
        }
        item.parent = host;
        item.anchors.fill = host;
        item.visible = true;
    }

    Rectangle {
        anchors.fill: parent
        color: "#1b1e20"
    }

    KSvg.FrameSvgItem {
        id: frame

        anchors.fill: parent
        imagePath: "dialogs/background"
    }

    Item {
        id: host

        anchors.fill: parent
        anchors.leftMargin: frame.margins.left
        anchors.topMargin: frame.margins.top
        anchors.rightMargin: frame.margins.right
        anchors.bottomMargin: frame.margins.bottom
    }

    Connections {
        target: root.Plasmoid
        function onAppletAdded(applet, geometryHint) {
            root.adopt(applet);
        }
    }

    Component.onCompleted: {
        for (const applet of Plasmoid.applets) {
            adopt(applet);
        }
    }
}
