/*
 * The automation controls, synthwave84, without a card: the mode as three neon
 * pills, then what the daemon is doing and until when. The lit pill (filled pink,
 * dark text) is the state; while the daemon is not automatic, AUTOMATIC carries
 * the pink outline, the way back. Drawn inside the Displays column when the theme
 * lists both modules, or in its own card (Automation.qml).
 *
 *   ( AUTOMATIC )  ( MANUAL )  ( PAUSED ▾ )
 *   Following the sun and the weather · Next 4:20 PM · Set 50% → 23%
 *
 * Left and Right move between pills, Space or Enter picks one, Down opens the
 * pause durations on PAUSED. The rules are in modules/ModeControl.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../components" as Components

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property Mod.ModeControl control: Mod.ModeControl {
        client: root.client
        settings: root.settings
    }

    spacing: style.rowGap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode switch", "Automation mode")

    function focusCell(index) {
        const count = control.order.length;
        const cell = cells.itemAt((index + count) % count);
        if (cell) {
            cell.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    function openDurations(cell) {
        durations.x = Math.max(0, cell.mapToItem(root, 0, 0).x);
        durations.y = cell.mapToItem(root, 0, cell.height).y;
        durations.open();
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        Repeater {
            id: cells

            model: root.control.choices

            delegate: Pill {
                id: cell

                required property var modelData
                required property int index

                readonly property string choiceId: modelData.id
                readonly property bool lit: root.control.choice === choiceId
                readonly property bool call: choiceId === "automatic" && root.control.choice !== ""
                    && root.control.choice !== "automatic"

                Layout.fillWidth: true
                style: root.style
                text: modelData.label
                filled: lit
                tone: lit || call ? root.style.pink : root.style.cyan
                arrow: choiceId === "paused"
                ringed: activeFocus

                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData.label
                Accessible.description: modelData.tip
                Accessible.checkable: true
                Accessible.checked: lit
                Accessible.onPressAction: root.control.pick(choiceId, root)

                Keys.onLeftPressed: root.focusCell(index - 1)
                Keys.onRightPressed: root.focusCell(index + 1)
                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        root.control.pick(choiceId, root);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Down && choiceId === "paused") {
                        root.openDurations(cell);
                        event.accepted = true;
                    }
                }

                onClicked: {
                    cell.forceActiveFocus(Qt.MouseFocusReason);
                    root.control.pick(choiceId, root);
                }
                onArrowClicked: root.openDurations(cell)
            }
        }
    }

    Text {
        Layout.fillWidth: true
        text: root.control.displayText
        textFormat: Text.PlainText
        font: root.style.bodyFont
        color: root.control.hinting ? root.style.yellow : root.style.ink
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }

    Text {
        Layout.fillWidth: true
        visible: text.length > 0
        text: root.control.note
        textFormat: Text.PlainText
        font: root.style.labelFont
        color: root.style.cyan
    }

    Components.ModeDurationMenu {
        id: durations

        control: root.control
    }
}
