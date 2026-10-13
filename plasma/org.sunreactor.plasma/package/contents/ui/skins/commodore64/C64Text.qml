/*
 * One line, or wrapped lines, of PETSCII text in art px: 4 art px per character
 * cell, 8 rows per line. PixelGlyphs rasterises it on the CPU at art resolution
 * (exact inside a PixelSurface). Lower case shows as capitals; labels are
 * folded to the glyphs the face has (Petscii.js). Measured in cells: artWidth is
 * 4 per character.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "Petscii.js" as Petscii

Item {
    id: root

    property string text: ""
    property color ink: "transparent"
    /*! Wrap width in art px (0 = one line, no wrap or elision). */
    property int maximumArtWidth: 0
    /*! Lines allowed when wrapping; the last one elides with a "." */
    property int maximumLines: 1
    property int alignment: Qt.AlignLeft
    property int lineSpacing: 0

    readonly property string shown: Petscii.fold(text)
    readonly property int artWidth: glyphs.artWidth
    readonly property int artHeight: glyphs.artHeight
    readonly property int lineCount: glyphs.lineCount
    readonly property bool elided: glyphs.elided

    implicitWidth: artWidth
    implicitHeight: artHeight
    width: artWidth
    height: artHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: text
    Accessible.ignored: text.length === 0

    PixelGlyphs {
        id: glyphs

        text: root.shown
        face: "petscii"
        capitals: true
        letterSpacing: 0
        lineSpacing: root.lineSpacing
        color: root.ink
        maximumArtWidth: root.maximumArtWidth
        maximumLines: root.maximumLines
        alignment: root.alignment
    }
}
