/*
 * Screenshot harness containment (tools/shot.sh installs it into a private
 * XDG_DATA_HOME, never into the user's). It holds a single applet and shows
 * its popup at the size the popup asks for: the full representation's layout
 * hints, top left over Plasma's popup background. In the planar form factor
 * the full representation then renders as it would inside the popup, and the
 * window can be larger than the popup. The popup's place and size are logged
 * as SUNREACTOR_HARNESS_POPUP (window pixels, logical px) for tools/shot.sh to
 * crop to.
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.ksvg as KSvg

ContainmentItem {
    id: root

    /*! The applet's root item, once adopted: the popup's frame. */
    property Item popup: null
    /*! The popup's full representation. Typed, so its attached Layout hints resolve. */
    property Item full: null

    Binding {
        target: root
        property: "full"
        value: root.popup ? root.popup.fullRepresentationItem : null
    }

    function adopt(applet) {
        const item = root.itemFor(applet);
        if (!item) {
            return;
        }
        item.parent = host;
        item.visible = true;
        item.anchors.left = host.left;
        item.anchors.top = host.top;
        // Until the full representation exists, the window's size.
        item.width = Qt.binding(() => root.full ? root.full.Layout.preferredWidth : host.width);
        item.height = Qt.binding(() => root.full ? root.full.Layout.preferredHeight : host.height);
        root.popup = item;
    }

    /*! The popup's extent in window pixels: x, y, then its size (logical px). */
    function report(): void {
        if (!root.popup) {
            return;
        }
        const at = root.popup.mapToItem(null, 0, 0);
        console.info("SUNREACTOR_HARNESS_POPUP " + JSON.stringify({
            x: at.x,
            y: at.y,
            width: root.popup.width,
            height: root.popup.height,
        }));
    }

    Connections {
        target: root.popup
        ignoreUnknownSignals: true

        function onWidthChanged(): void {
            root.report();
        }
        function onHeightChanged(): void {
            root.report();
        }
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
