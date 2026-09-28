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
import "../wardrobe/amiga"

PlasmaComponents3.ItemDelegate {
    id: root

    required property SunReactorClient client
    property var tokens: null

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

    readonly property color accentColor: (tokens && tokens.accentColor)
        ? tokens.accentColor
        : ((client.themeAccent && client.themeAccent.length > 0) ? client.themeAccent : Kirigami.Theme.highlightColor)

    Layout.fillWidth: true

    leftPadding: Kirigami.Units.smallSpacing * 2
    rightPadding: Kirigami.Units.smallSpacing * 2
    topPadding: Kirigami.Units.smallSpacing
    bottomPadding: Kirigami.Units.smallSpacing

    background: Rectangle {
        radius: root.tokens ? root.tokens.cardRadius : Kirigami.Units.smallSpacing * 1.5
        color: root.tokens ? Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, 0.12)
                           : Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, 0.10)
        border.width: root.tokens ? root.tokens.cardBorderWidth : 1
        border.color: root.tokens ? root.tokens.cardBorderColor
                                  : Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, 0.25)

        // Nothing OS pulsing indicator dot
        Rectangle {
            visible: root.tokens && root.tokens.isNothing
            width: 5
            height: 5
            radius: 2.5
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 4
            color: root.tokens ? root.tokens.liveIndicatorColor : "#E50914"

            SequentialAnimation on opacity {
                running: root.tokens && root.tokens.isNothing
                loops: Animation.Infinite
                NumberAnimation { to: 0.3; duration: 900; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 1.0; duration: 900; easing.type: Easing.InOutQuad }
            }
        }
    }

    hoverEnabled: false
    Accessible.ignored: true

    contentItem: RowLayout {
        spacing: Kirigami.Units.gridUnit

        // Amiga Boing Ball or standard mode icon
        Kirigami.Icon {
            visible: !(root.tokens && root.tokens.boingBallVisible)
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            source: root.modeIcon
        }

        AmigaBoingBall {
            visible: root.tokens && root.tokens.boingBallVisible
            size: Kirigami.Units.iconSizes.medium
            running: true
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.modeTitle
                textFormat: Text.PlainText
                font.family: (root.tokens && root.tokens.fontFamily) || ""
                color: (root.tokens && root.tokens.textColor) ? root.tokens.textColor : Kirigami.Theme.textColor
                elide: Text.ElideRight
            }

            PlasmaExtras.DescriptiveLabel {
                Layout.fillWidth: true
                text: root.modeDetail
                textFormat: Text.PlainText
                font.family: (root.tokens && root.tokens.fontFamily) || ""
                color: (root.tokens && root.tokens.textMutedColor) ? root.tokens.textMutedColor : Kirigami.Theme.disabledTextColor
                elide: Text.ElideRight
            }
        }

        PlasmaComponents3.Button {
            visible: root.client.isOverrideActive
            icon.name: "edit-undo"
            text: i18ndc(root.domain, "@action:button Return to automatic brightness", "Automatic")
            font.family: (root.tokens && root.tokens.fontFamily) || ""
            onClicked: root.client.clearAllOverrides()
        }
    }
}
