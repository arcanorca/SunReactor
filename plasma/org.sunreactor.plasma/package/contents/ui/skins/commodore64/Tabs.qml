/*
 * Tab bar, Commodore 64 skin: one reverse-video keycap per page, labelled with
 * its function key (F1, F3, ...). The page on screen is the white keycap.
 * Click, Space, Enter and Tab / arrows as on any button. The weather tab's
 * temperature rides in its label (hint). Module contract: components/tabs/.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    C64Style {
        id: style

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.PageTabList

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: style.margin + style.ch + style.margin / 2
        regionName: "c64-tabs"

        Row {
            id: keys

            x: style.left(surface.artWidth)
            y: style.margin
            spacing: style.cw

            Repeater {
                model: root.tabs

                delegate: C64Keycap {
                    id: key

                    required property var modelData
                    required property int index

                    text: "F" + (1 + 2 * key.index) + " " + modelData.text
                        + (modelData.hint ? " " + modelData.hint : "")
                    lit: root.current === modelData.id
                    fill: lit ? style.hot : style.ink
                    ink: style.screen

                    Accessible.role: Accessible.PageTab
                    Accessible.name: modelData.text

                    onActivated: root.picked(modelData.id)
                }
            }
        }
    }
}
