/*
 * Xbox 360 Automation: three glossy blade mode pills and daemon status text.
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

    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected || client.statusStale

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

    XboxCard {
        id: card

        style: root.style
        Layout.fillWidth: true
        implicitHeight: body.implicitHeight + 2 * card.pad

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

                    delegate: XboxPill {
                        id: choice

                        required property var modelData
                        required property int index

                        readonly property string choiceId: modelData.id

                        Layout.fillWidth: true
                        Layout.preferredHeight: root.style.snap(root.style.unit * 1.85)
                        style: root.style
                        label: modelData.label
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
                                durations.openBelow(choice, root);
                                event.accepted = true;
                            }
                        }

                        onClicked: {
                            choice.forceActiveFocus(Qt.MouseFocusReason);
                            root.control.pick(choiceId, root);
                        }
                        onArrowClicked: durations.openBelow(choice, root)
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: root.style.line
                color: root.style.alpha(root.style.steel, 0.42)
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: root.style.gap * 0.4

                Text {
                    Layout.fillWidth: true
                    text: root.control.primaryText
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.control.hinting ? root.style.greenBright : root.style.ink
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.control.scheduleText
                    textFormat: Text.PlainText
                    font: root.style.captionFont
                    color: root.style.muted
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.control.note
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.greenBright
                }
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
