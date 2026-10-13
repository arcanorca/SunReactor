/*
 * Automation, instrument family: mode, reason and what happens next, as a
 * legend block.
 *
 *   AUTOMATION ──────────────────────────────────────────────────
 *   [AUTO]  Following the sun and the weather
 *   NEXT    16:20   Set 50%  → 23 %                     IN 1 H 50
 *
 * When the user has taken over, the tag turns to the warning tone and the
 * block offers the way back. Hidden while the daemon simply follows the sun
 * and the curve has no next step.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }

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
    readonly property bool automatic: client.mode === SunReactorClient.Automatic
    readonly property bool available: client.isConnected && (!automatic || nextMilestone !== null)

    readonly property string modeTag: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return i18ndc(domain, "Automation mode tag", "Paused");
        case SunReactorClient.Manual:
            return i18ndc(domain, "Automation mode tag", "Manual");
        case SunReactorClient.IdleDimmed:
            return i18ndc(domain, "Automation mode tag", "Idle");
        default:
            return i18ndc(domain, "Automation mode tag", "Auto");
        }
    }
    readonly property string modeTitle: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return i18nd(domain, "Paused");
        case SunReactorClient.Manual:
            return i18nd(domain, "Set by hand");
        case SunReactorClient.IdleDimmed:
            return i18nd(domain, "Dimmed while idle");
        default:
            return client.hasWeatherReading
                ? i18nd(domain, "Following the sun and the weather")
                : i18nd(domain, "Following the sun");
        }
    }
    /*! Until when the current non-automatic state lasts, as text. */
    readonly property string untilText: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return client.suspendUntilEpochS > 0 ? Format.timeText(client.suspendUntilEpochS)
                                                 : i18ndc(domain, "Paused with no end time", "until resumed");
        case SunReactorClient.Manual:
            return client.overrideUntilEpochS > 0 ? Format.timeText(client.overrideUntilEpochS)
                                                  : i18ndc(domain, "Manual with no end time", "until switched back");
        case SunReactorClient.IdleDimmed:
            return i18ndc(domain, "Idle dimming ends", "until you return");
        default:
            return "";
        }
    }

    function durationText(seconds) {
        const minutes = Math.max(0, Math.round(seconds / 60));
        const h = Math.floor(minutes / 60);
        const m = minutes % 60;
        if (h > 0) {
            return i18ndc(domain, "Time remaining, hours and minutes, compact", "in %1 h %2", h,
                          String(m).padStart(2, "0"));
        }
        return i18ndc(domain, "Time remaining in minutes, compact", "in %1 min", m);
    }

    /*! Tag column width, shared by the "Next"/"Until" legend so values align. */
    readonly property real labelColumn: style.snap(tagMetrics.width + style.snap(style.sizeLegend * 0.55) * 2)

    spacing: style.rowGap

    Accessible.role: Accessible.StaticText
    Accessible.name: modeTitle

    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.domain, "Instrument legend over the automation state", "Automation")
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        Tag {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: root.labelColumn
            style: root.style
            text: root.modeTag
            tone: root.automatic ? root.style.ok : root.style.warning
            filled: !root.automatic
        }

        Text {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            text: root.modeTitle
            textFormat: Text.PlainText
            font: root.style.bodyFont
            color: root.style.ink
            elide: Text.ElideRight
        }

        PushButton {
            Layout.alignment: Qt.AlignVCenter
            visible: root.client.isOverrideActive
            style: root.style
            tone: root.style.ink
            text: i18ndc(root.domain, "@action:button Return to automatic brightness", "Automatic")
            onClicked: root.client.clearAllOverrides()
        }

        TextMetrics {
            id: tagMetrics
            font: root.style.legendFont
            text: "MANUAL"
        }
    }

    // Until when (paused / manual / idle) or what the curve does next.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2
        visible: root.untilText.length > 0 || root.nextMilestone !== null

        Text {
            Layout.alignment: Qt.AlignBaseline
            Layout.preferredWidth: root.labelColumn
            text: root.automatic ? i18ndc(root.domain, "Legend: the next step of the curve", "Next")
                                 : i18ndc(root.domain, "Legend: how long the current state lasts", "Until")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }

        Text {
            Layout.alignment: Qt.AlignBaseline
            text: root.automatic ? Format.timeText(root.nextMilestone ? root.nextMilestone.t : 0) : root.untilText
            textFormat: Text.PlainText
            font: root.style.dataFont
            color: root.style.ink
        }

        Text {
            Layout.alignment: Qt.AlignBaseline
            Layout.fillWidth: true
            visible: root.automatic && root.nextMilestone !== null
            text: root.nextMilestone
                ? (root.nextMilestone.pct >= 0
                    ? i18ndc(root.domain, "Milestone name, then the brightness it targets", "%1 → %2%",
                             root.nextMilestone.label, root.nextMilestone.pct)
                    : root.nextMilestone.label)
                : ""
            textFormat: Text.PlainText
            font: root.style.bodyFont
            color: root.style.inkMuted
            elide: Text.ElideRight
        }

        Item {
            Layout.fillWidth: true
            visible: !root.automatic
        }

        Text {
            Layout.alignment: Qt.AlignBaseline
            visible: root.automatic && root.nextMilestone !== null
            text: root.nextMilestone ? root.durationText(root.nextMilestone.t - root.client.nowEpochS) : ""
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkFaint
        }
    }
}
