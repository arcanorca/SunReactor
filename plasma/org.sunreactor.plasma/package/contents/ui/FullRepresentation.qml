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

PlasmaExtras.Representation {
    id: root

    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property var monitors: client.isConnected ? client.monitors : []
    /*! A master slider only earns its place once there is more than one display. */
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property int overrideMinutes: Plasmoid.configuration.overrideDurationMinutes

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
            spacing: Kirigami.Units.smallSpacing * 2

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
            }

            WeatherItem {
                visible: root.client.isConnected && root.client.weatherEnabled
                client: root.client
            }

            ForecastStrip {
                visible: root.client.isConnected && root.client.weatherEnabled
                client: root.client
            }

            DisplayItem {
                visible: root.client.isConnected && root.showAllDisplays
                text: i18nd(root.domain, "All displays")
                iconName: "brightness-high"
                percent: root.client.globalPercent
                hint: root.client.isOverrideActive
                    ? i18ndc(root.domain, "This brightness was set by hand", "Manual")
                    : ""
                onRequested: percent => root.client.setGlobalOverride(percent, root.overrideMinutes)
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

                    onRequested: percent => root.client.setMonitorOverride(modelData.logicalId,
                                                                          percent,
                                                                          root.overrideMinutes)
                }
            }

            SunTimesRow {
                id: sunTimes
                visible: root.client.isConnected
                client: root.client
                showSolarElevation: Plasmoid.configuration.showSolarElevation
            }
        }
    }

    Timer {
        interval: 2500
        running: true
        onTriggered: console.log("SR-SIZE implicit=" + root.implicitHeight
            + " height=" + root.height
            + " preferred=" + root.Layout.preferredHeight
            + " max=" + root.Layout.maximumHeight
            + " column=" + contentColumn.implicitHeight
            + " scrollView=" + scrollView.height
            + " sunTimesVisible=" + sunTimes.visible
            + " sunTimesY=" + sunTimes.y + " sunTimesH=" + sunTimes.height
            + " footerH=" + (root.footer ? root.footer.height : -1))
    }

    footer: PopupFooter {
        client: root.client
        defaultPauseMinutes: Plasmoid.configuration.defaultSuspendMinutes
        visible: root.client.isConnected
    }
}
