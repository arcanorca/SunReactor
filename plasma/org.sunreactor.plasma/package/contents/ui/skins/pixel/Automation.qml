/*
 * Pixel Automation: the mode as three framed cells, Automatic | Manual |
 * Paused with a pixel arrow for other durations, then what the daemon is doing
 * and until when. The lit cell is reverse video; while the daemon is not
 * automatic, AUTOMATIC gets the hot colour: the way back. The rules are in
 * modules/ModeControl.qml. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "PixelChrome.js" as Chrome

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

    PixelStyle {
        id: pstyle

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

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

    /*! The segment captions: the full names do not fit three cells in a 19 gu popup. */
    readonly property var captions: ({
        automatic: i18ndc(control.domain, "Mode segment, compact: follow the sun again", "Auto"),
        manual: i18ndc(control.domain, "Mode segment, compact: set brightness by hand", "Manual"),
        paused: i18ndc(control.domain, "Mode segment, compact: stop adjusting for a while", "Pause"),
    })

    /*! The status sentence and the idle note, joined: wrapped, never cut. */
    readonly property string statusLine: root.control.hinting || root.control.note.length === 0
        ? root.control.displayText
        : root.control.displayText + " · " + root.control.note
    /*! The sentence in the 5x7 face: the middle dot and the arrow have no glyph there,
        so they become ASCII, and the no-break spaces become plain spaces. */
    readonly property string shownLine: statusLine.replace(/·/g, "-").replace(/→/g, ">")
        .replace(/[  ]/g, " ")

    /*! Art pixels of one cell, its frame, and the pixel arrow when it has one. */
    function chromeOf(cell) {
        const out = [];
        const W = cell.width;
        const H = cell.height;
        if (cell.lit) {
            Chrome.push(out, 0, 0, W, H, cell.face);
        } else {
            Chrome.ring(out, 0, 0, W, H, cell.call ? pstyle.hot : pstyle.ink, 1);
            // The way back is drawn with a double frame.
            if (cell.call) {
                Chrome.ring(out, 1, 1, W - 2, H - 2, pstyle.hot, 1);
            }
        }
        if (cell.caret) {
            const color = cell.lit ? pstyle.inverse : (cell.call ? pstyle.hot : pstyle.ink);
            const x = cell.caretX;
            const y = Math.floor((H - 3) / 2);
            Chrome.push(out, x, y, 5, 1, color);
            Chrome.push(out, x + 1, y + 1, 3, 1, color);
            Chrome.push(out, x + 2, y + 2, 1, 1, color);
        }
        if (cell.activeFocus && !cell.lit) {
            Chrome.ring(out, -2, -2, W + 4, H + 4, pstyle.hot, 1);
        }
        return out;
    }

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "automation"

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.control.domain, "Title of the automation panel", "Mode")
            contentHeight: cellRow.height + 3 + status.height

            Row {
                id: cellRow

                x: 0
                y: 0
                spacing: 4

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
                        readonly property bool caret: choiceId === "paused"
                        readonly property int padX: 4
                        readonly property int caretX: padX + label.artWidth + 3
                        readonly property color face: choiceId === "automatic" ? pstyle.ink : pstyle.hot
                        readonly property color ink: lit ? pstyle.inverse : (call ? pstyle.hot : pstyle.ink)

                        width: label.artWidth + 2 * padX + (caret ? 3 + 5 : 0)
                        height: label.capHeight + 4
                        activeFocusOnTab: true

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
                            } else if (event.key === Qt.Key_Down && caret) {
                                root.openDurations(cell);
                                event.accepted = true;
                            }
                        }

                        Repeater {
                            model: root.chromeOf(cell)

                            delegate: Rectangle {
                                required property var modelData

                                x: modelData.x
                                y: modelData.y
                                width: modelData.w
                                height: modelData.h
                                color: modelData.c
                                antialiasing: false
                            }
                        }

                        PixelLabel {
                            id: label

                            theme: root.theme
                            x: cell.padX
                            y: Math.floor((cell.height - capHeight) / 2)
                            text: root.captions[cell.choiceId] || cell.modelData.label
                            face: pstyle.bodyFace
                            capitals: true
                            color: cell.ink
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                cell.forceActiveFocus(Qt.MouseFocusReason);
                                root.control.pick(cell.choiceId, root);
                            }
                        }

                        // The arrow has its own hit area: a click there opens the durations.
                        MouseArea {
                            visible: cell.caret
                            x: cell.caretX - 2
                            width: 9
                            height: parent.height
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                cell.forceActiveFocus(Qt.MouseFocusReason);
                                root.openDurations(cell);
                            }
                        }
                    }
                }
            }

            // What the daemon is doing, and until when, wrapped over up to three lines
            // in the body face; a dimmed-while-idle note follows it.
            PixelLabel {
                id: status

                theme: root.theme
                x: 0
                y: cellRow.height + 3
                text: root.shownLine
                face: pstyle.bodyFace
                color: root.control.hinting ? pstyle.hot : pstyle.ink
                maximumArtWidth: panel.contentWidth
                // Two lines at most: a third line re-entered a layout loop on the handheld
                // theme (main thread at 100%, see tools/nest/last_sweep.md).
                maximumLines: 2
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
