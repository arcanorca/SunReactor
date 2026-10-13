/*
 * Nintendo DS touch screen, automation: three big touch buttons - AUTO, MANUAL,
 * PAUSE - in the console's own idiom. The chosen one is inverted (accent fill)
 * with the stylus tip on it; a press inverts the button and shifts it one pixel
 * down. The daemon's sentence sits in a speech bubble under them. The rules are
 * in modules/ModeControl.qml. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel"
import "Nds.js" as Nds

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

    /*! The button captions: the short forms fit three big touch buttons. */
    readonly property var captions: ({
        automatic: i18ndc(control.domain, "Mode segment, compact: follow the sun again", "Auto"),
        manual: i18ndc(control.domain, "Mode segment, compact: set brightness by hand", "Manual"),
        paused: i18ndc(control.domain, "Mode segment, compact: stop adjusting for a while", "Pause"),
    })

    /*! The status sentence and the idle note, joined: one line that wraps. */
    readonly property string statusLine: control.hinting || control.note.length === 0
        ? control.displayText
        : control.displayText + " · " + control.note
    /*! The sentence in the 5x7 face, which wraps: the middle dot and the arrow
        have no glyph there, so they become ASCII. */
    readonly property string shownLine: statusLine.replace(/·/g, "-").replace(/→/g, ">").replace(/[  ]/g, " ")

    NdsLid {
        id: surface

        width: parent.width
        theme: root.theme
        grid: root.theme.grid
        fillsWidth: true
        lidHeight: 21 + bubbleH + 3
        regionName: "automation"

        readonly property int bubbleH: status.artHeight + 5

        Repeater {
            model: root.control.choices

            delegate: Item {
                id: touch

                required property var modelData
                required property int index

                readonly property string choiceId: modelData.id
                readonly property bool active: root.control.choice === choiceId
                readonly property int gap: 3
                readonly property int buttonWidth: Math.floor((surface.artWidth - 8 - 2 * gap) / 3)

                // Inverted while pressed; a pressed button also sinks one pixel.
                readonly property bool inverted: active !== mouse.pressed
                readonly property color fillColor: inverted ? root.theme.accent : root.theme.bg
                readonly property color edgeColor: inverted || activeFocus ? root.theme.accent : root.theme.borderInactive
                readonly property color labelColor: inverted ? root.theme.bg : root.theme.fg

                x: 4 + index * (buttonWidth + gap)
                y: 4 + (mouse.pressed ? 1 : 0)
                width: buttonWidth
                height: 13
                activeFocusOnTab: true

                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData.label
                Accessible.description: modelData.tip
                Accessible.checkable: true
                Accessible.checked: active
                Accessible.onPressAction: root.control.pick(choiceId, touch)

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        root.control.pick(choiceId, touch);
                        event.accepted = true;
                    }
                }

                Repeater {
                    model: Nds.box([], 0, 0, touch.width, touch.height, touch.fillColor, touch.edgeColor)
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
                    theme: root.theme
                    x: Math.floor((touch.width - artWidth) / 2)
                    y: Math.floor((touch.height - capHeight) / 2)
                    face: pstyle.bodyFace
                    capitals: true
                    color: touch.labelColor
                    text: root.captions[touch.choiceId] || touch.modelData.label
                }

                // The stylus tip rests on the chosen button.
                Repeater {
                    model: touch.active ? Nds.cursor([], touch.width - 5, -2, root.theme.secondaryAccent) : []
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

                MouseArea {
                    id: mouse

                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        touch.forceActiveFocus(Qt.MouseFocusReason);
                        root.control.pick(touch.choiceId, touch);
                    }
                }
            }
        }

        // The speech bubble: the status sentence, wrapped.
        Repeater {
            model: Nds.box([], 4, 21, surface.artWidth - 8, surface.bubbleH, root.theme.bg, root.theme.borderInactive)
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
        Rectangle {
            x: 13
            y: 19
            width: 5
            height: 3
            color: root.theme.bg
            antialiasing: false
        }
        Rectangle {
            x: 13
            y: 19
            width: 1
            height: 2
            color: root.theme.borderInactive
            antialiasing: false
        }
        Rectangle {
            x: 17
            y: 19
            width: 1
            height: 2
            color: root.theme.borderInactive
            antialiasing: false
        }

        PixelLabel {
            id: status

            theme: root.theme
            x: 8
            y: 23
            face: "small"
            color: root.control.hinting ? root.theme.accent : pstyle.ink
            text: root.shownLine
            maximumArtWidth: Math.max(8, surface.artWidth - 16)
            maximumLines: 3
        }
    }
}
