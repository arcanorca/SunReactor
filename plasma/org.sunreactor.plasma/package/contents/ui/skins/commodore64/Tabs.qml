/*
 * Tab bar, Commodore 64 skin: one reverse-video keycap per page, labelled with
 * its function key (F1, F3, ...). The page on screen is the white keycap.
 * Click, Space, Enter and Tab / arrows as on any button. The weather tab's
 * temperature rides in its label (hint). Module contract: components/tabs/.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "Petscii.js" as Petscii

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

    /*! Key captions that fit `cols` cells: the full names with the hint, then
     *  without it, then the short names; a row that is still too wide shortens
     *  its longest caption (Petscii.fitRow). */
    readonly property var shortNames: ({ brightness: "BRIGHT", weather: "WEATHER" })
    function captions(cols) {
        const gap = 1;
        const keyed = text => root.tabs.map((tab, i) => "F" + (1 + 2 * i) + " " + text(tab));
        const variants = [
            tab => tab.text + (tab.hint ? " " + tab.hint : ""),
            tab => tab.text,
            tab => shortNames[tab.id] || tab.text,
        ];
        for (const text of variants) {
            const labels = keyed(text);
            const width = labels.reduce((sum, s) => sum + Petscii.cells(s), 0) + gap * Math.max(0, labels.length - 1);
            if (width <= cols) {
                return labels;
            }
        }
        return Petscii.fitRow(keyed(variants[2]), cols, gap);
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

        readonly property int cols: style.cols(artWidth)
        readonly property var labels: root.captions(cols)

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

                    text: surface.labels[key.index] || ""
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
