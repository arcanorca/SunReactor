/*
 * Automation, Wii style: the three modes as large rounded buttons, like the
 * OK and Back pills of a Wii menu. The chosen one is filled blue with white
 * text; while the daemon is not automatic, Automatic carries the blue outline,
 * the way back. Paused opens its pause durations from the chevron.
 * What each choice does is in modules/ModeControl.qml.
 *
 *   ( Automatic )  ( Manual )  ( Paused ▾ )
 *   Following the sun and the weather · Next 4:20 PM
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WiiStyle style: WiiStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected

    spacing: 0

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode switch", "Automation mode")

    function focusChoice(index) {
        const count = control.order.length;
        const choice = choices.itemAt((index + count) % count);
        if (choice) {
            choice.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    function openDurations(choice) {
        durations.x = Math.max(0, choice.mapToItem(root, 0, 0).x);
        durations.y = choice.mapToItem(root, 0, choice.height).y;
        durations.open();
    }

    WiiTile {
        id: tile

        style: root.style
        Layout.fillWidth: true
        implicitHeight: body.implicitHeight + 2 * tile.pad

        ColumnLayout {
            id: body

            anchors.fill: parent
            spacing: root.style.gap

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                Repeater {
                    id: choices

                    model: root.control.choices

                    delegate: WiiPill {
                        id: choice

                        required property var modelData
                        required property int index

                        readonly property string choiceId: modelData.id

                        Layout.fillWidth: true
                        Layout.preferredHeight: root.style.snap(root.style.unit * 1.9)
                        style: root.style
                        label: modelData.label
                        variant: "fill"
                        current: root.control.choice === choiceId
                        called: choiceId === "automatic" && root.control.choice !== ""
                            && root.control.choice !== "automatic"
                        arrow: choiceId === "paused"
                        focused: activeFocus
                        activeFocusOnTab: true

                        Accessible.role: Accessible.RadioButton
                        Accessible.name: modelData.label
                        Accessible.description: modelData.tip
                        Accessible.checkable: true
                        Accessible.checked: choice.current
                        Accessible.onPressAction: root.control.pick(choiceId, root)

                        Keys.onLeftPressed: root.focusChoice(choice.index - 1)
                        Keys.onRightPressed: root.focusChoice(choice.index + 1)
                        Keys.onPressed: event => {
                            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                                root.control.pick(choiceId, root);
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Down && choiceId === "paused") {
                                root.openDurations(choice);
                                event.accepted = true;
                            }
                        }

                        onClicked: {
                            choice.forceActiveFocus(Qt.MouseFocusReason);
                            root.control.pick(choiceId, root);
                        }
                        onArrowClicked: root.openDurations(choice)
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.control.displayText
                textFormat: Text.PlainText
                font: root.style.bodyFont
                color: root.control.hinting ? root.style.accentInk : root.style.ink
                // The sentence keeps its words: it wraps once when the tile is narrow.
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                visible: text.length > 0
                text: root.control.note
                textFormat: Text.PlainText
                font: root.style.captionFont
                color: root.style.inkSoft
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
