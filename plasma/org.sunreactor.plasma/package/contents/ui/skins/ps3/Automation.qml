/*
 * Automation, PlayStation 3: the mode as three XMB text items in a row,
 * Automatic / Manual / Paused. The chosen one is bright with a soft underline
 * glow; a chevron after Paused opens the pause durations. Under them, what the
 * daemon is doing and until when. The rules are in modules/ModeControl.qml,
 * shared with every skin: this file only draws them.
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

    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected

    spacing: style.gap * 0.5

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode buttons", "Automation mode")

    function focusCell(index) {
        const count = control.order.length;
        const cell = cells.itemAt(((index % count) + count) % count);
        if (cell) {
            cell.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    function openDurations(item) {
        const at = item.mapToItem(root, 0, item.height);
        durations.x = Math.max(0, at.x);
        durations.y = at.y;
        durations.open();
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap3

        Repeater {
            id: cells

            model: root.control.choices

            delegate: XmbLabel {
                id: cell

                required property var modelData
                required property int index

                Layout.fillWidth: true
                style: root.style
                text: modelData.label
                selected: root.control.choice === modelData.id
                arrow: modelData.id === "paused"
                textFont: root.style.tabFont
                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData.label
                Accessible.description: modelData.tip
                Accessible.checkable: true
                Accessible.checked: selected

                onActivated: root.control.pick(modelData.id, root)
                onMoved: delta => root.focusCell(index + delta)
                onDown: {
                    if (modelData.id === "paused") {
                        root.openDurations(cell);
                    }
                }
                onArrowActivated: root.openDurations(cell)
            }
        }
    }

    Text {
        Layout.fillWidth: true
        visible: text.length > 0
        text: root.control.displayText
        textFormat: Text.PlainText
        font: root.style.legendFont
        color: root.control.hinting ? root.style.warning : root.style.inkSoft
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }

    Text {
        Layout.fillWidth: true
        visible: text.length > 0
        text: root.control.note
        textFormat: Text.PlainText
        font: root.style.legendFont
        color: root.style.inkSoft
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
