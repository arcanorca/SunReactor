/*
 * Displays, Windows XP: one row per display in a group box captioned Brightness.
 * A row is the name, an XP trackbar whose groove is the green segmented progress
 * bar up to the value, then the value and the automatic range (or the tag that
 * says why the display is not automatic). Clicking the name opens two more
 * trackbars for the range. "All displays" leads once there is more than one.
 * Dragging writes an override; the Manual tag clears it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../modules/Monitors.js" as Monitors

WinWindow {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    style: ui
    width: parent ? parent.width : 0

    readonly property string domain: settings.domain
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: Monitors.hasDisplayData(client)
    property var expandedLimits: ({})
    title: i18ndc(domain, "Window title of the display brightness", "Displays")

    /*! One display: name, trackbar, value and range; the range trackbars open below. */
    component DisplayRow: ColumnLayout {
        id: row

        required property WinStyle style
        property string name: ""
        property int percent: -1
        property int minPct: -1
        property int maxPct: -1
        property bool hasLimits: false
        property bool hasOverride: false
        property bool controllable: true
        property string hint: ""
        property string notice: ""
        property bool limitsOpen: false

        /*! The name column, shared by the row and its range trackbars so the
            trackbars line up. Wide enough for a typical monitor model name. */
        readonly property real nameWidth: row.style.snap(row.style.unit * 7)

        signal requested(int percent)
        signal limitsRequested(int minVal, int maxVal)
        signal limitsToggled()
        signal clearOverrideRequested()

        spacing: row.style.gap

        RowLayout {
            Layout.fillWidth: true
            spacing: row.style.gap

            Text {
                Layout.preferredWidth: row.nameWidth
                Layout.maximumWidth: row.nameWidth
                text: row.name
                textFormat: Text.PlainText
                font: row.style.boldFont
                color: row.limitsOpen ? row.style.frame : row.style.ink
                elide: Text.ElideRight

                MouseArea {
                    anchors.fill: parent
                    enabled: row.hasLimits
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: row.limitsToggled()
                }
            }
            WinTrackbar {
                Layout.fillWidth: true
                style: row.style
                value: Math.max(0, row.percent)
                controllable: row.controllable && row.percent >= 0
                onRequested: v => row.requested(v)
            }
            Text {
                Layout.preferredWidth: row.style.snap(row.style.unit * 2.1)
                horizontalAlignment: Text.AlignRight
                text: row.percent >= 0 ? row.percent + "%" : "—"
                textFormat: Text.PlainText
                font: row.style.figureFont
                color: row.style.ink
            }
            WinButton {
                visible: row.hasOverride || row.hasLimits || row.hint.length > 0
                enabled: row.hasOverride ? root.client.isConnected : row.hasLimits
                Layout.preferredWidth: Math.max(row.style.snap(row.style.unit * 2.9), implicitWidth)
                Layout.preferredHeight: row.style.snap(row.style.sizeBody * 1.45)
                style: row.style
                current: !row.hasOverride && row.limitsOpen
                text: row.hint.length > 0 ? row.hint
                    : (row.hasLimits ? row.minPct + "–" + row.maxPct : "")
                onClicked: row.hasOverride ? row.clearOverrideRequested() : row.limitsToggled()

                HoverHandler {
                    id: hintHover
                }
                QQC2.ToolTip.visible: hintHover.hovered && row.notice.length > 0
                QQC2.ToolTip.text: row.notice
                QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: row.limitsOpen && row.hasLimits
            spacing: row.style.gap

            RowLayout {
                Layout.fillWidth: true
                spacing: row.style.gap

                Text {
                    Layout.preferredWidth: row.nameWidth
                    text: i18ndc(root.domain, "Range trackbar: the automatic minimum", "Min")
                    textFormat: Text.PlainText
                    font: row.style.smallFont
                    color: row.style.inkMuted
                }
                WinTrackbar {
                    id: minTrack
                    Layout.fillWidth: true
                    style: row.style
                    from: 0
                    to: Math.max(0, row.maxPct)
                    value: row.minPct
                    controllable: row.controllable && row.hasLimits
                    onRequested: v => row.limitsRequested(Math.min(v, row.maxPct), row.maxPct)
                }
                Text {
                    Layout.preferredWidth: row.style.snap(row.style.unit * 2.1)
                    horizontalAlignment: Text.AlignRight
                    text: minTrack.shown + "%"
                    textFormat: Text.PlainText
                    font: row.style.figureFont
                    color: row.style.ink
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: row.style.gap

                Text {
                    Layout.preferredWidth: row.nameWidth
                    text: i18ndc(root.domain, "Range trackbar: the automatic maximum", "Max")
                    textFormat: Text.PlainText
                    font: row.style.smallFont
                    color: row.style.inkMuted
                }
                WinTrackbar {
                    id: maxTrack
                    Layout.fillWidth: true
                    style: row.style
                    from: Math.min(100, row.minPct)
                    to: 100
                    value: row.maxPct
                    controllable: row.controllable && row.hasLimits
                    onRequested: v => row.limitsRequested(row.minPct, Math.max(v, row.minPct))
                }
                Text {
                    Layout.preferredWidth: row.style.snap(row.style.unit * 2.1)
                    horizontalAlignment: Text.AlignRight
                    text: maxTrack.shown + "%"
                    textFormat: Text.PlainText
                    font: row.style.figureFont
                    color: row.style.ink
                }
            }
        }
    }

    WinGroupBox {
        Layout.fillWidth: true
        style: root.ui
        caption: i18ndc(root.domain, "Group box caption over the display rows", "Brightness")

        DisplayRow {
            Layout.fillWidth: true
            visible: root.showAllDisplays
            style: root.ui
            name: root.settings.allDisplaysLabel
            percent: root.client.globalPercent
            controllable: root.client.isConnected
            hasOverride: root.client.isOverrideActive
            hint: root.client.isOverrideActive
                ? i18ndc(root.domain, "This brightness was set by hand", "Manual") : ""
            onRequested: v => root.client.setGlobalOverride(v, root.settings.overrideMinutes)
            onClearOverrideRequested: root.client.clearAllOverrides()
        }

        Repeater {
            model: root.monitors

            delegate: DisplayRow {
                id: display

                required property var modelData

                readonly property bool unavailable: Monitors.unavailable(modelData)
                readonly property bool hasHold: modelData.hasOverride !== true && Monitors.hasExternalHold(modelData)
                readonly property string tagNotice: Monitors.monitorTag(
                    modelData, root.settings.pausedTag, root.settings.queuedTag)

                Layout.fillWidth: true
                style: root.ui
                name: modelData.logicalId
                percent: modelData.percent
                minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                hasLimits: Monitors.hasValidLimits(modelData)
                limitsOpen: Monitors.isExpanded(root.expandedLimits, modelData.logicalId)
                hasOverride: modelData.hasOverride === true || hasHold
                controllable: root.client.isConnected && modelData.enabled && !unavailable
                notice: Monitors.monitorNotice(
                    modelData, root.settings.contestedNotice, root.settings.deferredNotice)
                hint: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Display tag: excluded from automation", "Off");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Display tag: not answering", "No link");
                    }
                    if (tagNotice.length > 0) {
                        return tagNotice;
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "This brightness was set by hand", "Manual");
                    }
                    if (hasHold) {
                        return i18ndc(root.domain, "External brightness hold badge", "Ext · %1",
                                      Monitors.externalHoldAge(Monitors.externalHoldSeconds(modelData)));
                    }
                    return "";
                }

                onRequested: v => root.client.setMonitorOverride(modelData.logicalId, v,
                                                                 root.settings.overrideMinutes)
                onLimitsToggled: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, modelData.logicalId)
                onLimitsRequested: (lo, hi) => root.client.setMonitorLimits(modelData.logicalId, lo, hi)
                onClearOverrideRequested: hasHold
                    ? root.client.clearExternalHold(modelData.logicalId)
                    : root.client.clearMonitorOverride(modelData.logicalId)
            }
        }
    }
}
