/*
 * Workbench text: the Topaz-style 8x8 face (PixelGlyphs "topaz", art pixels,
 * 1:1). `pitch` 2 doubles every glyph for the hero figure. Characters the face
 * does not draw become "?", so the text always renders in the Workbench face.
 * Elision and wrapping come from the glyph layout: an elided line ends in the
 * face's own ellipsis, and maximumLines > 1 wraps words. The measured art
 * size (artWidth, artHeight) is what the layout reserves.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

Item {
    id: root

    property string text: ""
    property color color: "black"
    /*! Art pixels per font pixel: 1 for body text, 2 for the hero figure. */
    property int pitch: 1
    /*! Elide (one line) or wrap (maximumLines > 1) beyond this many art px; 0 = none. */
    property int maximumArtWidth: 0
    property int maximumLines: 1
    property int alignment: Qt.AlignLeft

    /*! The face draws printable ASCII plus a few Workbench symbols; the rest is "?". */
    readonly property string shownText: text.replace(/[^\x20-\x7e°·–…▼]/g, "?")
    readonly property int artWidth: glyphs.artWidth
    readonly property int artHeight: glyphs.artHeight
    /*! Height of the capitals, for vertical centring (7 art px per pitch). */
    readonly property int capHeight: glyphs.capHeight
    readonly property bool elided: glyphs.elided
    readonly property int lineCount: glyphs.lineCount

    implicitWidth: artWidth
    implicitHeight: artHeight
    width: artWidth
    height: artHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: text
    Accessible.ignored: text.length === 0

    PixelGlyphs {
        id: glyphs

        text: root.shownText
        face: "topaz"
        color: root.color
        pitch: root.pitch
        letterSpacing: 0
        lineSpacing: root.pitch
        maximumArtWidth: root.maximumArtWidth
        maximumLines: root.maximumLines
        alignment: root.alignment
    }
}
