/*
 * How long a pause lasts: 30 minutes, an hour, two hours, or until resumed.
 * Opened from the arrow on the Paused choice: the skin calls openBelow() with
 * the control the arrow belongs to. The Paused choice itself pauses for the
 * configured default.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3

import "../modules"
import "PopupOverlay.js" as PopupOverlay

PlasmaComponents3.Menu {
    id: root

    required property ModeControl control

    property bool _overlayRegistered: false
    property var _overlayHost: null
    property Item _overlayAnchor: null

    function syncOverlay(active: bool): void {
        const target = root._overlayAnchor ? root._overlayAnchor : (root.parent ? root.parent : root);
        const next = PopupOverlay.sync(target, _overlayHost, _overlayRegistered, active);
        _overlayHost = next.host;
        _overlayRegistered = next.registered;
    }

    onAboutToShow: syncOverlay(true)
    onAboutToHide: syncOverlay(false)
    Component.onDestruction: syncOverlay(false)

    /*! Opens the menu under `anchor`, left-aligned with it, in the coordinates of `host`
        (the item the menu is declared in). */
    function openBelow(anchor: Item, host: Item): void {
        root._overlayAnchor = host ? host : anchor;
        x = Math.max(0, anchor.mapToItem(host, 0, 0).x);
        y = anchor.mapToItem(host, 0, anchor.height).y;
        open();
    }

    PlasmaComponents3.MenuItem {
        text: i18ndc(root.control.domain, "@action:inmenu Pause for a duration", "For 30 Minutes")
        onTriggered: root.control.pause(30)
    }
    PlasmaComponents3.MenuItem {
        text: i18ndc(root.control.domain, "@action:inmenu Pause for a duration", "For 1 Hour")
        onTriggered: root.control.pause(60)
    }
    PlasmaComponents3.MenuItem {
        text: i18ndc(root.control.domain, "@action:inmenu Pause for a duration", "For 2 Hours")
        onTriggered: root.control.pause(120)
    }
    PlasmaComponents3.MenuSeparator {}
    PlasmaComponents3.MenuItem {
        text: i18ndc(root.control.domain, "@action:inmenu", "Until I Resume")
        onTriggered: root.control.pause(0)
    }
}
