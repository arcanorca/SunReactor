/*
 * Automation, casiodigital: the mode as three physical keys on the case, the way
 * a watch has its buttons, and the sentence of what the daemon is doing on the
 * glass above them.
 *
 *   MODE
 *   ┌───────────────────────────────────────────────────┐
 *   │ Following the sun and the weather · Next 4:20 PM  │
 *   └───────────────────────────────────────────────────┘
 *    ( AUTOMATIC )   ( MANUAL )   ( PAUSED  ▾ )
 *
 * The chosen key is pressed in with a backlit LED; while something else is chosen,
 * AUTOMATIC carries a ring: the way back. Left and Right move between keys, Space
 * or Enter presses one, Down (or the arrow) opens the pause durations. The rules
 * are in modules/ModeControl.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CasioStyle style: CasioStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected || client.statusStale

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    function focusKey(index) {
        const count = control.order.length;
        const key = cells.itemAt((index + count) % count);
        if (key) {
            key.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18ndc(root.control.domain, "Automation mode, printed on the case", "Mode")
        trailing: root.control.note
        footerShown: true

        ColumnLayout {
            Layout.fillWidth: true
            spacing: root.style.gap

            Text {
                Layout.fillWidth: true
                text: root.control.primaryText
                textFormat: Text.PlainText
                font: root.style.cap(Math.max(10, Math.round(root.style.unit * 0.62)))
                color: root.control.hinting ? root.style.backlight : root.style.ink
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }

            Rectangle {
                Layout.fillWidth: true
                visible: root.control.scheduleText.length > 0
                Layout.preferredHeight: root.style.hairline
                color: root.style.ghost
            }

            Text {
                Layout.fillWidth: true
                visible: text.length > 0
                text: root.control.scheduleText
                textFormat: Text.PlainText
                font: root.style.data(Math.max(10, Math.round(root.style.unit * 0.64)))
                color: root.style.inkMuted
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
        }

        footer: Repeater {
            id: cells

            model: root.control.choices

            delegate: CasioKey {
                id: key

                required property var modelData
                required property int index

                Layout.fillWidth: true
                style: root.style
                label: modelData.label
                lit: root.control.choice === modelData.id
                call: modelData.id === "automatic" && root.control.choice !== ""
                    && root.control.choice !== "automatic"
                arrow: modelData.id === "paused"
                interactive: root.control.choice !== ""
                Accessible.description: modelData.tip

                onClicked: root.control.pick(modelData.id, root)
                onArrowClicked: durations.openBelow(key, root)

                Keys.onLeftPressed: root.focusKey(index - 1)
                Keys.onRightPressed: root.focusKey(index + 1)
                Keys.onDownPressed: {
                    if (modelData.id === "paused") {
                        durations.openBelow(key, root);
                    }
                }
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
