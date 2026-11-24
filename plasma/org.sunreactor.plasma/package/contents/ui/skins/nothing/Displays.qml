/*
 * Displays, Nothing: a card with one block per display. The name in small caps
 * with its level in dot numerals, a row of round dots as the slider (the red head
 * is the level), and the target and range in tiny grey mono. A hairline separates
 * the blocks. An override replaces the target line with its badge and AUTO.
 *
 *   DISPLAYS
 *   ALL DISPLAYS                                        44%
 *   ●●●●●●●●●●●●●●●●●●○○○○○○○○○○○○○○○○○○○○○○○○○○○○○○○○
 *   ─────────────────────────────────────────────────────
 *   MI-MONITOR                                          39%
 *   ●●●●●●●●●●●●●●●●●●●○○○○○○○○○○○○○○○○○○○○○○○○○○○○○○○○
 *   TARGET 39 · RANGE 1–50
 *
 * Dragging, clicking and the arrow, Page, Home and End keys write a manual
 * override exactly like the instrument skin does (same client calls, same
 * throttle, same accessible names). AUTO returns one display to the sun; the
 * ALL row sets every display, and its way back is the Automation module's.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../../modules/DayCurve.js" as DayCurve
import "../../modules/Monitors.js" as Monitors

Rectangle {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0
    property var expandedLimits: ({})

    implicitHeight: content.implicitHeight + 2 * style.pad
    radius: style.radius
    antialiasing: true
    color: style.surface
    border.width: style.hairline
    border.color: style.outline

    Accessible.role: Accessible.Grouping
    Accessible.name: i18nd(root.domain, "Displays")

    /*! One block: name and level, the slider, then the target, range and override. */
    component Channel: ColumnLayout {
        id: channel

        required property string name
        required property int percent
        required property bool controllable
        required property string detail
        property bool hasOverride: false
        property string overrideBadge: ""
        property string accessibleValue: ""
        property int minPct: -1
        property int maxPct: -1
        property bool hasLimits: false
        property bool limitsOpen: false
        /*! A hairline above the block, between two displays. */
        property bool ruled: false

        signal requested(int percent)
        signal limitsRequested(int minPct, int maxPct)
        signal limitsToggled()
        signal clearRequested()

        Layout.fillWidth: true
        spacing: root.style.gap * 0.5

        Rectangle {
            visible: channel.ruled
            Layout.fillWidth: true
            Layout.topMargin: root.style.gap * 0.5
            implicitHeight: root.style.hairline
            color: root.style.outline
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap

            Text {
                Layout.fillWidth: true
                text: channel.name
                textFormat: Text.PlainText
                font: root.style.label
                color: channel.controllable ? root.style.ink : root.style.muted
                elide: Text.ElideMiddle
            }

            DotText {
                Layout.alignment: Qt.AlignVCenter
                Layout.minimumWidth: implicitWidth
                text: channel.percent >= 0 ? channel.percent + "%" : "--%"
                face: "small"
                size: 1
                letterSpacing: 1
                pitch: root.style.pitchDigit
                dotScale: root.style.digitScale
                litColor: channel.percent >= 0 ? root.style.ink : root.style.muted
                accentColor: root.style.accent
                dimColor: root.style.textGhost
            }
        }

        DotSlider {
            Layout.fillWidth: true
            style: root.style
            value: channel.percent
            enabled: channel.controllable
            accessibleName: channel.name
            accessibleDescription: channel.accessibleValue
            onRequested: percent => channel.requested(percent)
        }

        // Only when there is something to say: the master row has no detail line. A
        // manual override replaces the target line with its badge and the way back.
        RowLayout {
            Layout.fillWidth: true
            visible: channel.detail.length > 0 || channel.hasOverride
            spacing: root.style.gap * 0.5

            Text {
                Layout.fillWidth: true
                text: channel.hasOverride ? channel.overrideBadge : channel.detail
                textFormat: Text.PlainText
                font: root.style.label
                color: channel.hasOverride ? root.style.ink : root.style.muted
                elide: Text.ElideRight
            }

            Pill {
                Layout.alignment: Qt.AlignVCenter
                visible: channel.hasLimits && channel.minPct >= 0 && channel.maxPct >= 0
                style: root.style
                active: channel.limitsOpen
                text: i18ndc(root.domain, "Range limits", "RANGE %1–%2", channel.minPct, channel.maxPct)
                accessibleDescription: i18nd(root.domain, "Open brightness range limits")
                onClicked: channel.limitsToggled()
            }

            Pill {
                Layout.alignment: Qt.AlignVCenter
                visible: channel.hasOverride
                style: root.style
                text: i18ndc(root.domain, "Return to automatic solar tracking", "AUTO")
                accessibleDescription: i18nd(root.domain, "Return this display to automatic brightness")
                onClicked: channel.clearRequested()
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: channel.hasLimits && channel.limitsOpen && channel.minPct >= 0 && channel.maxPct >= 0
            spacing: root.style.gap * 0.5

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                Text {
                    Layout.preferredWidth: root.style.unit * 2.2
                    text: i18ndc(root.domain, "Night minimum boundary, compact", "MIN")
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                }
                DotSlider {
                    id: minDotSlider

                    Layout.fillWidth: true
                    style: root.style
                    from: 0
                    to: Math.max(0, maxDotSlider.shown)
                    value: channel.minPct
                    accessibleName: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                    onRequested: p => channel.limitsRequested(Math.min(p, channel.maxPct), channel.maxPct)
                }
                Text {
                    Layout.preferredWidth: root.style.unit * 2.0
                    horizontalAlignment: Text.AlignRight
                    text: minDotSlider.shown + "%"
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.ink
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                Text {
                    Layout.preferredWidth: root.style.unit * 2.2
                    text: i18ndc(root.domain, "Day maximum boundary, compact", "MAX")
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                }
                DotSlider {
                    id: maxDotSlider

                    Layout.fillWidth: true
                    style: root.style
                    from: Math.min(100, minDotSlider.shown)
                    to: 100
                    value: channel.maxPct
                    accessibleName: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                    onRequested: p => channel.limitsRequested(channel.minPct, Math.max(p, channel.minPct))
                }
                Text {
                    Layout.preferredWidth: root.style.unit * 2.0
                    horizontalAlignment: Text.AlignRight
                    text: maxDotSlider.shown + "%"
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.ink
                }
            }
        }
    }

    ColumnLayout {
        id: content

        x: root.style.pad
        y: root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap

        Text {
            Layout.fillWidth: true
            text: i18nd(root.domain, "Displays")
            textFormat: Text.PlainText
            font: root.style.label
            color: root.style.muted
        }

        // --- the master row: every display at once -------------------------------------
        Channel {
            visible: root.showAllDisplays
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
            controllable: root.client.isConnected
            detail: ""
            accessibleValue: percent >= 0 ? i18ndc(root.domain, "Brightness percentage", "%1%", percent)
                                          : i18ndc(root.domain, "No value is known yet", "—")
            onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
        }

        // --- one block per display ------------------------------------------------------
        Repeater {
            model: root.monitors

            delegate: Channel {
                id: strip

                required property var modelData
                required property int index

                readonly property string monitorId: modelData.logicalId
                readonly property bool unavailable: Monitors.unavailable(modelData)
                readonly property bool hasHold: modelData.hasOverride !== true && Monitors.hasExternalHold(modelData)
                /*! What the slider shows: the override, else the applied value; -1 unknown. */
                readonly property int level: modelData.percent !== undefined ? modelData.percent : -1
                readonly property real aimAt: DayCurve.targetAt(root.client.dayCurve, monitorId,
                                                                  root.client.nowEpochS)

                ruled: root.showAllDisplays || index > 0
                name: monitorId
                percent: strip.level
                minPct: modelData.minPct !== undefined ? modelData.minPct : 15
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
                hasLimits: true
                limitsOpen: Monitors.isExpanded(root.expandedLimits, monitorId)
                controllable: modelData.enabled === true && !unavailable
                hasOverride: modelData.hasOverride === true || hasHold
                overrideBadge: hasHold
                    ? i18ndc(root.domain, "External brightness hold badge, compact", "EXT · %1",
                             Monitors.externalHoldAge(Monitors.externalHoldSeconds(modelData)))
                    : (root.client.overrideUntilEpochS > root.client.nowEpochS
                        ? i18ndc(root.domain, "Override badge: set by hand until a time of day", "MAN until %1",
                                 Format.timeText(root.client.overrideUntilEpochS))
                        : i18ndc(root.domain, "Override badge: set by hand, no end time", "MAN"))
                detail: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "OFF");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "NO LINK");
                    }
                    const aim = aimAt >= 0 ? String(Math.round(aimAt)) : "—";
                    return i18ndc(root.domain, "Target and automatic range of a display, in small caps",
                                  "TARGET %1 · RANGE %2–%3", aim, minPct, maxPct);
                }
                accessibleValue: level >= 0 ? i18ndc(root.domain, "Brightness percentage", "%1%", level)
                                            : i18ndc(root.domain, "No value is known yet", "—")
                onRequested: percent => root.client.setMonitorOverride(monitorId, percent, root.settings.overrideMinutes)
                onLimitsToggled: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, monitorId)
                onLimitsRequested: (lo, hi) => root.client.setMonitorLimits(monitorId, lo, hi)
                onClearRequested: strip.hasHold
                    ? root.client.clearExternalHold(monitorId)
                    : root.client.clearMonitorOverride(monitorId)
            }
        }
    }
}
