/*
 * Automation module: what the daemon is doing, why, and what happens next.
 * When the user has taken over it offers the way back. While the daemon is
 * simply following the sun it only appears if the day curve says what the
 * next step is ("08:54 · Rise 50% → 34%"); otherwise there is nothing to say.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../Format.js" as Format

PlasmaComponents3.ItemDelegate {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain

    /*! The next milestone of the first display's curve still ahead, or null. */
    readonly property var nextMilestone: {
        const curve = client.dayCurve;
        if (!curve || !curve.monitors || curve.monitors.length === 0) {
            return null;
        }
        for (const milestone of curve.monitors[0].milestones) {
            if (milestone.t > client.nowEpochS) {
                return milestone;
            }
        }
        return null;
    }
    readonly property string nextText: nextMilestone
        ? (nextMilestone.pct >= 0
            ? i18ndc(domain, "Time, then the daemon's milestone name, then the brightness it targets",
                     "%1 · %2 → %3%", Format.timeText(nextMilestone.t), nextMilestone.label, nextMilestone.pct)
            : i18ndc(domain, "Time, then the daemon's milestone name", "%1 · %2",
                     Format.timeText(nextMilestone.t), nextMilestone.label))
        : ""

    readonly property bool available: client.isConnected
        && (client.mode !== SunReactorClient.Automatic || nextText.length > 0)

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
            return client.hasWeatherReading
                ? i18nd(root.domain, "Following the sun and the weather")
                : i18nd(root.domain, "Following the sun");
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
            return root.nextText.length > 0
                ? i18ndc(root.domain, "Placeholder is the next step of the brightness curve", "Next: %1",
                         root.nextText)
                : "";
        }
    }

    Layout.fillWidth: true

    leftPadding: Kirigami.Units.smallSpacing * 2
    rightPadding: Kirigami.Units.smallSpacing * 2
    topPadding: Kirigami.Units.smallSpacing
    bottomPadding: Kirigami.Units.smallSpacing

    background: Frame {
        theme: root.theme
        pixel: false
        style: root.theme.available ? root.theme.border : "line"
        color: root.client.mode === SunReactorClient.Automatic ? root.theme.line : root.theme.warning
        fillColor: root.theme.alpha(root.client.mode === SunReactorClient.Automatic
                                        ? root.theme.fg : root.theme.warning, 0.08)
    }

    hoverEnabled: false
    Accessible.ignored: true

    contentItem: RowLayout {
        spacing: Kirigami.Units.gridUnit

        Kirigami.Icon {
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            source: root.modeIcon
            color: root.theme.fg
            isMask: root.theme.available
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.modeTitle
                textFormat: Text.PlainText
                font.family: root.theme.bodyFont
                color: root.theme.fg
                elide: Text.ElideRight
            }

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                visible: text.length > 0
                text: root.modeDetail
                textFormat: Text.PlainText
                font.family: root.theme.bodyFont
                color: root.theme.textMuted
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
