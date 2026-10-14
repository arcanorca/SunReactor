/*
 * Automation, Windows XP: the mode as XP radio buttons in a group box captioned
 * Mode. Automatic is the default choice, ringed. Paused has the arrow that opens
 * the durations. The status line under it says what the daemon is doing and until
 * when. The rules are in modules/ModeControl.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

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

    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected
    title: i18ndc(control.domain, "Window title of the automation mode", "Automation")

    function openDurations(anchorItem) {
        durations.x = Math.max(0, anchorItem.mapToItem(root, 0, 0).x);
        durations.y = anchorItem.mapToItem(root, 0, anchorItem.height).y;
        durations.open();
    }

    WinGroupBox {
        Layout.fillWidth: true
        style: root.ui
        caption: i18ndc(root.control.domain, "Group box caption over the mode radio buttons", "Mode")

        RowLayout {
            Layout.fillWidth: true
            spacing: root.ui.gap2

            Repeater {
                model: root.control.choices

                delegate: RowLayout {
                    id: choice

                    required property var modelData

                    spacing: root.ui.gap

                    WinRadio {
                        id: radio

                        style: root.ui
                        text: choice.modelData.label
                        checked: root.control.choice === choice.modelData.id
                        isDefault: choice.modelData.id === "automatic"
                        onClicked: root.control.pick(choice.modelData.id, radio)
                    }
                    WinButton {
                        id: arrow

                        visible: choice.modelData.id === "paused"
                        style: root.ui
                        text: "▾"
                        implicitWidth: root.ui.snap(root.ui.sizeBody * 1.6)
                        onClicked: root.openDurations(arrow)
                    }
                }
            }
            Item {
                Layout.fillWidth: true
            }
        }
    }

    Text {
        Layout.fillWidth: true
        text: root.control.displayText
        textFormat: Text.PlainText
        font: root.ui.bodyFont
        color: root.control.hinting ? root.ui.frame : root.ui.ink
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }
    Text {
        Layout.fillWidth: true
        visible: text.length > 0
        text: root.control.note
        textFormat: Text.PlainText
        font: root.ui.smallFont
        color: root.ui.inkMuted
    }

    ModeDurationMenu {
        id: durations

        parent: root
        control: root.control
    }
}
