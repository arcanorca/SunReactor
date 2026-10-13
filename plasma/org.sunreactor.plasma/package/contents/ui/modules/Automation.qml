/*
 * Automation, plain Plasma look (no daemon theme): the mode as three buttons,
 * Automatic | Manual | Paused, with an arrow beside Paused for other durations;
 * under them, what the daemon is doing and until when. The rules are in
 * ModeControl.qml, shared with every skin.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property bool available: client.isConnected

    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }

    spacing: Kirigami.Units.smallSpacing

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode buttons", "Automation mode")

    RowLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        PlasmaComponents3.Button {
            id: automaticButton

            Layout.fillWidth: true
            text: root.control.choices[0].label
            // Not checkable: the state comes from the daemon, not from the click.
            checked: root.control.choice === "automatic"
            highlighted: root.control.choice !== "automatic" && root.control.choice !== ""
            Accessible.role: Accessible.RadioButton
            Accessible.checkable: true
            Accessible.checked: checked
            Accessible.description: root.control.choices[0].tip
            KeyNavigation.right: manualButton
            onClicked: root.control.pick("automatic", root)
            Keys.onReturnPressed: root.control.pick("automatic", root)
        }

        PlasmaComponents3.Button {
            id: manualButton

            Layout.fillWidth: true
            text: root.control.choices[1].label
            checked: root.control.choice === "manual"
            Accessible.role: Accessible.RadioButton
            Accessible.checkable: true
            Accessible.checked: checked
            Accessible.description: root.control.choices[1].tip
            KeyNavigation.left: automaticButton
            KeyNavigation.right: pausedButton
            onClicked: root.control.pick("manual", root)
            Keys.onReturnPressed: root.control.pick("manual", root)
        }

        PlasmaComponents3.Button {
            id: pausedButton

            Layout.fillWidth: true
            text: root.control.choices[2].label
            checked: root.control.choice === "paused"
            Accessible.role: Accessible.RadioButton
            Accessible.checkable: true
            Accessible.checked: checked
            Accessible.description: root.control.choices[2].tip
            KeyNavigation.left: manualButton
            KeyNavigation.right: durationsButton
            onClicked: root.control.pick("paused", root)
            Keys.onReturnPressed: root.control.pick("paused", root)
            Keys.onDownPressed: durations.open()
        }

        PlasmaComponents3.ToolButton {
            id: durationsButton

            icon.name: "arrow-down"
            text: i18ndc(root.control.domain, "Accessible name of the pause durations arrow", "Pause durations")
            display: PlasmaComponents3.AbstractButton.IconOnly
            checked: durations.visible
            KeyNavigation.left: pausedButton
            onClicked: durations.visible ? durations.close() : durations.open()

            ModeDurationMenu {
                id: durations

                control: root.control
                y: durationsButton.height
            }
        }
    }

    PlasmaComponents3.Label {
        Layout.fillWidth: true
        visible: root.control.displayText.length > 0
        text: root.control.displayText
        textFormat: Text.PlainText
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
        color: root.control.hinting ? Kirigami.Theme.neutralTextColor : Kirigami.Theme.textColor
    }

    PlasmaExtras.DescriptiveLabel {
        Layout.fillWidth: true
        visible: text.length > 0
        text: root.control.note
        wrapMode: Text.WordWrap
    }
}
