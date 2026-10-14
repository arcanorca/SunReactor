/*
 * Harness replacement for plasmoidviewer's shell view (views/Desktop.qml).
 * tools/shot.sh copies the system plasmoidviewershell into its private
 * XDG_DATA_HOME and swaps this file in: no SDK toolbar over the applet and no
 * fillScreen, so the window is exactly the size passed with -s and a capture
 * of it contains only the containment and the applet.
 */

import QtQuick

Rectangle {
    id: root

    color: "#202326"
    width: 640
    height: 480

    property Item containment

    onContainmentChanged: {
        containment.visible = true;
        containment.anchors.fill = root;
    }

    Component.onCompleted: {
        desktop.stayBehind = true;
        desktop.fillScreen = false;
    }
}
