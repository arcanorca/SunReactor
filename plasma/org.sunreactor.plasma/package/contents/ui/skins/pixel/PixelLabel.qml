/*
 * Text inside a PixelSurface (art pixels, 1:1). Uses a built-in glyph face
 * (PixelGlyphs "small" 5x7 or "tiny" 3x5) when it can show every character,
 * otherwise the theme's bitmap font through BitmapText - so a translation with
 * characters the built-in faces lack still renders crisply, just wider.
 * face "bitmap" always uses the theme's bitmap font (C64, DMG titles).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"

Item {
    id: root

    required property Theme theme
    property string text: ""
    property color color: theme.fg
    /*! "small", "tiny" or "bitmap". */
    property string face: "small"
    property bool capitals: false
    /*! 1-bit "greyed out" (System 1 disabled text). */
    property bool stipple: false
    /*! Elide (one line) / wrap width (maximumLines > 1) in art px; 0 = none. */
    property int maximumArtWidth: 0
    property int maximumLines: 1
    property int alignment: Qt.AlignLeft
    property int letterSpacing: 1

    readonly property string shownText: capitals ? text.toUpperCase() : text
    readonly property bool builtin: face !== "bitmap" && glyphs.covers(shownText, face)
    readonly property int artWidth: builtin ? glyphs.artWidth : bitmap.artWidth
    readonly property int artHeight: builtin ? glyphs.artHeight : bitmap.artHeight
    /*! Height of the capitals, for vertical centring. */
    readonly property int capHeight: builtin ? glyphs.capHeight : 8
    readonly property bool elided: builtin ? glyphs.elided : false

    implicitWidth: artWidth
    implicitHeight: artHeight
    width: artWidth
    height: artHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: text
    Accessible.ignored: text.length === 0

    PixelGlyphs {
        id: glyphs

        visible: root.builtin
        text: root.builtin ? root.shownText : ""
        face: root.face === "tiny" ? "tiny" : "small"
        color: root.color
        stipple: root.stipple
        maximumArtWidth: root.maximumArtWidth
        maximumLines: root.maximumLines
        alignment: root.alignment
        letterSpacing: root.letterSpacing
    }

    BitmapText {
        id: bitmap

        visible: !root.builtin
        theme: root.theme
        inSurface: true
        text: root.builtin ? "" : root.shownText
        color: root.color
        maximumArtWidth: root.maximumArtWidth
    }
}
