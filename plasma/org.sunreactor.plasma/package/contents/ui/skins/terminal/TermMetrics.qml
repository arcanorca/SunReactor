/*
 * The character-cell grid of the terminal skins.
 *
 * A cell is `cw` x `ch` logical px, and both are whole device pixels at the
 * window's effective device pixel ratio (theme.grid.devicePixelRatio, which is
 * the fractional value on Wayland). The cell width comes from the font's real
 * advance, rounded to device pixels; every text run gets `letterSpacing` =
 * cw - advance, so glyph n of a run starts exactly at n * cw and text columns
 * line up with the rectangles that draw frames, gauges and block fills.
 *
 * Box-drawing lines and block elements are never drawn with glyphs (glyph
 * joins leave gaps at fractional font sizes); they are rectangles on the cell
 * grid: `lineX(col)` / `lineY(row)` give the device-aligned centre line of a
 * cell, `lw` the stroke (one device pixel up to DPR ~1.9, then two).
 *
 * Font: JetBrains Mono (bundled, OFL), regular + bold, ligatures off. It
 * covers the TUI's status glyphs, so a run never mixes in a fallback font
 * with another advance.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import org.kde.kirigami as Kirigami

import "../../engine"
import "Term.js" as Term

Item {
    id: root

    required property Theme theme

    visible: false
    width: 0
    height: 0

    FontLoader {
        id: regular
        source: Qt.resolvedUrl("../../../fonts/JetBrainsMono-Regular.ttf")
    }
    FontLoader {
        id: bold
        source: Qt.resolvedUrl("../../../fonts/JetBrainsMono-Bold.ttf")
    }

    readonly property string family: regular.status === FontLoader.Ready ? regular.font.family : theme.monoFont
    /*! Loaded so `font.bold` picks the real bold face, never a smeared one. */
    readonly property bool boldReady: bold.status === FontLoader.Ready

    readonly property real dpr: theme.grid.devicePixelRatio > 0 ? theme.grid.devicePixelRatio : 1
    readonly property bool comfortable: theme.density === "comfortable"

    /*! Font size in logical px: the Plasma body size (10 pt -> 13 px). */
    readonly property int pixelSize: {
        const f = Kirigami.Theme.defaultFont;
        const px = f.pixelSize > 0 ? f.pixelSize : f.pointSize * Screen.logicalPixelDensity * 25.4 / 72;
        return Math.max(9, Math.round(px));
    }

    /*! The font's own advance and line height (logical px). Both bundled
        candidates (JetBrains Mono, IBM Plex Mono) advance 600/1000 em; it is
        taken from the design, because FontMetrics measured outside a window
        reports device-scaled values under QT_SCALE_FACTOR. Line box: JetBrains
        Mono ascent 1020 + descent 300 per 1000 em. */
    readonly property real advance: pixelSize * 0.6
    readonly property real fontHeight: pixelSize * 1.32

    /*! Cell size in device px (integers) and logical px. */
    readonly property int cwDev: Math.max(1, Math.round(advance * dpr))
    readonly property int chDev: Math.max(2, Math.round(fontHeight * (comfortable ? 1.28 : 1.06) * dpr))
    readonly property real cw: cwDev / dpr
    readonly property real ch: chDev / dpr
    /*! Added to every glyph advance so runs stay on the grid. */
    readonly property real letterSpacing: cw - advance
    /*! Top padding that centres the font's line box in a cell, device aligned. */
    readonly property real textTop: Math.round((chDev - fontHeight * dpr) / 2) / dpr

    /*! Stroke of box-drawing lines: one device pixel, two from DPR ~1.9. */
    readonly property int lwDev: Math.max(1, Math.round(dpr * 0.8))
    readonly property real lw: lwDev / dpr
    /*! Heavy lines (┃ ━, focus marks): twice the stroke. */
    readonly property real lwHeavy: (lwDev * 2) / dpr

    /*! Rounded corners follow the TUI's glyph style (╭ vs ┌). */
    readonly property bool rounded: theme.glyphStyle !== "pixel"
    /*! Detail connector: the TUI's ⎿, as the box glyph that matches the corners. */
    readonly property string detailGlyph: rounded ? "╰" : "└"

    /*! Whole cells that fit into a logical width. */
    function cols(logicalWidth) {
        return Math.max(0, Math.floor(Math.round(logicalWidth * dpr) / cwDev));
    }
    /*! Left margin that centres the whole cells of a width (device aligned);
        every module uses it, so their columns line up down the popup. */
    function inset(logicalWidth) {
        return Math.floor((Math.round(logicalWidth * dpr) - cols(logicalWidth) * cwDev) / 2) / dpr;
    }
    /*! Logical x of a column / y of a row. */
    function cx(col) {
        return col * cwDev / dpr;
    }
    function cy(row) {
        return row * chDev / dpr;
    }
    /*! Device-aligned position of a cell's centre line (where │ and ─ run). */
    function lineX(col) {
        return (col * cwDev + Math.floor((cwDev - lwDev) / 2)) / dpr;
    }
    function lineY(row) {
        return (row * chDev + Math.floor((chDev - lwDev) / 2)) / dpr;
    }
    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // Cell-string helpers (Term.js), re-exported so components need one import.
    function len(text) {
        return Term.len(text);
    }
    function fit(text, cells) {
        return Term.fit(text, cells);
    }
}
