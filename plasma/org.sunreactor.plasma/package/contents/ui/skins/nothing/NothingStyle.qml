/*
 * The Nothing skin's design system, derived from one Theme (skins/nothing/BRAND.md).
 *
 * Colour comes from the palette only: black surface, white ink, grey for
 * secondary text, one red accent for the single signal. Type is two faces:
 * small UPPERCASE JetBrains Mono labels with +10 % tracking (one size for every
 * label), and the dot-matrix 5x7 face for numerals (DotText, no unlit cells).
 * Dot pitches are integer device pixels after DotField's own snapping, so the
 * matrix stays crisp at any scale.
 *
 * The helpers at the bottom read the daemon's day curve the way the instrument
 * skin does; they are copied rather than shared, so this skin can change alone.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- colour -------------------------------------------------------------------

    /*! Card fill: matte Nothing charcoal (#18181A on OLED black), authentic Nothing OS 2.5/3.0 squircle. */
    readonly property color surface: theme.mix(theme.bg, theme.fg, 0.095)
    /*! Recessed track / elevated matte charcoal (#252528 on OLED black). */
    readonly property color surfaceElevated: theme.mix(theme.bg, theme.fg, 0.145)
    readonly property color ink: theme.fg
    readonly property color muted: theme.textMuted
    /*! Subtle hairline outlines of cards and pills. */
    readonly property color outline: theme.alpha(theme.fg, 0.10)
    /*! The one signal colour: live lamp, now dot, ring head, slider thumb, call to action. */
    readonly property color accent: theme.accent
    /*! Unlit cells of dot numerals: none drawn, so only the glyphs show. */
    readonly property color textGhost: "transparent"
    /*! Unlit cells of bars, rings and rows: the LED-matrix grid. */
    readonly property color dotGhost: theme.alpha(theme.fg, 0.12)

    readonly property real hairline: theme.hairline
    readonly property real dpr: theme.grid.devicePixelRatio

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // --- metrics ------------------------------------------------------------------

    readonly property real unit: theme.gridUnit
    /*! Card corner radius (BRAND.md: 20 to 24). */
    readonly property real radius: 22
    /*! Space between rows and sections. */
    readonly property real gap: Math.max(4, Math.round(unit * 0.34))
    /*! Inner padding of a card, on every side. */
    readonly property real pad: Math.round(unit * 0.42)
    /*! Hero numerals, the ring, the day plot and the sliders: at least 4 device px. */
    readonly property real pitch: Math.max(4, Math.round(unit * 0.22))
    /*! Row numerals (per display, forecast, telemetry): 4 device px at every scale. DotField
     *  snaps the logical pitch to whole device pixels, so 4 / dpr lands on exactly 4. */
    readonly property real pitchDigit: 4 / dpr
    /*! Row icons (forecast): 12 x 12 pictograms at 3 device px minimum. */
    readonly property real pitchFine: Math.max(3, Math.round(unit * 0.17))
    /*! Every dot numeral: a lit dot is this share of its pitch (bold, not a faint speckle). */
    readonly property real digitScale: 0.75
    /*! The 12 x 12 weather pictogram, larger than the row icons. */
    readonly property real pitchIcon: Math.max(4, Math.round(unit * 0.28))
    /*! Every pill: tabs, modes, displays, badges. */
    readonly property real pillHeight: Math.round(unit * 1.38)

    // --- type ---------------------------------------------------------------------

    /*! JetBrains Mono (OFL), Geist (OFL), and Doto round-dot matrix (OFL). */
    readonly property FontLoader jetbrains: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/JetBrainsMono-Regular.ttf")
    }
    readonly property FontLoader geistReg: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Geist-Regular.ttf")
    }
    readonly property FontLoader geistMedium: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Geist-Medium.ttf")
    }
    readonly property FontLoader dotoSemi: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Doto-SemiBold.ttf")
    }
    readonly property FontLoader dotoBold: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Doto-Bold.ttf")
    }

    readonly property string mono: jetbrains.status === FontLoader.Ready ? jetbrains.font.family : theme.monoFont
    readonly property string geistRegular: geistReg.status === FontLoader.Ready ? geistReg.font.family : theme.sansFont
    readonly property string geistMed: geistMedium.status === FontLoader.Ready ? geistMedium.font.family : geistRegular
    readonly property string dotoFamily: dotoBold.status === FontLoader.Ready ? dotoBold.font.family : mono
    readonly property int labelPx: Math.max(9, Math.round(unit * 0.54))
    readonly property int bodyPx: Math.max(11, Math.round(unit * 0.68))
    readonly property int sentencePx: Math.max(10, Math.round(unit * 0.60))
    readonly property int figurePx: Math.max(12, Math.round(unit * 0.78))
    readonly property int dotoHeadingPx: Math.max(11, Math.round(unit * 0.70))
    readonly property int dotoHeroPx: Math.round(unit * 1.42)
    readonly property int dotoValuePx: Math.max(13, Math.round(unit * 0.98))

    readonly property font heading: Qt.font({
        family: root.dotoFamily,
        pixelSize: root.dotoHeadingPx,
        weight: Font.Bold,
        letterSpacing: root.dotoHeadingPx * 0.06,
        capitalization: Font.AllUppercase,
    })
    readonly property font dotHero: Qt.font({
        family: root.dotoFamily,
        pixelSize: root.dotoHeroPx,
        weight: Font.Bold,
        features: { "tnum": 1 },
    })
    readonly property font dotValue: Qt.font({
        family: root.dotoFamily,
        pixelSize: root.dotoValuePx,
        weight: Font.Bold,
        features: { "tnum": 1 },
    })

    /*! Small UPPERCASE mono labels with +10 % tracking. */
    readonly property font label: Qt.font({
        family: root.mono,
        pixelSize: root.labelPx,
        letterSpacing: root.labelPx * 0.10,
        capitalization: Font.AllUppercase,
    })
    /*! Crisp Swiss grotesque primary UI text (display names, headings). */
    readonly property font body: Qt.font({
        family: root.geistMed,
        pixelSize: root.bodyPx,
        weight: Font.Medium,
    })
    /*! Crisp Swiss grotesque copy for status and condition lines. */
    readonly property font sentence: Qt.font({
        family: root.geistRegular,
        pixelSize: root.sentencePx,
        weight: Font.Normal,
    })
    /*! Tabular figures for values that change in place. */
    readonly property font figure: Qt.font({
        family: root.geistMed,
        pixelSize: root.figurePx,
        weight: Font.Medium,
        features: { "tnum": 1 },
    })
}
