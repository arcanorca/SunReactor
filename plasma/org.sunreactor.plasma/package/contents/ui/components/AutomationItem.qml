/*
 * Says what the daemon is doing and, when the user has taken over, offers the
 * way back. This is the only row that explains the widget's behaviour, so it
 * stays to one line of plain language.
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../Format.js" as Format

PlasmaComponents3.ItemDelegate {
    id: root

    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    // One icon per mode, and none of them repeats an icon used further down
    // the popup: the sky belongs to the weather row, not to this one.
    readonly property string modeIcon: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return "media-playback-pause";
        case SunReactorClient.Manual:
            return "document-edit";
        case SunReactorClient.IdleDimmed:
            return "brightness-low";
        default:
            return "clock";
        }
    }

    readonly property string modeTitle: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return i18nd(root.domain, "Paused");
        case SunReactorClient.Manual:
            return i18nd(root.domain, "Set by hand");
        case SunReactorClient.IdleDimmed:
            return i18nd(root.domain, "Dimmed while idle");
        default:
            return i18nd(root.domain, "Automatic");
        }
    }

    readonly property string modeDetail: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return client.suspendUntilEpochS > 0
                ? i18ndc(root.domain, "Placeholder is a time of day", "Until %1",
                         Format.timeText(client.suspendUntilEpochS))
                : i18nd(root.domain, "Until you resume it");
        case SunReactorClient.Manual:
            return client.overrideUntilEpochS > 0
                ? i18ndc(root.domain, "Placeholder is a time of day", "Until %1",
                         Format.timeText(client.overrideUntilEpochS))
                : i18nd(root.domain, "Until you switch back to automatic");
        case SunReactorClient.IdleDimmed:
            return i18nd(root.domain, "Restored as soon as you come back");
        default:
            return client.hasWeatherReading
                ? i18nd(root.domain, "Following the sun and the weather")
                : i18nd(root.domain, "Following the sun");
        }
    }

    Layout.fillWidth: true

    background.visible: false
    hoverEnabled: false
    Accessible.ignored: true

    contentItem: RowLayout {
        spacing: Kirigami.Units.gridUnit

        Kirigami.Icon {
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            source: root.modeIcon
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.modeTitle
                textFormat: Text.PlainText
                elide: Text.ElideRight
            }

            PlasmaExtras.DescriptiveLabel {
                Layout.fillWidth: true
                text: root.modeDetail
                textFormat: Text.PlainText
                elide: Text.ElideRight
            }
        }

        PlasmaComponents3.Button {
            visible: root.client.isOverrideActive
            icon.name: "edit-undo"
            text: i18ndc(root.domain, "@action:button Return to automatic brightness", "Automatic")
            onClicked: root.client.clearAllOverrides()
        }
    }
}
