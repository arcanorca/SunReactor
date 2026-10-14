/*
 * Terminal skin of the automation module: the mode as bracketed cells, in the
 * same box as the TUI's automation panel.
 *
 *     ╭ Automation ─────────────────────── ● Live ╮
 *     │ ❯[ Automatic ]  [ Manual ]  [ Paused ] ▾  │   the lit cell is inverse
 *     │ Following the sun and the weather · Ne…  │   one row, the rest elides
 *     │ ← → choose · ⏎ select · ↓ durations       │   while focused
 *     ╰───────────────────────────────────────────╯
 *
 * One focus stop: Left and Right move the cursor (❯), Enter or Space picks the
 * cell under it, Down opens the pause durations on Paused. Automatic is in the
 * accent colour while the daemon is not automatic: the way back. The rules are
 * in modules/ModeControl.qml. On a narrow popup the row tightens (`forms`):
 * the spacing goes, then the spaces inside the brackets.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Term.js" as Term

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

    /*! The row's forms, richest first: cells inside the brackets, cells between
        the choices, cells before the arrow. The first one that fits the frame is
        used, so a narrow popup or a longer translation tightens the row instead
        of running past the frame. */
    readonly property var forms: [
        { pad: 1, gap: 2, arrow: 1 },
        { pad: 1, gap: 1, arrow: 0 },
        { pad: 0, gap: 2, arrow: 1 },
        { pad: 0, gap: 1, arrow: 0 },
    ]

    function labelIn(form, index): string {
        const text = control.choices[index].label;
        return form.pad > 0 ? "[ " + text + " ]" : "[" + text + "]";
    }

    /*! Each cell's marker column and the row's width in cells, for one form. */
    function arrange(form): var {
        const starts = [];
        let at = 2;
        let end = 2;
        for (let i = 0; i < control.choices.length; ++i) {
            const arrow = control.choices[i].id === "paused" ? form.arrow + 1 : 0;
            const width = 1 + term.len(labelIn(form, i)) + arrow;
            starts.push(at);
            end = at + width;
            at += width + form.gap;
        }
        return { form: form, starts: starts, width: end };
    }

    /*! The first form that fits the frame's cells; the tightest when none does. */
    readonly property var arrangement: {
        for (const form of forms) {
            const found = arrange(form);
            if (found.width <= frame.innerCols) {
                return found;
            }
        }
        return arrange(forms[forms.length - 1]);
    }
    readonly property var labels: control.choices.map((choice, i) => labelIn(arrangement.form, i))
    /*! Column of each cell's marker; the label follows one cell later. */
    readonly property var starts: arrangement.starts

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

    /*! The status sentence and the idle note, one row: the hint replaces both. */
    readonly property string statusLine: root.control.hinting || root.control.note.length === 0
        ? root.control.displayText
        : root.control.displayText + " · " + root.control.note

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
                        readonly property int caretCells: choiceId === "paused" ? root.arrangement.form.arrow + 1 : 0

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
                            x: term.cx(1 + term.len(root.labels[cell.index]) + root.arrangement.form.arrow)
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
                            width: term.cx(cell.caretCells)
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

            // What the daemon is doing, and until when: one row, the rest elides.
            Item {
                width: lines.width
                height: term.ch

                TermText {
                    tm: term
                    x: term.cx(2)
                    cells: Math.max(0, frame.innerCols - 2)
                    value: Term.shedSegments(root.statusLine, Math.max(0, frame.innerCols - 2))
                    color: root.control.hinting ? root.theme.warning : root.theme.fg
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
