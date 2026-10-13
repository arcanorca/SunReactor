/*
 * How long a pause lasts: 30 minutes, an hour, two hours, or until resumed.
 * Opened from the arrow on the Paused choice; the skin places it and calls
 * open(). The Paused choice itself pauses for the configured default.
 */

pragma ComponentBehavior: Bound

import org.kde.plasma.components as PlasmaComponents3

import "../modules"

PlasmaComponents3.Menu {
    id: root

    required property ModeControl control

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
