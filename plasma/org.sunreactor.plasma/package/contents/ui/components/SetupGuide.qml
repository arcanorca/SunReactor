/*
 * Setup & offline helper card: pure QML (no C++ plugin import) so it renders
 * cleanly both when installed from KDE Store before the native daemon/plugin
 * is present, and when `sunreactord` is offline.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami

Item {
    id: root

    property string domain: "plasma_applet_org.sunreactor.plasma"
    property bool pluginMissing: false
    property string iconName: "org.sunreactor.plasma"
    property string title: ""
    property string explanation: ""
    property bool showStartCommand: true
    property bool showInstallCommand: true

    readonly property string installCommand: "curl -sL https://raw.githubusercontent.com/arcanorca/SunReactor/main/install.sh | bash"
    readonly property string startCommand: "systemctl --user start sunreactord"
    readonly property string reloadCommand: "systemctl --user restart plasma-plasmashell.service"

    function tr(msg: string): string {
        return i18nd(root.domain, msg); // qmllint disable unqualified
    }

    implicitWidth: card.implicitWidth
    implicitHeight: card.implicitHeight + Kirigami.Units.largeSpacing

    Rectangle {
        id: card

        anchors {
            left: parent.left
            right: parent.right
            verticalCenter: parent.verticalCenter
        }
        implicitHeight: layout.implicitHeight + 2 * Kirigami.Units.largeSpacing
        radius: Kirigami.Units.cornerRadius * 2
        color: Qt.alpha(Kirigami.Theme.backgroundColor, 0.72)
        border.width: 1
        border.color: Qt.alpha(Kirigami.Theme.textColor, 0.14)

        ColumnLayout {
            id: layout

            anchors {
                fill: parent
                margins: Kirigami.Units.largeSpacing
            }
            spacing: Kirigami.Units.mediumSpacing

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.mediumSpacing

                Kirigami.Icon {
                    Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                    Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                    Layout.alignment: Qt.AlignTop
                    source: root.iconName
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing / 2

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: root.title.length > 0
                            ? root.title
                            : root.tr("Install SunReactor Service")
                        textFormat: Text.PlainText
                        font.weight: Font.DemiBold
                        wrapMode: Text.WordWrap
                    }

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: root.explanation.length > 0
                            ? root.explanation
                            : root.tr("This KDE Store package needs the SunReactor background daemon and C++ Plasma plugin. Run one command to install everything automatically:")
                        textFormat: Text.PlainText
                        font: Kirigami.Theme.smallFont
                        color: Kirigami.Theme.disabledTextColor
                        wrapMode: Text.WordWrap
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                visible: !root.pluginMissing && root.showStartCommand
                spacing: Kirigami.Units.smallSpacing / 2

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.tr("Start background service:")
                    textFormat: Text.PlainText
                    font: Kirigami.Theme.smallFont
                    opacity: 0.85
                }

                CopyCommand {
                    Layout.fillWidth: true
                    expand: true
                    domain: root.domain
                    command: root.startCommand
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                visible: root.showInstallCommand || root.pluginMissing
                spacing: Kirigami.Units.smallSpacing / 2

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.pluginMissing
                        ? root.tr("1. Install daemon and native plugin:")
                        : root.tr("Not installed yet? Single-shot setup:")
                    textFormat: Text.PlainText
                    font: Kirigami.Theme.smallFont
                    opacity: 0.85
                }

                CopyCommand {
                    Layout.fillWidth: true
                    expand: true
                    domain: root.domain
                    command: root.installCommand
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                visible: root.pluginMissing
                spacing: Kirigami.Units.smallSpacing / 2

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.tr("2. Reload Plasma:")
                    textFormat: Text.PlainText
                    font: Kirigami.Theme.smallFont
                    opacity: 0.85
                }

                CopyCommand {
                    Layout.fillWidth: true
                    expand: true
                    domain: root.domain
                    command: root.reloadCommand
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.tr("Arch/AUR: yay -S plasma6-applets-sunreactor · Uninstall: add --uninstall")
                    textFormat: Text.PlainText
                    font: Kirigami.Theme.smallFont
                    color: Kirigami.Theme.disabledTextColor
                    wrapMode: Text.WordWrap
                }

                PlasmaComponents3.ToolButton {
                    icon.name: "internet-web-browser"
                    text: "GitHub"
                    display: QQC2.AbstractButton.TextBesideIcon
                    font: Kirigami.Theme.smallFont
                    onClicked: Qt.openUrlExternally("https://github.com/arcanorca/SunReactor")
                }
            }
        }
    }
}
