/*
 * The Nothing skin's design system, derived from one Theme (skins/nothing/BRAND.md).
 *
 * Colour comes from the palette only: black surface, white ink, grey for
 * secondary text, one red accent for the single signal. Labels are small
 * UPPERCASE JetBrains Mono with +8 % tracking; numerals and headlines are the
 * dot-matrix faces (DotText). The dot pitch is an integer number of device
 * pixels after DotField's own snapping, so the matrix stays crisp at any scale.
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
    /*! The one signal colour: live lamp, now column, call to action, slider thumb. */
    readonly property color accent: theme.accent
    /*! Unlit cells of dot text: barely there, so the glyphs read as glyphs. */
    readonly property color textGhost: theme.alpha(theme.fg, 0.055)
    /*! Unlit cells of bars, rings and curves: the LED-matrix grid. */
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
    readonly property real gap: Math.max(4, Math.round(unit * 0.5))
    /*! Inner padding of a card. */
    readonly property real pad: Math.round(unit * 0.66)
    /*! Dot pitch of text and bars, logical px (DotField snaps it to device px). */
    readonly property real pitch: Math.max(3, Math.round(unit * 0.2))
    /*! Finer pitch for the tiny face and the row percentages: one step under the text pitch. */
    readonly property real pitchFine: Math.max(3, pitch - 1)
    /*! The hero numerals use the text pitch at size 1 of the 5x7 face: 7 dots, about twice a label. */
    readonly property real pitchBig: pitch

    // --- type ---------------------------------------------------------------------

    /*! JetBrains Mono (bundled, OFL). A QtObject takes no children, so it is a property. */
    readonly property FontLoader jetbrains: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/JetBrainsMono-Regular.ttf")
    }

    /*! The bundled face, or the theme's mono face if it did not load. */
    readonly property string mono: jetbrains.status === FontLoader.Ready ? jetbrains.font.family : theme.monoFont
    readonly property int labelPx: Math.max(9, Math.round(unit * 0.56))
    readonly property int textPx: Math.max(11, Math.round(unit * 0.7))
    readonly property int figurePx: Math.max(12, Math.round(unit * 0.82))

    /*! Small UPPERCASE mono labels with +8 % tracking. */
    readonly property font label: Qt.font({
        family: root.mono,
        pixelSize: root.labelPx,
        letterSpacing: root.labelPx * 0.08,
        capitalization: Font.AllUppercase,
    })
    /*! Plain mono sentence (lower case is written by the caller). */
    readonly property font plain: Qt.font({
        family: root.mono,
        pixelSize: root.textPx,
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

    /*! The clear-sky curve as DotCurve samples ({ t, pct }). */
    function plainCurve(samples) {
        return samples ? samples.map(s => ({ t: s.t, pct: s.pctNoWeather })) : [];
    }
}
