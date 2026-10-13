/*
 * The synthwave84 skin's design system, derived from one Theme. Every colour
 * comes from the palette: the night gradient and the card body are shades of
 * bg, the neon roles are accent (hot pink), secondary_accent (cyan) and warning
 * (yellow). Type is the bundled Orbitron for headings, the hero and the
 * temperature, and IBM Plex for body and data.
 *
 * Outrun lettering is sheared by `shear` (tan 10 degrees) on the headings only;
 * the body text stays upright so the numbers read.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- colour -------------------------------------------------------------------

    readonly property color ink: theme.fg
    /*! Hot pink: the sun, the way back, lit states, the day's "now". */
    readonly property color pink: theme.accent
    /*! Electric cyan: grid, day curve, legends. */
    readonly property color cyan: theme.secondaryAccent
    readonly property color yellow: theme.warning
    readonly property color mint: theme.success
    readonly property color red: theme.error
    readonly property color muted: theme.textMuted
    /*! Quiet rules and dividers: the palette's inactive border. */
    readonly property color lilac: theme.borderInactive
    readonly property color night: theme.bg
    /*! Card body: a shade of the night, a little see-through over the gradient. */
    readonly property color card: theme.alpha(theme.mix(theme.bg, Qt.rgba(0, 0, 0, 1), 0.2), 0.78)
    /*! Text on a lit neon fill: the night itself. */
    readonly property color onNeon: theme.bg

    function alpha(c, a) {
        return theme.alpha(c, a);
    }

    function hex(c) {
        return theme.hexOf(c);
    }

    // --- device pixels and metrics ----------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! One device pixel, the neon outline of every card and pill. */
    readonly property real hairline: 1 / dpr
    /*! Two device pixels: lit tubes, focus rings. */
    readonly property real line2: Math.max(2, Math.round(dpr)) / dpr

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    /*! Card corner radius (theme widget.radius, 6 for this theme). */
    readonly property real radius: theme.radius > 0 ? theme.radius : 6
    readonly property real unit: Kirigami.Units.gridUnit
    readonly property real gap: snap(Kirigami.Units.smallSpacing)
    readonly property real gap2: snap(Kirigami.Units.smallSpacing * 2)
    readonly property real rowGap: snap(Kirigami.Units.smallSpacing * 1.5)
    /*! Inside a card. */
    readonly property real pad: snap(Kirigami.Units.smallSpacing * 2)
    /*! Glow strength, 0..1 (widget.glow). */
    readonly property real glow: Math.max(0, Math.min(1, theme.glow))
    /*! Italic lean of the headings: tan(10 degrees). */
    readonly property real shear: Math.tan(10 * Math.PI / 180)

    // --- type scale (logical px) ----------------------------------------------------------

    readonly property int sizeBody: Math.max(12, Math.round(unit * 0.78))
    readonly property int sizeLabel: Math.max(9, Math.round(sizeBody * 0.76))
    readonly property int sizeHeading: Math.max(11, Math.round(sizeBody * 0.92))
    /*! The readout: 1.6x body (a row's value, beside its name). */
    readonly property int sizeReadout: Math.round(sizeBody * 1.6)
    /*! The hero percentage: 2.2x body, the poster's figure without the full-card bulk. */
    readonly property int sizeHero: Math.round(sizeBody * 2.2)

    readonly property string bodyFamily: theme.bodyFont
    readonly property string dataFamily: theme.monoFont
    readonly property string displayFamily: theme.digitsFont

    function makeFont(family, size, weight, spacing, caps, tabular) {
        return Qt.font({
            family: family,
            pixelSize: size,
            weight: weight,
            letterSpacing: spacing,
            capitalization: caps ? Font.AllUppercase : Font.MixedCase,
            features: tabular ? { "tnum": 1, "zero": 0 } : {},
        });
    }

    readonly property font bodyFont: makeFont(bodyFamily, sizeBody, Font.Normal, 0, false, false)
    readonly property font bodyStrongFont: makeFont(bodyFamily, sizeBody, Font.DemiBold, 0, false, false)
    /*! Small caps legends, letter-spaced. */
    readonly property font labelFont: makeFont(bodyFamily, sizeLabel, Font.DemiBold, sizeLabel * 0.16, true, false)
    readonly property font dataFont: makeFont(dataFamily, sizeBody, Font.Normal, 0, false, true)
    readonly property font dataSmallFont: makeFont(dataFamily, sizeLabel, Font.Normal, 0, false, true)
    /*! Sheared section headings. */
    readonly property font headingFont: makeFont(displayFamily, sizeHeading, Font.Bold, sizeHeading * 0.12, true, false)
    readonly property font readoutFont: makeFont(displayFamily, sizeReadout, Font.Bold, 0, false, false)
    readonly property font heroFont: makeFont(displayFamily, sizeHero, Font.Black, -sizeHero * 0.02, false, false)

    // --- weather glyphs ----------------------------------------------------------------

    /*! WeatherGlyph roles: cyan line work, the sun in pink, rain in cyan, the
        bolt in yellow. The glyph lifts any role that is too close to bg. */
    readonly property var glyphPalette: ({
        bg: hex(theme.bg),
        fg: hex(cyan),
        text_muted: hex(theme.mix(cyan, theme.bg, 0.45)),
        accent: hex(pink),
        secondary_accent: hex(cyan),
        warning: hex(yellow)
    })
}
