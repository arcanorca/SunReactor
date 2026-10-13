/*
 * Automation, Nothing: the mode as three round pills, then what the daemon is
 * doing, in one plain lower-case line.
 *
 *   ( AUTOMATIC )  ( MANUAL )  ( PAUSED ▾ )
 *   following the sun and the weather · next 16:20 → 23%
 *
 * The filled pill is the state. While the daemon is not automatic, AUTOMATIC
 * carries the one red lamp: the way back. Left and Right move between pills,
 * Space or Enter picks one, Down (or the arrow) opens the pause durations. The
 * rules are in modules/ModeControl.qml; this file only draws them.
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

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.control.domain, "Accessible name of the automation mode switch", "Automation mode")

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

                Layout.fillWidth: true
                style: root.style
                radio: true
                text: modelData.label
                accessibleName: modelData.label
                accessibleDescription: modelData.tip
                active: root.control.choice === choiceId
                // The way back from a pause or a manual setting carries the red lamp.
                call: choiceId === "automatic" && root.control.choice !== ""
                    && root.control.choice !== "automatic"
                arrow: choiceId === "paused"

                Keys.onLeftPressed: root.focusCell(index - 1)
                Keys.onRightPressed: root.focusCell(index + 1)
                Keys.onDownPressed: {
                    if (choiceId === "paused") {
                        root.openDurations(cell);
                    }
                }

                onClicked: root.control.pick(choiceId, root)
                onArrowClicked: root.openDurations(cell)
            }
        }
    }

    Text {
        // Wrapped text reports its one-line width as implicit: a preferred width of 1 keeps the column narrow.
        Layout.fillWidth: true
        Layout.preferredWidth: 1
        Layout.topMargin: root.style.gap * 0.5
        text: (root.control.displayText + (root.control.note.length > 0 ? " · " + root.control.note : ""))
            .toLowerCase()
        textFormat: Text.PlainText
        font: root.style.plain
        color: root.style.ink
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
