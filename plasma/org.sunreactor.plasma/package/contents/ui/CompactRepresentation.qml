/*
 * The panel item: the current sky as pixel art, with the temperature and -
 * when asked for - the sun's height beside it.
 *
 * Scrolling deliberately does nothing. A wheel event over a panel icon is far
 * too easy to trigger by accident, and the accident here is every monitor
 * changing brightness.
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

MouseArea {
    id: root

    required property PlasmoidItem plasmoidItem
    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property bool vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
    readonly property bool horizontal: Plasmoid.formFactor === PlasmaCore.Types.Horizontal
    readonly property bool showTemperature: Plasmoid.configuration.showTemperature
        && client.isConnected && client.hasTemperature
    readonly property bool showElevation: Plasmoid.configuration.showSolarElevation
        && client.isConnected && client.hasSolarElevation
    readonly property bool showLabels: showTemperature || showElevation

    /*! Guards the click that follows the popup closing on press. */
    property bool wasExpanded: false

    function togglePause() {
        if (client.isSuspended) {
            client.resume();
            return;
        }
        const minutes = Plasmoid.configuration.defaultSuspendMinutes;
        client.suspend(minutes > 0 ? minutes : 60);
    }

    // The panel fixes one axis and lets the content decide the other.
    Layout.minimumWidth: root.vertical ? 0 : content.implicitWidth
    Layout.preferredWidth: root.vertical ? Kirigami.Units.iconSizes.medium : content.implicitWidth
    Layout.maximumWidth: root.vertical ? Number.POSITIVE_INFINITY : content.implicitWidth
    Layout.minimumHeight: root.vertical ? content.implicitHeight : 0
    Layout.preferredHeight: root.vertical ? content.implicitHeight : Kirigami.Units.iconSizes.medium
    Layout.maximumHeight: root.vertical ? content.implicitHeight : Number.POSITIVE_INFINITY

    activeFocusOnTab: true
    hoverEnabled: true
    acceptedButtons: Qt.LeftButton | Qt.MiddleButton

    Accessible.role: Accessible.Button
    Accessible.name: Plasmoid.title
    Accessible.description: plasmoidItem.toolTipSubText

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.plasmoidItem.expanded = !root.plasmoidItem.expanded;
            event.accepted = true;
        }
    }

    GridLayout {
        id: content

        anchors.centerIn: parent
        rows: root.vertical ? 2 : 1
        columns: root.vertical ? 1 : 2
        rowSpacing: 0
        columnSpacing: Kirigami.Units.smallSpacing

        Kirigami.Icon {
            id: sky

            readonly property int available: root.vertical
                ? Math.min(root.width, Math.max(Kirigami.Units.iconSizes.small,
                                                root.height - (root.showLabels ? labels.implicitHeight : 0)))
                : (root.horizontal ? root.height : Kirigami.Units.iconSizes.medium)

            /*! The sprites are drawn on a 16px grid; anything other than a
                whole multiple of it turns the pixels into mush. */
            readonly property int crisp: 16 * Math.max(1, Math.floor(available / 16))

            Layout.alignment: Qt.AlignCenter
            implicitWidth: crisp
            implicitHeight: crisp
            source: Plasmoid.icon
            roundToIconSize: false
            active: root.containsMouse
        }

        ColumnLayout {
            id: labels

            Layout.alignment: Qt.AlignCenter
            spacing: 0
            visible: root.showLabels

            PlasmaComponents3.Label {
                Layout.alignment: Qt.AlignCenter
                visible: root.showTemperature
                text: i18ndc(root.domain, "Temperature in degrees, panel-sized", "%1°",
                             Math.round(root.client.temperatureC))
                textFormat: Text.PlainText
                font.features: ({ "tnum": 1 })
                font.pixelSize: root.vertical || root.showElevation
                    ? Kirigami.Theme.smallFont.pixelSize
                    : Kirigami.Theme.defaultFont.pixelSize
            }

            PlasmaComponents3.Label {
                Layout.alignment: Qt.AlignCenter
                visible: root.showElevation
                text: i18ndc(root.domain, "Sun height above the horizon, panel-sized", "%1°",
                             root.client.solarElevation.toFixed(1))
                textFormat: Text.PlainText
                font.features: ({ "tnum": 1 })
                font.pixelSize: Kirigami.Theme.smallFont.pixelSize
                opacity: 0.75
            }
        }
    }

    onPressed: wasExpanded = plasmoidItem.expanded

    onClicked: mouse => {
        if (mouse.button === Qt.MiddleButton) {
            root.togglePause();
        } else {
            root.plasmoidItem.expanded = !root.wasExpanded;
        }
    }
}
