/*
 * The Wii skin's design system, derived from one Theme. Every colour is a
 * palette colour or a mix of two of them; the type is Nunito (bundled, OFL);
 * the metrics follow the Plasma grid unit and the theme's radius.
 *
 *   face          a channel tile: a light grey-blue base, glossed on top
 *   ink, inkSoft  slate text; the soft legend is AA on the face (4.6:1)
 *   accent        Wii blue, for fills, halos and the gloss of a slider
 *   accentLine    the blue of lines and outlines that must read on the face (3:1)
 *   accentInk     the blue of numerals and of a filled button (AA for text)
 *
 * No animation, no glow and no blur: depth comes from flat, faint offsets.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- device pixels ----------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! One device pixel (about) in logical px: outlines of the tiles. */
    readonly property real line: Math.max(1, Math.round(dpr)) / dpr
    /*! Two device pixels: the selected outline and the thumb ring. */
    readonly property real line2: Math.max(2, Math.round(dpr * 1.5)) / dpr
    /*! The stroke of the day curve: three device pixels, about 3 logical px. */
    readonly property real stroke: Math.max(2, Math.round(dpr * 2.5)) / dpr

    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }
    function floorSnap(v) {
        return Math.floor(v * dpr) / dpr;
    }

    // --- colour -----------------------------------------------------------------

    /*! A palette colour at an alpha: the same as Theme.alpha, one name away. */
    function alpha(c, a) {
        return theme.alpha(c, a);
    }

    readonly property color paper: theme.bg
    readonly property color face: theme.mix(theme.bg, theme.borderInactive, 0.18)
    /*! The bottom of the popup's backdrop; the top is the theme background. */
    readonly property color backdropBottom: theme.mix(theme.bg, theme.borderInactive, 0.5)
    readonly property color ink: theme.fg
    readonly property color inkSoft: theme.mix(theme.fg, theme.textMuted, 0.5)
    readonly property color inkFaint: theme.textMuted
    readonly property color accent: theme.accent
    readonly property color accentLine: theme.mix(theme.accent, theme.fg, 0.25)
    readonly property color accentInk: theme.mix(theme.accent, theme.fg, 0.6)
    readonly property color sky: theme.secondaryAccent
    readonly property color outline: theme.borderInactive
    readonly property color track: theme.borderInactive
    readonly property color ring: theme.textMuted
    readonly property color sun: theme.warning
    readonly property color ok: theme.success
    readonly property color danger: theme.error
    /*! The error red deepened for small text: AA on the face. */
    readonly property color dangerInk: theme.mix(theme.error, theme.fg, 0.35)
    readonly property color shadow: theme.alpha(theme.fg, 0.10)
    readonly property color gloss: theme.alpha(theme.bg, 0.9)
    readonly property color glossClear: theme.alpha(theme.bg, 0)
    /*! WeatherGlyph roles on a channel tile, in the Wii's colours: a yellow sun,
        Wii blue rain, slate-grey clouds. The engine lifts any role that is too
        faint on the face, so the palette is a request, not a promise. */
    readonly property var glyphPalette: ({
        bg: theme.hexOf(face),
        fg: theme.hexOf(theme.mix(ink, inkFaint, 0.35)),
        text_muted: theme.hexOf(inkFaint),
        accent: theme.hexOf(sun),
        secondary_accent: theme.hexOf(accent),
        warning: theme.hexOf(sun)
    })

    // --- type ---------------------------------------------------------------------

    /*! Nunito Regular; the bold face is the same family, weight 700. */
    readonly property FontLoader regularFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Nunito-Regular.ttf")
    }
    readonly property FontLoader boldFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Nunito-Bold.ttf")
    }
    readonly property string family: regularFace.status === FontLoader.Ready
        ? regularFace.font.family : theme.bodyFont
    readonly property bool boldReady: boldFace.status === FontLoader.Ready

    /*! The type scale (logical px), anchored to the Plasma grid unit. The hero
        numeral is 2.5 x the body size: never more than 2.6 x. */
    readonly property real unit: Kirigami.Units.gridUnit
    readonly property int sizeCaption: Math.max(9, Math.round(unit * 0.56))
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.72))
    readonly property int sizeTitle: Math.max(12, Math.round(unit * 0.8))
    readonly property int sizeValue: Math.max(13, Math.round(unit * 0.94))
    readonly property int sizeHero: Math.round(unit * 1.8)

    function fontOf(pixelSize, bold) {
        return Qt.font({
            family: root.family,
            pixelSize: pixelSize,
            weight: bold && root.boldReady ? Font.Bold : Font.Normal,
        });
    }

    readonly property font captionFont: fontOf(sizeCaption, false)
    readonly property font captionBoldFont: fontOf(sizeCaption, true)
    readonly property font bodyFont: fontOf(sizeBody, false)
    readonly property font bodyBoldFont: fontOf(sizeBody, true)
    readonly property font titleFont: fontOf(sizeTitle, true)
    readonly property font valueFont: fontOf(sizeValue, true)
    readonly property font heroFont: fontOf(sizeHero, true)

    // --- metrics ------------------------------------------------------------------

    /*! The channel tile's corner radius: the theme's (18). */
    readonly property real radius: theme.radius
    readonly property real gap: snap(Kirigami.Units.smallSpacing)
    readonly property real gap2: snap(Kirigami.Units.smallSpacing * 2)
    readonly property real tilePad: snap(theme.compact ? Kirigami.Units.smallSpacing : Kirigami.Units.smallSpacing * 1.5)
    readonly property real rowGap: snap(Kirigami.Units.smallSpacing)
    /*! Height of a pill button: a touch target that still sits in the popup's budget. */
    readonly property real pillHeight: snap(unit * 1.5)

    /*! A radius for a box of w x h: the tile radius, never more than half the short side. */
    function radiusFor(w, h) {
        return Math.max(0, Math.min(root.radius, w / 2, h / 2));
    }
}
