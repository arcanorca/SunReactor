/*
 * The panel item: one of the panel modes (panel/PanelView.qml - sky sprite,
 * brightness readout, today's sparkline, ring gauge) with the temperature
 * beside it, in horizontal and vertical panels and the system tray.
 * Click opens the popup, middle-click pauses or resumes.
 *
 * Scrolling deliberately does nothing. A wheel event over a panel icon is far
 * too easy to trigger by accident, and the accident here is every monitor
 * changing brightness.
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "engine"
import "panel"

MouseArea {
    id: root

    required property PlasmoidItem plasmoidItem
    required property SunReactorClient client

    PlasmoidEngine {
        id: engine
        client: root.client
    }

    readonly property Theme theme: engine.theme
    readonly property bool vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
    /*! System tray (and any containment that wants square items). */
    readonly property bool square: (Plasmoid.containmentDisplayHints & PlasmaCore.Types.ContainmentForcesSquarePlasmoids)
        || Plasmoid.formFactor === PlasmaCore.Types.Planar
    readonly property real thickness: square ? Math.min(width, height)
        : (vertical ? width : (height > 0 ? height : Kirigami.Units.iconSizes.medium))

    /*! Guards the click that follows the popup closing on press. */
    property bool wasExpanded: false

    function togglePause() {
        if (client.isSuspended) {
            client.resume();
        } else {
            const minutes = Plasmoid.configuration.defaultSuspendMinutes;
            client.suspend(minutes > 0 ? minutes : 60);
        }
    }

    // The panel fixes one axis and the content decides the other; a tray
    // cell is square and decided by the tray.
    Layout.minimumWidth: square || vertical ? 0 : view.implicitWidth
    Layout.preferredWidth: square ? -1 : (vertical ? Kirigami.Units.iconSizes.medium : view.implicitWidth)
    Layout.maximumWidth: square || vertical ? Number.POSITIVE_INFINITY : view.implicitWidth
    Layout.minimumHeight: square || !vertical ? 0 : view.implicitHeight
    Layout.preferredHeight: square ? -1 : (vertical ? view.implicitHeight : Kirigami.Units.iconSizes.medium)
    Layout.maximumHeight: square || !vertical ? Number.POSITIVE_INFINITY : view.implicitHeight

    activeFocusOnTab: true
    hoverEnabled: true
    acceptedButtons: Qt.LeftButton | Qt.MiddleButton

    Accessible.role: Accessible.Button
    Accessible.name: Plasmoid.title
    Accessible.description: plasmoidItem.toolTipMainText + "\n" + plasmoidItem.toolTipSubText
    Accessible.onPressAction: plasmoidItem.expanded = !plasmoidItem.expanded

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.plasmoidItem.expanded = !root.plasmoidItem.expanded;
            event.accepted = true;
        }
    }

    PanelView {
        id: view

        anchors.centerIn: parent
        theme: root.theme
        client: root.client
        mode: Plasmoid.configuration.panelMode
        displayId: Plasmoid.configuration.panelDisplay
        showTemperature: Plasmoid.configuration.showTemperature
        showElevation: Plasmoid.configuration.showSolarElevation
        vertical: root.vertical
        square: root.square
        thickness: root.thickness
        skySource: Plasmoid.icon
        hovered: root.containsMouse
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
