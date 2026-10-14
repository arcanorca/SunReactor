/*
 * Automation, Nothing: a card with the mode as three round pills, then what the
 * daemon is doing, in one small grey sentence.
 *
 *   ┌──────────────────────────────────────── card ─┐
 *   │ AUTOMATION                                    │
 *   │ (AUTOMATIC)  (MANUAL)  (PAUSED ⌄)             │
 *   │ following the sun and the weather · next …    │
 *   └───────────────────────────────────────────────┘
 *
 * The filled pill is the state. While the daemon is not automatic, AUTOMATIC
 * carries the one red lamp: the way back. Left and Right move between pills,
 * Space or Enter picks one, Down (or the caret) opens the pause durations. The
 * rules are in modules/ModeControl.qml; this file only draws them.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

Rectangle {
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
    /*! The sentence with its note, as the skin sets it (lower case). */
    readonly property string fullSentence: (root.control.displayText
        + (root.control.note.length > 0 ? " · " + root.control.note : "")).toLowerCase()
    /*! The sentence from all of its " · " segments down to the first: the note
        goes first, then the least important segment, never a cut word. */
    readonly property var sentenceChoices: {
        const parts = root.fullSentence.split(" · ");
        const out = [];
        for (let n = parts.length; n >= 1; --n) {
            out.push(parts.slice(0, n).join(" · "));
        }
        return out;
    }
    /*! One measuring item per choice. Repeater.itemAt() only returns Items, so the
        TextMetrics lives inside an Item and its width is read through that item. */
    Repeater {
        id: sentenceWidths

        model: root.sentenceChoices

        Item {
            required property string modelData

            readonly property real advanceWidth: metrics.advanceWidth

            TextMetrics {
                id: metrics

                font: root.style.sentence
                text: modelData
            }
        }
    }
    /*! The longest choice that fits `limit` px. */
    function sentenceFor(limit: real): string {
        if (sentenceWidths.count < sentenceChoices.length) {
            return sentenceChoices[sentenceChoices.length - 1];
        }
        for (let i = 0; i < sentenceChoices.length; ++i) {
            const probe = sentenceWidths.itemAt(i);
            if (probe && probe.advanceWidth <= limit) { // qmllint disable missing-property
                return sentenceChoices[i];
            }
        }
        return sentenceChoices[sentenceChoices.length - 1];
    }
    readonly property bool available: client.isConnected

    implicitHeight: content.implicitHeight + 2 * style.pad
    radius: style.radius
    antialiasing: true
    color: style.surface
    border.width: style.hairline
    border.color: style.outline

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

    ColumnLayout {
        id: content

        x: root.style.pad
        y: root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap

        Text {
            Layout.fillWidth: true
            text: i18ndc(root.control.domain, "Section label of the automation card", "Automation")
            textFormat: Text.PlainText
            font: root.style.label
            color: root.style.muted
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap * 0.5

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
            id: sentence

            // Wrapped text reports its one-line width as implicit: a preferred width of 1 keeps the column narrow.
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            // The note goes when the sentence with it does not fit the line: the
            // state is kept whole, never cut with an ellipsis.
            text: root.sentenceFor(sentence.width)
            textFormat: Text.PlainText
            font: root.style.sentence
            color: root.style.muted
            wrapMode: Text.WordWrap
            maximumLineCount: 1
            elide: Text.ElideRight
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
