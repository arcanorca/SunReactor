/*
 * Footer part "pause": stop reacting for a while (durations menu), or resume.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3

import ".."

FooterButton {
    id: root

    required property PopupFooter footer

    readonly property string domain: footer.domain

    function minutesUntil(epochSeconds) {
        return Math.max(5, Math.round((epochSeconds - footer.client.nowEpochS) / 60));
    }

    theme: footer.theme
    icon.name: footer.client.isSuspended ? "media-playback-start" : "media-playback-pause"
    text: footer.client.isSuspended
        ? i18ndc(domain, "@action:button", "Resume")
        : i18ndc(domain, "@action:button", "Pause")
    highlighted: footer.client.isSuspended
    down: pauseMenu.visible || pressed
    toolTip: pauseMenu.visible ? "" : (footer.client.isSuspended
        ? i18nd(domain, "Go back to adjusting brightness automatically")
        : i18nd(domain, "Stop adjusting brightness for a while"))
    Accessible.role: footer.client.isSuspended ? Accessible.Button : Accessible.ButtonMenu

    onClicked: {
        if (footer.client.isSuspended) {
            footer.client.resume();
        } else {
            pauseMenu.visible ? pauseMenu.close() : pauseMenu.open();
        }
    }

    PlasmaComponents3.Menu {
        id: pauseMenu

        // Opens upwards: the footer sits at the bottom of the popup.
        y: -height

        PlasmaComponents3.MenuItem {
            text: i18ndc(root.domain, "@action:inmenu Pause for a duration", "For 30 Minutes")
            onTriggered: root.footer.client.suspend(30)
        }
        PlasmaComponents3.MenuItem {
            text: i18ndc(root.domain, "@action:inmenu Pause for a duration", "For 1 Hour")
            onTriggered: root.footer.client.suspend(60)
        }
        PlasmaComponents3.MenuItem {
            text: i18ndc(root.domain, "@action:inmenu Pause for a duration", "For 2 Hours")
            onTriggered: root.footer.client.suspend(120)
        }
        PlasmaComponents3.MenuSeparator {}
        PlasmaComponents3.MenuItem {
            text: i18ndc(root.domain, "@action:inmenu", "Until Sunset")
            visible: root.footer.client.sunsetEpochS > root.footer.client.nowEpochS
            height: visible ? implicitHeight : 0
            onTriggered: root.footer.client.suspend(root.minutesUntil(root.footer.client.sunsetEpochS))
        }
        PlasmaComponents3.MenuItem {
            text: i18ndc(root.domain, "@action:inmenu", "Until Sunrise")
            visible: root.footer.client.sunriseEpochS > root.footer.client.nowEpochS
            height: visible ? implicitHeight : 0
            onTriggered: root.footer.client.suspend(root.minutesUntil(root.footer.client.sunriseEpochS))
        }
        PlasmaComponents3.MenuItem {
            text: i18ndc(root.domain, "@action:inmenu", "Until I Resume")
            onTriggered: root.footer.client.suspend(0)
        }
    }
}
