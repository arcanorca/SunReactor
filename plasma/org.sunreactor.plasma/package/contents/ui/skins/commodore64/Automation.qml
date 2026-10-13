/*
 * Commodore 64 Automation: the mode as three reverse-video function-key
 * keycaps, F1 AUTO, F3 MAN, F5 PAUSE (F1 / F3 / F5 press them from anywhere in
 * the popup). The daemon's mode is the white keycap; Down on PAUSE opens the
 * durations. Then the status line, wrapped over two lines when it must, and the
 * READY. prompt that closes the brightness page. The rules are in
 * modules/ModeControl.qml. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected

    C64Style {
        id: style

        theme: root.theme
    }

    /*! The function-key captions, as on the C64 keyboard. */
    readonly property var captions: ({
        automatic: "F1 " + i18ndc(control.domain, "Mode segment, compact: follow the sun again", "AUTO"),
        manual: "F3 " + i18ndc(control.domain, "Mode segment, compact: set brightness by hand", "MAN"),
        paused: "F5 " + i18ndc(control.domain, "Mode segment, compact: stop adjusting for a while", "PAUSE"),
    })
    /*! The captions by choice id, shortened to fit the row (one cell between keys). */
    function keyTexts(cols) {
        const ids = ["automatic", "manual", "paused"];
        const fitted = Petscii.fitRow(ids.map(id => root.captions[id]), cols, 1);
        const out = {};
        ids.forEach((id, i) => { out[id] = fitted[i]; });
        return out;
    }

    /*! The status sentence and the idle note, joined: one line that may wrap. */
    readonly property string statusLine: root.control.hinting || root.control.note.length === 0
        ? root.control.displayText
        : root.control.displayText + " · " + root.control.note

    function keyFor(id) {
        for (let i = 0; i < cells.count; ++i) {
            const key = cells.itemAt(i);
            if (key && key.choiceId === id) {
                return key;
            }
        }
        return null;
    }

    function pressKey(id) {
        const key = keyFor(id);
        root.control.pick(id, key ? key : root);
    }

    function focusCell(index) {
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

    Shortcut {
        sequence: "F1"
        enabled: root.visible
        onActivated: root.pressKey("automatic")
    }
    Shortcut {
        sequence: "F3"
        enabled: root.visible
        onActivated: root.pressKey("manual")
    }
    Shortcut {
        sequence: "F5"
        enabled: root.visible
        onActivated: root.pressKey("paused")
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode switch", "Automation mode")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + style.padTop + style.padBottom
        regionName: "c64-automation"

        readonly property int cols: style.cols(artWidth)
        readonly property var keyTexts: root.keyTexts(cols)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.padTop
            width: surface.cols * style.cw

            C64Text {
                ink: style.ink
                text: "LOAD \"MODE\",8,1"
                maximumArtWidth: body.width
            }

            Row {
                id: cells

                spacing: style.cw

                Repeater {
                    model: root.control.choices

                    delegate: C64Keycap {
                        id: key

                        required property var modelData
                        required property int index

                        readonly property string choiceId: modelData.id

                        text: surface.keyTexts[key.choiceId] || modelData.label
                        lit: root.control.choice === key.choiceId
                        fill: lit ? style.hot : style.ink
                        ink: style.screen

                        Accessible.role: Accessible.RadioButton
                        Accessible.name: modelData.label
                        Accessible.description: modelData.tip

                        Keys.onLeftPressed: root.focusCell(key.index - 1)
                        Keys.onRightPressed: root.focusCell(key.index + 1)
                        Keys.onDownPressed: {
                            if (key.choiceId === "paused") {
                                root.openDurations(key);
                            }
                        }

                        onActivated: root.control.pick(key.choiceId, key)
                    }
                }
            }

            C64Text {
                ink: root.control.hinting ? style.hot : style.ink
                text: root.statusLine
                maximumArtWidth: body.width
                maximumLines: 3
            }

            C64Text {
                ink: style.ink
                text: "READY. █"
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
