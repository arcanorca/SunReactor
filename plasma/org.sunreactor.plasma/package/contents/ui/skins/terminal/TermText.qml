/*
 * One run of text on the cell grid. Every glyph advances exactly one cell
 * (TermMetrics.letterSpacing), the run is `len(text)` cells wide and one cell
 * high, so `x: tm.cx(col)` places it on a column and runs placed next to each
 * other never drift. `cells` cuts the run with "…" (the TUI's truncate);
 * `inverse` draws it in video inverse, like the TUI's selection capsule.
 */

pragma ComponentBehavior: Bound

import QtQuick

Text {
    id: root

    required property TermMetrics tm
    /*! The string to draw; `text` is derived from it. */
    property string value: ""
    /*! Maximum width in cells, -1 = unlimited. */
    property int cells: -1
    property bool strong: false
    /*! Video inverse: `inverseColor` behind, the text in `color`. */
    property bool inverse: false
    property color inverseColor: "transparent"
    /*! Extra cells of inverse background on each side (capsule padding). */
    property int pad: 0

    /*! Width of the drawn run in cells. */
    readonly property int usedCells: tm.len(text)

    text: cells >= 0 ? tm.fit(value, cells) : value
    textFormat: Text.PlainText
    wrapMode: Text.NoWrap
    width: tm.cx(usedCells)
    height: tm.ch
    topPadding: tm.textTop
    verticalAlignment: Text.AlignTop

    font.family: tm.family
    font.pixelSize: tm.pixelSize
    font.letterSpacing: tm.letterSpacing
    font.bold: strong
    font.kerning: false
    // Terminals draw every character on its own; no programming ligatures.
    font.features: ({ "calt": 0, "liga": 0, "clig": 0, "dlig": 0 })

    Rectangle {
        z: -1
        visible: root.inverse
        x: -root.tm.cx(root.pad)
        width: root.tm.cx(root.usedCells + 2 * root.pad)
        height: root.tm.ch
        color: root.inverseColor
        antialiasing: false
    }
}
