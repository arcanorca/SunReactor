/*
 * Terminal skin of the automation module: the mode as bracketed cells, in the
 * same box as the TUI's automation panel.
 *
 *     ╭ Automation ─────────────────────── ● Live ╮
 *     │ ❯[ Automatic ]  [ Manual ]  [ Paused ] ▾  │   the lit cell is inverse
 *     │ Following the sun and the weather · Next  │
 *     │ 4:20 PM · Set 50% → 23%                   │
 *     │ ← → choose · ⏎ select · ↓ durations       │   while focused
 *     ╰───────────────────────────────────────────╯
 *
 * One focus stop: Left and Right move the cursor (❯), Enter or Space picks the
 * cell under it, Down opens the pause durations on Paused. Automatic is in the
 * accent colour while the daemon is not automatic: the way back. The rules are
 * in modules/ModeControl.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

FocusScope {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected

    /*! The cell the keys act on: 0 automatic, 1 manual, 2 paused. */
    property int cursor: 0

    readonly property int gap: 2
    readonly property var labels: control.choices.map(choice => "[ " + choice.label + " ]")
    /*! Column of each cell's marker; the label follows one cell later. */
    readonly property var starts: {
        const out = [];
        let at = 2;
        for (let i = 0; i < labels.length; ++i) {
            out.push(at);
            at += 1 + term.len(labels[i]) + gap;
        }
        return out;
    }

    activeFocusOnTab: true
    implicitHeight: frame.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(domain, "Accessible name of the automation mode switch", "Automation mode")

    TermMetrics {
        id: term

        theme: root.theme
    }

    function pick(index) {
        root.cursor = index;
        root.control.pick(root.control.order[index], root);
    }

    function openDurations(cell) {
        durations.x = Math.max(0, cell.mapToItem(root, 0, 0).x);
        durations.y = cell.mapToItem(root, 0, cell.height).y;
        durations.open();
    }

    /*! Splits the sentence at its " · " separators into lines of at most `width` cells. */
    function wrapParts(text, width) {
        const out = [];
        let line = "";
        for (const part of text.split(" · ")) {
            const joined = line.length > 0 ? line + " · " + part : part;
            if (line.length > 0 && term.len(joined) > width) {
                out.push(line);
                line = part;
            } else {
                line = joined;
            }
        }
        if (line.length > 0) {
            out.push(line);
        }
        return out;
    }

    Keys.onLeftPressed: root.cursor = (root.cursor + root.labels.length - 1) % root.labels.length
    Keys.onRightPressed: root.cursor = (root.cursor + 1) % root.labels.length
    Keys.onPressed: event => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.pick(root.cursor);
            event.accepted = true;
        } else if (event.key === Qt.Key_Down && root.cursor === 2) {
            root.openDurations(cells.itemAt(2));
            event.accepted = true;
        }
    }

    readonly property var modeRuns: {
        const t = root.theme;
        switch (root.client.mode) {
        case SunReactorClient.Automatic:
        case SunReactorClient.IdleDimmed:
            return [{ t: "● ", c: t.success }, { t: i18ndc(root.domain, "Daemon state in the status line", "Live"), c: t.textMuted }];
        case SunReactorClient.Offline:
            return [{ t: "○ ", c: t.error }, { t: i18ndc(root.domain, "Daemon state in the status line", "Offline"), c: t.textMuted }];
        default:
            return [{ t: "▲ ", c: t.warning }, { t: i18ndc(root.domain, "Daemon state in the status line", "Attention"), c: t.textMuted }];
        }
    }

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        width: root.width
        focused: root.activeFocus
        regionName: "automation"
        title: i18nd(root.domain, "Automation")
        rightRuns: root.modeRuns
        contentRows: Math.max(1, lines.rows)

        Column {
            id: lines

            readonly property int rows: Math.round(height * term.dpr / term.chDev)

            width: term.cx(frame.innerCols)

            // The choices.
            Item {
                width: parent.width
                height: term.ch

                Repeater {
                    id: cells

                    model: root.control.choices

                    delegate: Item {
                        id: cell

                        required property var modelData
                        required property int index

                        readonly property string choiceId: modelData.id
                        readonly property bool lit: root.control.choice === choiceId
                        readonly property bool call: choiceId === "automatic" && root.control.choice !== ""
                            && root.control.choice !== "automatic"
                        readonly property bool here: root.activeFocus && root.cursor === index
                        readonly property color tone: choiceId === "automatic" ? root.theme.success : root.theme.warning
                        readonly property int caretCells: choiceId === "paused" ? 2 : 0

                        x: term.cx(root.starts[index])
                        y: 0
                        width: term.cx(1 + term.len(root.labels[index]) + caretCells)
                        height: term.ch

                        Accessible.role: Accessible.RadioButton
                        Accessible.name: modelData.label
                        Accessible.description: modelData.tip
                        Accessible.checkable: true
                        Accessible.checked: lit
                        Accessible.onPressAction: root.pick(index)

                        TermText {
                            tm: term
                            value: cell.here ? "❯" : " "
                            strong: true
                            color: root.theme.accent
                        }

                        TermText {
                            tm: term
                            x: term.cx(1)
                            value: root.labels[cell.index]
                            strong: cell.lit || cell.call || cell.here
                            color: cell.lit ? root.theme.onColor(cell.tone)
                                : (cell.call ? root.theme.accent : (cell.here ? root.theme.fg : root.theme.textMuted))
                            inverse: cell.lit
                            inverseColor: cell.tone
                        }

                        TermText {
                            visible: cell.choiceId === "paused"
                            tm: term
                            x: term.cx(1 + term.len(root.labels[cell.index]) + 1)
                            value: "▾"
                            color: cell.here ? root.theme.fg : root.theme.textMuted
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.forceActiveFocus(Qt.MouseFocusReason);
                                root.pick(cell.index);
                            }
                        }

                        // The arrow has its own hit area: a click there opens the durations.
                        MouseArea {
                            visible: cell.choiceId === "paused"
                            x: term.cx(1 + term.len(root.labels[cell.index]))
                            width: term.cx(2)
                            height: term.ch
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.forceActiveFocus(Qt.MouseFocusReason);
                                root.openDurations(cell);
                            }
                        }
                    }
                }
            }

            // What the daemon is doing, and until when; wrapped at " · ".
            Repeater {
                model: root.wrapParts(root.control.displayText, frame.innerCols - 3)

                delegate: Item {
                    id: line

                    required property string modelData

                    width: lines.width
                    height: term.ch

                    TermText {
                        tm: term
                        x: term.cx(2)
                        value: line.modelData
                        color: root.control.hinting ? root.theme.warning : root.theme.fg
                    }
                }
            }

            Item {
                visible: root.control.note.length > 0
                width: lines.width
                height: term.ch

                TermText {
                    tm: term
                    x: term.cx(2)
                    value: root.control.note
                    color: root.theme.textMuted
                }
            }

            Item {
                visible: root.activeFocus
                width: lines.width
                height: term.ch

                TermText {
                    tm: term
                    x: term.cx(2)
                    value: i18ndc(root.domain, "Key hint for the automation mode switch",
                                  "← → choose · ⏎ select · ↓ durations")
                    color: root.theme.textMuted
                }
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
