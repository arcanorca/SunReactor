/*
 * Attaches the CRT overlay (HgCrtLayer) to the popup window. The overlay has to
 * cover the whole popup, tab row and footer included, and a module can only draw
 * inside its own slot, so the overlay is parented to the window's content item.
 * Every hackergreen module holds one HgCrt: the first to appear creates the
 * overlay, each one counts itself in `users` while it lives, and the overlay
 * shows only while some hackergreen module is loaded. A theme switch that unloads
 * the modules therefore hides it, and one overlay is never stacked twice.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"

Item {
    id: root

    required property Theme theme

    /*! The shared HgCrtLayer this module is counted in, or null. */
    property var overlay: null

    visible: false
    width: 0
    height: 0

    Component {
        id: layerFactory

        HgCrtLayer {}
    }

    Component.onCompleted: {
        const top = root.Window.window ? root.Window.window.contentItem : null;
        if (!top) {
            return;
        }
        let found = null;
        for (const child of top.children) {
            if (child.objectName === "hgCrt") {
                found = child;
                break;
            }
        }
        if (found === null) {
            found = layerFactory.createObject(top, { theme: root.theme });
        }
        found.theme = root.theme;
        found.users += 1;
        root.overlay = found;
    }

    Component.onDestruction: {
        if (root.overlay !== null) {
            root.overlay.users -= 1;
        }
    }
}
