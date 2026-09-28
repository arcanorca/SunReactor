/*
 * The popup: what the displays are set to, why, and what changes next.
 *
 * Structure follows Plasma's own brightness applet - a Representation holding
 * a scrollable list of flat item delegates, with a footer for the actions.
 * There is no title: the popup does not repeat the name of the widget.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "components"
import "wardrobe"

PlasmaExtras.Representation {
    id: root

    required property SunReactorClient client

    ThemeTokens {
        id: tokens
        client: root.client
    }

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property var monitors: client.isConnected ? client.monitors : []
    /*! A master slider only earns its place once there is more than one display. */
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property int overrideMinutes: Plasmoid.configuration.overrideDurationMinutes
    readonly property color accentColor: tokens.accentColor

    Layout.minimumWidth: Kirigami.Units.gridUnit * 16
    Layout.preferredWidth: Kirigami.Units.gridUnit * 22
    Layout.maximumWidth: Kirigami.Units.gridUnit * 40
    Layout.minimumHeight: Math.min(implicitHeight, Kirigami.Units.gridUnit * 30)
    // Grows with the number of displays instead of reserving a fixed slab.
    Layout.preferredHeight: Math.min(implicitHeight, Kirigami.Units.gridUnit * 34)
    Layout.maximumHeight: Kirigami.Units.gridUnit * 34

    collapseMarginsHint: true

    contentItem: PlasmaComponents3.ScrollView {
        id: scrollView

        implicitHeight: contentColumn.implicitHeight
        PlasmaComponents3.ScrollBar.horizontal.visible: false

        ColumnLayout {
            id: contentColumn

            width: scrollView.availableWidth
            spacing: Kirigami.Units.gridUnit

            PlasmaExtras.PlaceholderMessage {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.gridUnit
                Layout.bottomMargin: Kirigami.Units.gridUnit
                visible: !root.client.isConnected
                iconName: "network-disconnect"
                text: i18nd(root.domain, "Not connected")
                explanation: i18nd(root.domain,
                    "The background service is not answering. Start it with:\nsystemctl --user start sunreactord")
            }

            AutomationItem {
                // Nothing to say while the daemon is simply doing its job.
                visible: root.client.isConnected
                    && root.client.mode !== SunReactorClient.Automatic
                client: root.client
                tokens: tokens
            }

            // Displays section (Themed GNOME HIG grouped row container)
            WardrobeCard {
                Layout.fillWidth: true
                visible: root.client.isConnected && root.monitors.length > 0
                contentImplicitHeight: displaysColumn.implicitHeight + Kirigami.Units.smallSpacing * 2
                tokens: tokens

                ColumnLayout {
                    id: displaysColumn

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Kirigami.Units.smallSpacing
                    spacing: Kirigami.Units.smallSpacing

                    DisplayItem {
                        visible: root.showAllDisplays
                        text: i18nd(root.domain, "All displays")
                        iconName: "brightness-high"
                        percent: root.client.globalPercent
                        accentColor: root.accentColor
                        tokens: tokens
                        hint: root.client.isOverrideActive
                            ? i18ndc(root.domain, "This brightness was set by hand", "Manual")
                            : ""
                        onRequested: percent => root.client.setGlobalOverride(percent, root.overrideMinutes)
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.leftMargin: Kirigami.Units.smallSpacing
                        Layout.rightMargin: Kirigami.Units.smallSpacing
                        implicitHeight: 1
                        color: tokens.cardBorderColor
                        visible: root.showAllDisplays
                    }

                    Repeater {
                        model: root.monitors

                        delegate: DisplayItem {
                            id: display

                            required property var modelData

                            readonly property bool unavailable: modelData.unreachable
                                || modelData.topology === "temporarily_unavailable"

                            text: modelData.logicalId
                            iconName: "video-display-brightness"
                            percent: modelData.percent
                            minPct: modelData.minPct !== undefined ? modelData.minPct : 15
                            maxPct: modelData.maxPct !== undefined ? modelData.maxPct : 60
                            hasLimits: true
                            hasOverride: modelData.hasOverride !== undefined && modelData.hasOverride
                            controllable: modelData.enabled && !unavailable
                            accentColor: root.accentColor
                            tokens: tokens
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

                            onRequested: percent => root.client.setMonitorOverride(modelData.logicalId,
                                                                                  percent,
                                                                                  root.overrideMinutes)
                            onLimitsRequested: (minVal, maxVal) => root.client.setMonitorLimits(modelData.logicalId,
                                                                                               minVal,
                                                                                               maxVal)
                            onClearOverrideRequested: () => root.client.clearMonitorOverride(modelData.logicalId)
                        }
                    }
                }
            }

            // Atmosphere & Solar cycle section (Themed GNOME HIG grouped row container)
            WardrobeCard {
                Layout.fillWidth: true
                visible: root.client.isConnected
                    && (root.client.weatherEnabled || (root.client.sunriseEpochS > 0 && root.client.sunsetEpochS > 0))
                contentImplicitHeight: solarColumn.implicitHeight + Kirigami.Units.smallSpacing * 2
                tokens: tokens

                ColumnLayout {
                    id: solarColumn

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Kirigami.Units.smallSpacing
                    spacing: Kirigami.Units.smallSpacing

                    WeatherItem {
                        visible: root.client.weatherEnabled
                        client: root.client
                        tokens: tokens
                    }

                    ForecastStrip {
                        visible: root.client.weatherEnabled
                        client: root.client
                        tokens: tokens
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.leftMargin: Kirigami.Units.smallSpacing
                        Layout.rightMargin: Kirigami.Units.smallSpacing
                        implicitHeight: 1
                        color: tokens.cardBorderColor
                        visible: root.client.weatherEnabled
                            && (root.client.sunriseEpochS > 0 && root.client.sunsetEpochS > 0)
                    }

                    SunTimesRow {
                        id: sunTimes
                        visible: root.client.sunriseEpochS > 0 && root.client.sunsetEpochS > 0
                        client: root.client
                        tokens: tokens
                        showSolarElevation: Plasmoid.configuration.showSolarElevation
                    }
                }
            }
        }
    }

    footer: PopupFooter {
        client: root.client
        tokens: tokens
        defaultPauseMinutes: Plasmoid.configuration.defaultSuspendMinutes
        visible: root.client.isConnected
    }
}
