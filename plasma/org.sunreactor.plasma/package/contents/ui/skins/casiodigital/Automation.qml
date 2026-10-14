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
    readonly property bool available: client.isConnected

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    function focusKey(index) {
        const count = control.order.length;
        const key = cells.itemAt((index + count) % count);
        if (key) {
            key.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    function openDurations(key) {
        durations.x = Math.max(0, key.mapToItem(root, 0, 0).x);
        durations.y = key.mapToItem(root, 0, key.height).y;
        durations.open();
    }

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18ndc(root.control.domain, "Automation mode, printed on the case", "Mode")
        trailing: root.control.note
        footerShown: true

        Text {
            Layout.fillWidth: true
            text: root.control.displayText
            textFormat: Text.PlainText
            font: root.style.body(root.style.sizeBody)
            color: root.style.ink
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
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
                onArrowClicked: root.openDurations(key)

                Keys.onLeftPressed: root.focusKey(index - 1)
                Keys.onRightPressed: root.focusKey(index + 1)
                Keys.onDownPressed: {
                    if (modelData.id === "paused") {
                        root.openDurations(key);
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
