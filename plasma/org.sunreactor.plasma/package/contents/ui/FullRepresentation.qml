/*
 * The popup: an instrument, composed of modules in the order the theme lists
 * them (theme.widget.modules; see modules/Modules.js for the contract).
 *
 * Structure follows Plasma's own brightness applet - a Representation holding
 * a scrollable column of flat rows, with a footer for the actions. There is no
 * title: the popup does not repeat the name of the widget. When a daemon theme
 * is in effect the content sits on the theme's background.
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
import "engine"
import "modules"
import "modules/Modules.js" as Modules

PlasmaExtras.Representation {
    id: root

    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property Theme theme: engine.theme

    PlasmoidEngine {
        id: engine

        client: root.client
    }

    ModuleSettings {
        id: moduleSettings

        overrideMinutes: Plasmoid.configuration.overrideDurationMinutes
        defaultSuspendMinutes: Plasmoid.configuration.defaultSuspendMinutes
        showSolarElevation: Plasmoid.configuration.showSolarElevation
    }

    Layout.minimumWidth: Kirigami.Units.gridUnit * 16
    Layout.preferredWidth: Kirigami.Units.gridUnit * 22
    Layout.maximumWidth: Kirigami.Units.gridUnit * 40
    Layout.minimumHeight: Math.min(implicitHeight, Kirigami.Units.gridUnit * 30)
    // Grows with the content instead of reserving a fixed slab.
    Layout.preferredHeight: Math.min(implicitHeight, Kirigami.Units.gridUnit * 38)
    Layout.maximumHeight: Kirigami.Units.gridUnit * 38

    collapseMarginsHint: true

    background: Frame {
        theme: root.theme
        visible: root.theme.available
        pixel: false
        fillColor: root.theme.bg
        color: root.theme.line
        style: root.theme.border === "none" ? "none" : "line"
    }

    contentItem: PlasmaComponents3.ScrollView {
        id: scrollView

        implicitHeight: contentColumn.implicitHeight
        PlasmaComponents3.ScrollBar.horizontal.visible: false

        ColumnLayout {
            id: contentColumn

            width: scrollView.availableWidth
            spacing: root.theme.sectionSpacing

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

            Repeater {
                model: root.theme.modules

                delegate: Loader {
                    id: slot

                    required property string modelData

                    Layout.fillWidth: true
                    Layout.leftMargin: root.theme.available ? Kirigami.Units.smallSpacing : 0
                    Layout.rightMargin: Layout.leftMargin
                    // Every module exposes `available` (modules/Modules.js); the
                    // loaded type is only known at run time.
                    visible: status === Loader.Ready && item.available === true // qmllint disable missing-property
                    active: Modules.file(modelData).length > 0

                    Component.onCompleted: {
                        const file = Modules.file(modelData);
                        if (file.length > 0) {
                            setSource(Qt.resolvedUrl(Modules.source(modelData, root.theme.themeId, root.theme.renderer)), {
                                theme: root.theme,
                                client: root.client,
                                settings: moduleSettings,
                            });
                        }
                    }
                }
            }
        }
    }

    footer: PopupFooter {
        client: root.client
        theme: root.theme
        defaultPauseMinutes: Plasmoid.configuration.defaultSuspendMinutes
        themeMode: Plasmoid.configuration.themeMode
        pinnedId: Plasmoid.configuration.pinnedTheme
        pinNotice: engine.notice
        visible: root.client.isConnected
        onFollowRequested: Plasmoid.configuration.themeMode = "follow"
        onPinRequested: id => {
            Plasmoid.configuration.pinnedTheme = id;
            Plasmoid.configuration.themeMode = "pinned";
        }
    }
}
