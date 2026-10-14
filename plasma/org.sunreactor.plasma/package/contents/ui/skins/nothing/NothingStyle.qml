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
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- colour -------------------------------------------------------------------

    /*! Card fill: the background lifted by 4 % of the ink (#0a0a0a on black). */
    readonly property color surface: theme.mix(theme.bg, theme.fg, 0.04)
    readonly property color ink: theme.fg
    readonly property color muted: theme.textMuted
    /*! Hairline outlines of cards and pills (border_inactive). */
    readonly property color outline: theme.borderInactive
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

    readonly property real unit: Kirigami.Units.gridUnit
    /*! Card corner radius (BRAND.md: 20 to 28). */
    readonly property real radius: 24
    /*! Space between rows and sections. */
    readonly property real gap: Math.max(4, Math.round(unit * 0.44))
    /*! Inner padding of a card, on every side. */
    readonly property real pad: Math.round(unit * 0.56)
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
    readonly property real pillHeight: Math.round(unit * 1.55)

    // --- type ---------------------------------------------------------------------

    /*! JetBrains Mono (bundled, OFL). A QtObject takes no children, so it is a property. */
    readonly property FontLoader jetbrains: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/JetBrainsMono-Regular.ttf")
    }

    /*! The bundled face, or the theme's mono face if it did not load. */
    readonly property string mono: jetbrains.status === FontLoader.Ready ? jetbrains.font.family : theme.monoFont
    /*! The one label size: every caption, tag, pill and status line. */
    readonly property int labelPx: Math.max(9, Math.round(unit * 0.56))
    /*! Values read in mono: readings, details, times. */
    readonly property int figurePx: Math.max(12, Math.round(unit * 0.84))

    /*! Small UPPERCASE mono labels with +10 % tracking. */
    readonly property font label: Qt.font({
        family: root.mono,
        pixelSize: root.labelPx,
        letterSpacing: root.labelPx * 0.10,
        capitalization: Font.AllUppercase,
    })
    /*! The same size for a lower-case sentence (the caller writes it lower case). */
    readonly property font sentence: Qt.font({
        family: root.mono,
        pixelSize: root.labelPx,
    })
    /*! Tabular figures for values that change in place. */
    readonly property font figure: Qt.font({
        family: root.mono,
        pixelSize: root.figurePx,
        features: { "tnum": 1, "zero": 0 },
    })

    // --- daemon data helpers ------------------------------------------------------

    /*! Short caps channel name: "len-p24h-20" -> "LEN". */
    function shortName(id) {
        const text = String(id);
        const head = text.split(/[-_ .]/)[0] || text;
        return head.slice(0, 4).toUpperCase();
    }

    /*! Linear interpolation of a sample field at time t; -1 outside the samples. */
    function valueAt(samples, t, field) {
        if (!samples || samples.length === 0 || t < samples[0].t || t > samples[samples.length - 1].t) {
            return -1;
        }
        for (let i = 1; i < samples.length; ++i) {
            const b = samples[i];
            if (b.t >= t) {
                const a = samples[i - 1];
                const f = b.t === a.t ? 0 : (t - a.t) / (b.t - a.t);
                return a[field] + (b[field] - a[field]) * f;
            }
        }
        return samples[samples.length - 1][field];
    }

    /*! Where the automatic curve of one display is at time t; -1 when unknown. */
    function targetAt(dayCurve, monitorId, t) {
        if (!dayCurve || !dayCurve.monitors) {
            return -1;
        }
        for (const curve of dayCurve.monitors) {
            if (curve.monitorId === monitorId) {
                return valueAt(curve.samples, t, "pct");
            }
        }
        return -1;
    }
}
