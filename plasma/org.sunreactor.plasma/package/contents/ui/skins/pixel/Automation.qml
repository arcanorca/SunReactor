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
            contentHeight: cellRow.height + 4 + statusLines.height + (note.visible ? 2 + note.height : 0)

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
                        height: label.capHeight + 6
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
                            text: cell.modelData.label
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

            // One line per " · " part: a readout list, as the pixel panels print it.
            Column {
                id: statusLines

                x: 0
                y: cellRow.height + 4
                spacing: 2

                Repeater {
                    model: root.control.displayText.split(" · ")

                    delegate: PixelLabel {
                        required property string modelData

                        theme: root.theme
                        text: modelData
                        // The bitmap face for every line: the small face has no colon,
                        // so lines with a time would otherwise change face mid-panel.
                        face: "bitmap"
                        color: root.control.hinting ? pstyle.hot : pstyle.ink
                        maximumArtWidth: panel.contentWidth
                    }
                }
            }

            PixelLabel {
                id: note

                visible: text.length > 0
                theme: root.theme
                x: 0
                y: statusLines.y + statusLines.height + 2
                text: root.control.note
                face: pstyle.bodyFace
                color: pstyle.dim
                stipple: pstyle.stippleDim
                maximumArtWidth: panel.contentWidth
                maximumLines: 1
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
