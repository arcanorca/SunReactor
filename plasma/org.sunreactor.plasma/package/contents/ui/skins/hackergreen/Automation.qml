/*
 * hackergreen automation: the mode as bracket buttons. The lit one is inverse
 * video, in phosphor for automatic and amber (warning) for manual and paused.
 *
 *   [ MODE ]                                            [ OK ]
 *   › [ AUTO ]  [ MAN ]  [ PAUSE ▾ ]
 *   Following the sun and the weather · Next 4:20 PM · Set 50% → 23%
 *   ← → choose · ⏎ select · ↓ durations                   (while focused)
 *
 * One focus stop: Left and Right move the cursor, Enter or Space picks the
 * button under it, Down opens the pause durations on PAUSE. AUTO is the way back
 * and stays bright while the daemon is not automatic. The rules are in
 * modules/ModeControl.qml; this file only draws them.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../terminal"
import "Hg.js" as Hg

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

    /*! The button the keys act on: 0 automatic, 1 manual, 2 paused. */
    property int cursor: 0

    readonly property int gap: 2
    /*! Short labels for the buttons; the choice ids come from ModeControl. */
    readonly property var shortLabels: ({
        automatic: i18ndc(domain, "Short mode button: follow the sun again", "AUTO"),
        manual: i18ndc(domain, "Short mode button: set brightness by hand", "MAN"),
        paused: i18ndc(domain, "Short mode button: stop adjusting for a while", "PAUSE")
    })
    readonly property var labels: control.choices.map(choice => "[ " + shortLabels[choice.id] + " ]")
    /*! Column of each button's bracket; the arrow of PAUSE follows its label. */
    readonly property var starts: {
        const out = [];
        let at = 0;
        for (let i = 0; i < labels.length; ++i) {
            out.push(at);
            at += term.len(labels[i]) + gap + (control.choices[i].id === "paused" ? 2 : 0);
        }
        return out;
    }

    activeFocusOnTab: true
    implicitHeight: section.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(domain, "Accessible name of the automation mode switch", "Automation mode")

    TermMetrics {
        id: term

        theme: root.theme
    }
    HgCrt {
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
            root.openDurations(buttons.itemAt(2));
            event.accepted = true;
        }
    }

    readonly property var modeFlag: {
        const t = root.theme;
        switch (root.client.mode) {
        case SunReactorClient.Automatic:
        case SunReactorClient.IdleDimmed:
            return Hg.flag(t, "ok", i18ndc(root.domain, "Daemon state in the status line", "OK"));
        case SunReactorClient.Offline:
            return Hg.flag(t, "err", i18ndc(root.domain, "Daemon state in the status line", "OFFLINE"));
        case SunReactorClient.Paused:
            return Hg.flag(t, "warn", i18ndc(root.domain, "Daemon state in the status line", "PAUSED"));
        default:
            return Hg.flag(t, "warn", i18ndc(root.domain, "Daemon state in the status line", "MANUAL"));
        }
    }

    readonly property int innerCols: term.cols(root.width)

    HgSection {
        id: section

        tm: term
        theme: root.theme
        width: root.width
        title: i18ndc(root.domain, "Section title of the automation mode", "MODE")
        bracket: true
        metaRuns: [root.modeFlag]
        bodyHeight: lines.height

        Column {
            id: lines

            width: root.width

            // The buttons: a caret on the focused one, the lit one inverse.
            Item {
                width: parent.width
                height: term.ch

                Repeater {
                    id: buttons

                    model: root.control.choices

                    delegate: Item {
                        id: button

                        required property var modelData
                        required property int index

                        readonly property string choiceId: modelData.id
                        readonly property bool lit: root.control.choice === choiceId
                        readonly property bool call: choiceId === "automatic" && root.control.choice !== ""
                            && root.control.choice !== "automatic"
                        readonly property bool here: root.activeFocus && root.cursor === index
                        readonly property color tone: choiceId === "automatic" ? root.theme.success : root.theme.warning

                        x: term.cx(2 + root.starts[index])
                        y: 0
                        width: term.cx(term.len(root.labels[index]) + (choiceId === "paused" ? 2 : 0))
                        height: term.ch

                        Accessible.role: Accessible.RadioButton
                        Accessible.name: modelData.label
                        Accessible.description: modelData.tip
                        Accessible.checkable: true
                        Accessible.checked: lit
                        Accessible.onPressAction: root.pick(index)

                        TermText {
                            tm: term
                            x: term.cx(-2)
                            visible: button.here
                            value: "›"
                            strong: true
                            color: root.theme.accent
                        }

                        TermText {
                            tm: term
                            value: root.labels[button.index]
                            strong: button.lit || button.call || button.here
                            color: button.lit ? root.theme.onColor(button.tone)
                                : (button.call ? root.theme.accent : (button.here ? root.theme.fg : root.theme.textMuted))
                            inverse: button.lit
                            inverseColor: button.tone
                        }

                        TermText {
                            visible: button.choiceId === "paused"
                            tm: term
                            x: term.cx(term.len(root.labels[button.index]) + 1)
                            value: "▾"
                            color: button.here ? root.theme.fg : root.theme.textMuted
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.forceActiveFocus(Qt.MouseFocusReason);
                                root.pick(button.index);
                            }
                        }

                        // The arrow has its own hit area: a click there opens the durations.
                        MouseArea {
                            visible: button.choiceId === "paused"
                            x: term.cx(term.len(root.labels[button.index]))
                            width: term.cx(2)
                            height: term.ch
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.forceActiveFocus(Qt.MouseFocusReason);
                                root.openDurations(button);
                            }
                        }
                    }
                }
            }

            // What the daemon is doing, and until when; wrapped at " · ".
            Repeater {
                model: root.wrapParts(root.control.displayText, root.innerCols - 2)

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
