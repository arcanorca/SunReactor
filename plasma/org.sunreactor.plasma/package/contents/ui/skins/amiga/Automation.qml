/*
 * Amiga Automation: a Workbench window "Mode" with three gadgets - AUTO | MANUAL
 * | PAUSE with a drop-down triangle for other durations - over the status
 * sentence, wrapped to up to three lines rather than cut. The chosen mode is the orange
 * gadget. The rules are in modules/ModeControl.qml. Module contract:
 * modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

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

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.control.domain, "Accessible name of the automation mode switch", "Automation mode")

    /*! The gadgets take their usual 2 px inside each outline when the three fit the window
        with one px between them; the window can be a grid unit narrower than that (the page
        keeps a grid unit for its scrollbar), and then the inner gap closes to 1 px. Reads
        only label and caret widths, which do not depend on the gap. */
    readonly property bool gadgetsTight: {
        let need = Math.max(0, cells.count - 1);
        for (let i = 0; i < cells.count; ++i) {
            need += cells.itemAt(i).fixedArt + 2 * 2; // qmllint disable missing-property
        }
        return need > panel.contentWidth;
    }

    function focusCell(index) {
        const count = root.control.order.length;
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

    /*! The gadget captions: the full names do not fit three gadgets in a 19 gu popup. */
    readonly property var captions: ({
        automatic: i18ndc(root.control.domain, "Mode segment, compact: follow the sun again", "Auto"),
        manual: i18ndc(root.control.domain, "Mode segment, compact: set brightness by hand", "Manual"),
        paused: i18ndc(root.control.domain, "Mode segment, compact: stop adjusting for a while", "Pause"),
    })

    /*! The status sentence and the idle note, joined. Automatic shows only its next step
        ("Next 4:20 PM · Set 50% -> 23%", two lines): the full "Following the sun and the
        weather" sentence needs four lines and would be cut. The AUTO gadget says the mode. */
    readonly property string statusLine: {
        const automatic = root.control.choice === "automatic" && !root.control.hinting
            && root.control.nextText.length > 0;
        const base = automatic ? root.control.nextText : root.control.displayText;
        return root.control.note.length === 0 ? base : base + " · " + root.control.note;
    }

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "automation"

        AmigaWindow {
            id: panel

            theme: root.theme
            style: tokens
            width: surface.artWidth
            title: i18ndc(root.control.domain, "Title of the automation window", "Mode")
            contentHeight: cellRow.height + 2 + status.height

            Row {
                id: cellRow

                spacing: 1

                Repeater {
                    id: cells

                    model: root.control.choices

                    delegate: Item {
                        id: cell

                        required property var modelData
                        required property int index

                        readonly property string choiceId: modelData.id
                        readonly property bool caret: choiceId === "paused"
                        /*! The label and the drop-down, in art px (the gadget adds its gap on each side). */
                        readonly property int fixedArt: gadget.labelArt + gadget.caretArt

                        width: gadget.width
                        height: gadget.height
                        activeFocusOnTab: true

                        AmigaGadget {
                            id: gadget

                            theme: root.theme
                            style: tokens
                            text: root.captions[cell.choiceId] || cell.modelData.label
                            gap: root.gadgetsTight ? 1 : 2
                            lit: root.control.choice === cell.choiceId
                            caret: cell.caret
                            onActivated: root.control.pick(cell.choiceId, cell)
                        }

                        // The drop-down triangle opens the other durations.
                        MouseArea {
                            visible: cell.caret
                            x: gadget.width - 9
                            width: 9
                            height: gadget.height
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                cell.forceActiveFocus(Qt.MouseFocusReason);
                                root.openDurations(cell);
                            }
                        }

                        Keys.onLeftPressed: root.focusCell(index - 1)
                        Keys.onRightPressed: root.focusCell(index + 1)
                        Keys.onDownPressed: if (cell.caret) { root.openDurations(cell); }
                    }
                }
            }

            // What the daemon is doing, and until when: wrapped, never cut.
            AmigaText {
                id: status

                x: 0
                y: cellRow.height + 2
                text: root.statusLine
                color: root.control.hinting ? tokens.hot : tokens.ink
                maximumArtWidth: panel.contentWidth
                maximumLines: 3
            }
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
