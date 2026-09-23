/*
 * The two things worth a button: stop reacting for a while, and re-evaluate
 * now. Everything else lives in the widget's context menu.
 */

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

PlasmaExtras.PlasmoidHeading {
    id: root

    required property SunReactorClient client
    /*! Duration of the plain "Pause" click, from the widget's settings. */
    property int defaultPauseMinutes: 60

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    function minutesUntil(epochSeconds) {
        return Math.max(5, Math.round((epochSeconds - client.nowEpochS) / 60));
    }

    position: QQC2.ToolBar.Footer
    // PlasmoidHeading pulls its insets out to the popup edge; the buttons
    // still need room to breathe.
    leftPadding: Kirigami.Units.smallSpacing
    rightPadding: Kirigami.Units.smallSpacing

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing

        PlasmaComponents3.Button {
            id: pauseButton

            icon.name: root.client.isSuspended ? "media-playback-start" : "media-playback-pause"
            text: root.client.isSuspended
                ? i18ndc(root.domain, "@action:button", "Resume")
                : i18ndc(root.domain, "@action:button", "Pause")
            highlighted: root.client.isSuspended
            down: pauseMenu.visible

            onClicked: {
                if (root.client.isSuspended) {
                    root.client.resume();
                } else {
                    pauseMenu.visible ? pauseMenu.close() : pauseMenu.open();
                }
            }

            QQC2.ToolTip.visible: hovered && !pauseMenu.visible
            QQC2.ToolTip.text: root.client.isSuspended
                ? i18nd(root.domain, "Go back to adjusting brightness automatically")
                : i18nd(root.domain, "Stop adjusting brightness for a while")

            PlasmaComponents3.Menu {
                id: pauseMenu

                // Opens upwards: the footer sits at the bottom of the popup.
                y: -height

                PlasmaComponents3.MenuItem {
                    text: i18ndc(root.domain, "@action:inmenu Pause for a duration", "For 30 Minutes")
                    onTriggered: root.client.suspend(30)
                }
                PlasmaComponents3.MenuItem {
                    text: i18ndc(root.domain, "@action:inmenu Pause for a duration", "For 1 Hour")
                    onTriggered: root.client.suspend(60)
                }
                PlasmaComponents3.MenuItem {
                    text: i18ndc(root.domain, "@action:inmenu Pause for a duration", "For 2 Hours")
                    onTriggered: root.client.suspend(120)
                }
                PlasmaComponents3.MenuSeparator {}
                PlasmaComponents3.MenuItem {
                    text: i18ndc(root.domain, "@action:inmenu", "Until Sunset")
                    visible: root.client.sunsetEpochS > root.client.nowEpochS
                    height: visible ? implicitHeight : 0
                    onTriggered: root.client.suspend(root.minutesUntil(root.client.sunsetEpochS))
                }
                PlasmaComponents3.MenuItem {
                    text: i18ndc(root.domain, "@action:inmenu", "Until Sunrise")
                    visible: root.client.sunriseEpochS > root.client.nowEpochS
                    height: visible ? implicitHeight : 0
                    onTriggered: root.client.suspend(root.minutesUntil(root.client.sunriseEpochS))
                }
                PlasmaComponents3.MenuItem {
                    text: i18ndc(root.domain, "@action:inmenu", "Until I Resume")
                    onTriggered: root.client.suspend(0)
                }
            }
        }

        Item {
            Layout.fillWidth: true
        }

        PlasmaComponents3.ToolButton {
            icon.name: "view-refresh"
            text: i18ndc(root.domain, "@action:button", "Update Now")
            display: PlasmaComponents3.AbstractButton.IconOnly
            enabled: root.client.isConnected

            onClicked: root.client.runOnce(true)

            QQC2.ToolTip.visible: hovered
            QQC2.ToolTip.text: i18nd(root.domain, "Re-read the sun position and weather, then apply the result")
        }
    }
}
