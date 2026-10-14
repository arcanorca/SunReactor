/*
 * Text in the theme's bitmap font at an exact 1:1 size (DESIGN.md section 4,
 * pixel rule 2). Glyphs are rasterised on the CPU at the font's native grid
 * (Press Start 2P: 8 px, unscii: 16 px, times `magnification`) without anti-aliasing,
 * so they only ever contain `color`, and are shown with nearest filtering.
 *
 *   inSurface: true   inside a PixelSurface; geometry in art pixels
 *   inSurface: false  standalone; one art pixel = theme.grid.unit logical px
 *
 * When the theme has no bitmap font (or it failed to load) this falls back to
 * vector mono text at the equivalent size - never to a bitmap font at a
 * non-native size, which would blur (DESIGN.md section 5).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

Item {
    id: root

    required property Theme theme
    property string text: ""
    property color color: theme.fg
    /*! Whole multiple of the font's native size (1 = 8 px Press Start 2P). */
    property int magnification: 1
    property bool inSurface: false
    /*! Elide on the right beyond this many art px (0 = never). */
    property int maximumArtWidth: 0
    property int letterSpacing: 0

    readonly property bool bitmap: theme.bitmapFont.length > 0
    readonly property real artScale: inSurface ? 1 : theme.grid.unit
    /*! Size in art pixels, for layouts inside a PixelSurface. */
    readonly property int artWidth: bitmap ? glyphs.artWidth : Math.ceil(fallback.implicitWidth / artScale)
    readonly property int artHeight: bitmap ? glyphs.artHeight : Math.ceil(fallback.implicitHeight / artScale)

    implicitWidth: bitmap ? glyphs.implicitWidth : fallback.implicitWidth
    implicitHeight: bitmap ? glyphs.implicitHeight : fallback.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: text

    PixelText {
        id: glyphs

        visible: root.bitmap
        text: root.bitmap ? root.text : ""
        color: root.color
        fontFamily: root.theme.bitmapFont
        pixelSize: root.theme.bitmapPixelSize * Math.max(1, root.magnification)
        artScale: root.artScale
        maximumArtWidth: root.maximumArtWidth
        letterSpacing: root.letterSpacing
    }

    Text {
        id: fallback

        visible: !root.bitmap
        text: root.bitmap ? "" : root.text
        color: root.color
        font.family: root.theme.monoFont
        font.pixelSize: Math.max(1, Math.round(root.theme.bitmapPixelSize * Math.max(1, root.magnification) * root.artScale))
        textFormat: Text.PlainText
        elide: root.maximumArtWidth > 0 ? Text.ElideRight : Text.ElideNone
        width: root.maximumArtWidth > 0 ? root.maximumArtWidth * root.artScale : implicitWidth
    }
}
