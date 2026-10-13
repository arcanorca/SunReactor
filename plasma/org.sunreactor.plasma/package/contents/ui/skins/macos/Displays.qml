/*
 * Displays, macos: Control Center's brightness sliders in one inset list. A row
 * per display, and an All row once there is more than one. The module reports
 * what the user asked for; the daemon decides what that means.
 *
 *   ╭──────────────────────────────────────────────╮
 *   │ All displays                                 │
 *   │ ██████████████████████░░░░░░░░░░░░░░░░  39%  │
 *   │ ──────────────────────────────────────────── │
 *   │ mi-monitor                    [Auto] [Range] │
 *   │ ████████████████░░░░░░░░░░░░░░░░░░░░░░  41%  │
 *   ╰──────────────────────────────────────────────╯
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Accessible name of the brightness list", "Displays")

    Surface {
        Layout.fillWidth: true
        style: root.style
        radius: root.style.radiusList
        padding: 0
        spacing: 0

        DisplayRow {
            visible: root.showAllDisplays
            Layout.fillWidth: true
            style: root.style
            domain: root.domain
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
            hasLimits: false
            hasOverride: false
            divided: true

            onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
        }

        Repeater {
            model: root.monitors

            delegate: DisplayRow {
                id: row

                required property var modelData
                required property int index

                readonly property bool unavailable: modelData.unreachable
                    || modelData.topology === "temporarily_unavailable"

                Layout.fillWidth: true
                style: root.style
                domain: root.domain
                name: modelData.logicalId
                percent: modelData.percent
                minPct: modelData.minPct !== undefined ? modelData.minPct : 15
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
                hasLimits: true
                hasOverride: modelData.hasOverride === true
                controllable: modelData.enabled && !unavailable
                divided: index < root.monitors.length - 1
                stateText: {
                    if (!modelData.enabled) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "Off");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "No link");
                    }
                    if (modelData.hasOverride) {
                        return i18ndc(root.domain, "Tag: this brightness was set by hand", "Manual");
                    }
                    return "";
                }
                stateTone: unavailable && modelData.enabled ? root.style.danger : root.style.inkSecondary

                onRequested: percent => root.client.setMonitorOverride(modelData.logicalId, percent,
                                                                       root.settings.overrideMinutes)
                onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId, minVal, maxVal)
                onClearOverrideRequested: () => root.client.clearMonitorOverride(modelData.logicalId)
            }
        }
    }
}
