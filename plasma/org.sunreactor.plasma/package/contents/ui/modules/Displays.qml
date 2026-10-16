/*
 * Displays module: one row per display (name, value, override, limits) and an
 * "All displays" master row once there is more than one. Unknown values stay
 * "—" with a disabled slider; sliders write overrides, never on scroll.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: client.isConnected ? client.monitors : []
    /*! A master slider only earns its place once there is more than one display. */
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0

    spacing: root.theme.spacing

    DisplayItem {
        visible: root.showAllDisplays
        theme: root.theme
        text: i18nd(root.domain, "All displays")
        iconName: "brightness-high"
        percent: root.client.globalPercent
        hint: root.client.isOverrideActive ? i18ndc(root.domain, "This brightness was set by hand", "Manual") : ""
        onRequested: percent => root.client.setGlobalOverride(percent, root.settings.overrideMinutes)
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.leftMargin: root.theme.spacing
        Layout.rightMargin: root.theme.spacing
        implicitHeight: root.theme.hairline
        color: root.theme.line
        visible: root.showAllDisplays
    }

    Repeater {
        model: root.monitors

        delegate: DisplayItem {
            id: display

            required property var modelData

            readonly property bool unavailable: modelData.unreachable
                || modelData.topology === "temporarily_unavailable"

            theme: root.theme
            text: modelData.logicalId
            iconName: "video-display-brightness"
            percent: modelData.percent
            minPct: modelData.minPct !== undefined ? modelData.minPct : 15
            maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
            hasLimits: true
            hasOverride: modelData.hasOverride === true
            controllable: modelData.enabled && !unavailable
            hint: {
                if (!modelData.enabled) {
                    return i18ndc(root.domain, "This display is excluded from automation", "Off");
                }
                if (unavailable) {
                    return i18ndc(root.domain, "This display is not answering", "Unavailable");
                }
                if (modelData.hasOverride) {
                    return i18ndc(root.domain, "This brightness was set by hand", "Manual");
                }
                return "";
            }

            onRequested: percent => root.client.setMonitorOverride(modelData.logicalId, percent,
                                                                   root.settings.overrideMinutes)
            onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId, minVal, maxVal)
            onClearOverrideRequested: () => root.client.clearMonitorOverride(modelData.logicalId)
        }
    }
}
