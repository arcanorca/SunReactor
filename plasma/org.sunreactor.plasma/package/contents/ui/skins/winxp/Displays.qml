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
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

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
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0
    title: i18ndc(domain, "Window title of the display brightness", "Displays")

    /*! One display: name, trackbar, value and range; the range trackbars open below. */
    component DisplayRow: ColumnLayout {
        id: row

        required property WinStyle style
        property string name: ""
        property int percent: -1
        property int minPct: 15
        property int maxPct: 60
        property bool hasLimits: false
        property bool hasOverride: false
        property bool controllable: true
        property string hint: ""
        property bool limitsOpen: false

        /*! The name column, shared by the row and its range trackbars so the
            trackbars line up. Wide enough for a typical monitor model name. */
        readonly property real nameWidth: row.style.snap(row.style.unit * 7)

        signal requested(int percent)
        signal limitsRequested(int minVal, int maxVal)
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
                    cursorShape: Qt.PointingHandCursor
                    onClicked: row.limitsOpen = !row.limitsOpen
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
            Text {
                Layout.preferredWidth: row.style.snap(row.style.unit * 2.6)
                horizontalAlignment: Text.AlignRight
                text: row.hint.length > 0 ? row.hint
                    : (row.hasLimits ? row.minPct + "–" + row.maxPct : "")
                textFormat: Text.PlainText
                font: row.style.smallBoldFont
                color: row.hasOverride ? row.style.frame : row.style.inkMuted

                MouseArea {
                    anchors.fill: parent
                    enabled: row.hasOverride
                    cursorShape: Qt.PointingHandCursor
                    onClicked: row.clearOverrideRequested()
                }
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
                    Layout.fillWidth: true
                    style: row.style
                    value: row.minPct
                    controllable: row.controllable
                    onRequested: v => row.limitsRequested(v, row.maxPct)
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
                    Layout.fillWidth: true
                    style: row.style
                    value: row.maxPct
                    controllable: row.controllable
                    onRequested: v => row.limitsRequested(row.minPct, v)
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
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
            hint: root.client.isOverrideActive
                ? i18ndc(root.domain, "This brightness was set by hand", "Manual") : ""
            onRequested: v => root.client.setGlobalOverride(v, root.settings.overrideMinutes)
        }

        Repeater {
            model: root.monitors

            delegate: DisplayRow {
                id: display

                required property var modelData

                readonly property bool unavailable: modelData.unreachable
                    || modelData.topology === "temporarily_unavailable"

                Layout.fillWidth: true
                style: root.ui
                name: modelData.logicalId
                percent: modelData.percent
                minPct: modelData.minPct !== undefined ? modelData.minPct : 15
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
                hasLimits: true
                hasOverride: modelData.hasOverride === true
                controllable: modelData.enabled && !unavailable
                hint: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Display tag: excluded from automation", "Off");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Display tag: not answering", "No link");
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "This brightness was set by hand", "Manual");
                    }
                    return "";
                }

                onRequested: v => root.client.setMonitorOverride(modelData.logicalId, v,
                                                                 root.settings.overrideMinutes)
                onLimitsRequested: (lo, hi) => root.client.setMonitorLimits(modelData.logicalId, lo, hi)
                onClearOverrideRequested: root.client.clearMonitorOverride(modelData.logicalId)
            }
        }
    }
}
