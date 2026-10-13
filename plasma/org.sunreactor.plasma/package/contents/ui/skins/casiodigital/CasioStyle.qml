/*
 * Design tokens for the casiodigital skin: a watch-style liquid crystal in a
 * moulded dark case. Every colour comes from the theme palette, so the skin is
 * the theme's identity and a user theme can reuse it:
 *
 *   case      bg lifted toward fg: dark resin, legends silk-screened in fg
 *   glass     the liquid crystal: secondary accent and fg, a grey-green
 *   ink       the segments: bg tinted by the secondary accent, dark on the glass
 *   ghost     ink at 8 %: the unlit segments you see on a real panel
 *   backlight the accent: the now marker, the lit key LED, the focus ring
 *
 * Sizes are logical px from the Plasma grid unit; digit heights are the cap
 * height of a 7-segment readout. The hero value is about 2.4 times the body.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- device pixels --------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    readonly property real hairline: 1 / dpr
    /*! Two device pixels: focus rings and the now marker. */
    readonly property real line2: 2 / dpr

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // --- colour ----------------------------------------------------------------

    readonly property color caseTop: theme.mix(theme.bg, theme.fg, 0.12)
    readonly property color caseBottom: theme.mix(theme.bg, theme.fg, 0.03)
    readonly property color caseEdge: theme.alpha(theme.fg, 0.2)
    /*! Hairline highlight on the inside top edge of a case. */
    readonly property color caseShine: theme.alpha(theme.fg, 0.26)
    /*! Silk-screened legends on the case. */
    readonly property color legend: theme.mix(theme.bg, theme.fg, 0.66)
    readonly property color legendMuted: theme.mix(theme.bg, theme.fg, 0.40)

    readonly property color glass: theme.mix(theme.secondaryAccent, theme.fg, 0.4)
    readonly property color glassShade: theme.mix(glass, theme.bg, 0.35)
    readonly property color glassLight: theme.mix(glass, theme.fg, 0.35)

    readonly property color ink: theme.mix(theme.bg, theme.secondaryAccent, 0.12)
    readonly property color ghost: theme.alpha(ink, 0.08)
    /*! Unlit indicator legends: more visible than a segment ghost, to read as print. */
    readonly property color inkFaint: theme.alpha(ink, 0.16)
    readonly property color inkMuted: theme.alpha(ink, 0.62)

    readonly property color backlight: theme.accent
    readonly property color backlightSoft: theme.alpha(theme.accent, 0.20)

    readonly property color keyTop: theme.mix(theme.bg, theme.fg, 0.14)
    readonly property color keyBottom: theme.mix(theme.bg, theme.fg, 0.05)
    readonly property color keyDownTop: theme.mix(theme.bg, theme.fg, 0.03)
    readonly property color keyDownBottom: theme.mix(theme.bg, theme.fg, 0.09)

    /*! Hex colours for WeatherGlyph: every role is the ink, on the glass. */
    readonly property var glyphPalette: ({
        bg: theme.hexOf(glass),
        fg: theme.hexOf(ink),
        text_muted: theme.hexOf(ink),
        accent: theme.hexOf(ink),
        secondary_accent: theme.hexOf(ink),
        warning: theme.hexOf(ink)
    })

    // --- type ------------------------------------------------------------------

    readonly property FontLoader plexCond: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexSansCondensed-Medium.ttf")
    }
    readonly property FontLoader plexSans: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexSans-Regular.ttf")
    }
    readonly property FontLoader plexMono: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexMono-Regular.ttf")
    }

    function family(loader, fallback) {
        return loader.status === FontLoader.Ready ? loader.font.family : fallback;
    }
    readonly property string capFamily: family(plexCond, Kirigami.Theme.defaultFont.family)
    readonly property string bodyFamily: family(plexSans, Kirigami.Theme.defaultFont.family)
    readonly property string dataFamily: family(plexMono, "monospace")

    /*! Printed caption: condensed, upper case, letter-spaced like a silk-screen. */
    function cap(px) {
        return Qt.font({
            family: capFamily,
            pixelSize: px,
            letterSpacing: px * 0.14,
            capitalization: Font.AllUppercase
        });
    }
    /*! Running text. */
    function body(px) {
        return Qt.font({ family: bodyFamily, pixelSize: px });
    }
    /*! Figures in running text (tabular). */
    function data(px) {
        return Qt.font({ family: dataFamily, pixelSize: px, features: { "tnum": 1 } });
    }

    readonly property real unit: Kirigami.Units.gridUnit
    readonly property int sizeCap: Math.max(8, Math.round(unit * 0.5))
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.72))
    /*! 7-segment digit heights: small for times, medium for readings, hero for the target. */
    readonly property int digitSmall: Math.round(sizeBody * 1.2)
    readonly property int digitMedium: Math.round(sizeBody * 1.6)
    readonly property int digitHero: Math.round(sizeBody * 2.4)

    // --- metrics ---------------------------------------------------------------

    readonly property real pad: snap(unit * 0.4)
    readonly property real glassPad: snap(unit * 0.42)
    readonly property real gap: snap(unit * 0.25)
    readonly property real gapWide: snap(unit * 0.5)
    readonly property real caseRadius: Math.max(2, theme.radius)
    readonly property real glassRadius: Math.max(2, theme.radius - 3)
    readonly property real keyRadius: Math.max(2, theme.radius - 1)
}
