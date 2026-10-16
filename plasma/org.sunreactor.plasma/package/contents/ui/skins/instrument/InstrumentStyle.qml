/*
 * The instrument family's design system, derived from one Theme.
 *
 * Every colour, face and size the instrument skins use comes from here, and
 * everything here comes from theme tokens (palette, widget.font, glow, radius,
 * density, the window's device pixel ratio). The theme id only selects an
 * identity where the hardware it stands for genuinely differs:
 *
 *   flavour   identity                               chosen by
 *   braun     printed panel, light grotesk readouts   default (sans themes, Plasma fallback)
 *   thinkpad  flat IBM type, red TrackPoint thumbs     id thinkpad
 *   vfd       segment digits, ghost segments, glow     id vfd_hifi, or widget.font = display
 *   nixie     tube digits in smoked glass, bargraphs   id nixie
 *   scope     graticule screen, phosphor trace         id oscilloscope
 *   crt       monochrome terminal type, phosphor       widget.font = mono (amber, grayscale, ...)
 *
 * Glow strength is always widget.glow (0 = none), so a user theme that
 * extends any of these gets the same identity with its own colours.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- identity --------------------------------------------------------------

    readonly property string flavour: {
        switch (theme.themeId) {
        case "nixie":
            return "nixie";
        case "vfd_hifi":
            return "vfd";
        case "oscilloscope":
            return "scope";
        case "thinkpad":
            return "thinkpad";
        default:
            if (theme.fontRole === "display") {
                return "vfd";
            }
            return theme.fontRole === "mono" ? "crt" : "braun";
        }
    }
    readonly property bool isBraun: flavour === "braun"
    readonly property bool isThinkpad: flavour === "thinkpad"
    readonly property bool isVfd: flavour === "vfd"
    readonly property bool isNixie: flavour === "nixie"
    readonly property bool isScope: flavour === "scope"
    readonly property bool isCrt: flavour === "crt"

    /*! Emissive displays sit behind glass; printed panels do not. */
    readonly property bool emissive: isVfd || isNixie || isScope || isCrt
    /*! Phosphor / tube / filament bloom, 0..1 (widget.glow). */
    readonly property real glow: Math.max(0, Math.min(1, theme.glow))
    /*! "type" (font), "segment" (7-segment VFD) or "nixie" (cold-cathode tubes). */
    readonly property string digitStyle: isNixie ? "nixie" : (isVfd ? "segment" : "type")
    /*! How a fader is drawn: cap, trackpoint, segments, tube or trace. */
    readonly property string faderStyle: {
        switch (flavour) {
        case "thinkpad":
            return "trackpoint";
        case "vfd":
        case "crt":
            return "segments";
        case "nixie":
            return "tube";
        case "scope":
            return "trace";
        default:
            return "cap";
        }
    }

    // --- device pixels ---------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! One device pixel in logical px. */
    readonly property real hairline: 1 / dpr
    /*! A line of whole device pixels close to one logical px. */
    readonly property real line: Math.max(1, Math.round(dpr)) / dpr
    /*! Two-device-pixel stroke for emphasis (thin enough to stay a line). */
    readonly property real line2: Math.max(2, Math.round(dpr * 1.5)) / dpr

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }
    /*! Rounds down to whole device pixels (for sizes that must not overflow). */
    function floorSnap(v) {
        return Math.floor(v * dpr) / dpr;
    }

    // --- colour ------------------------------------------------------------------

    readonly property color panel: theme.available ? theme.bg : Kirigami.Theme.backgroundColor
    readonly property color ink: theme.fg
    readonly property color inkMuted: theme.textMuted
    /*! Scale numerals and secondary legends: between muted text and the rules. */
    readonly property color inkFaint: theme.mix(panel, theme.textMuted, 0.72)
    readonly property color accent: theme.accent
    readonly property color accent2: theme.secondaryAccent
    readonly property color rule: theme.line
    /*! Graticule and minor ticks: a rule pushed back towards the panel. */
    readonly property color ruleFaint: theme.mix(panel, theme.line, isScope || isCrt || isVfd ? 0.75 : 0.6)
    readonly property color ruleStrong: theme.mix(theme.line, theme.fg, 0.22)
    readonly property color warning: theme.warning
    readonly property color danger: theme.error
    readonly property color ok: theme.success

    /*! Display glass: the panel with depth, faintly tinted by its own light. */
    readonly property color glass: {
        const deep = Qt.rgba(panel.r * 0.55, panel.g * 0.55, panel.b * 0.55, 1);
        return theme.mix(deep, ink, isNixie ? 0.018 : 0.012);
    }
    readonly property color glassEdge: theme.mix(glass, theme.line, 0.85)
    /*! Unlit segments / cold cathodes. */
    readonly property color ghost: theme.alpha(ink, isNixie ? 0.075 : (glow > 0 ? 0.075 : 0.05))
    /*! Printed fader cap (Braun): a warm light grey taken from the palette. */
    readonly property color capFace: theme.mix(ink, inkMuted, 0.35)
    readonly property color capShade: theme.mix(inkMuted, panel, 0.35)

    function stateTone(mode) {
        return theme.stateColor(mode);
    }

    // --- type ------------------------------------------------------------------------

    readonly property FontLoader geistRegular: FontLoader { source: Qt.resolvedUrl("../../../fonts/Geist-Regular.ttf") }
    readonly property FontLoader geistLight: FontLoader { source: Qt.resolvedUrl("../../../fonts/Geist-Light.ttf") }
    readonly property FontLoader geistMedium: FontLoader { source: Qt.resolvedUrl("../../../fonts/Geist-Medium.ttf") }
    readonly property FontLoader geistMono: FontLoader { source: Qt.resolvedUrl("../../../fonts/GeistMono-Regular.ttf") }
    readonly property FontLoader plexSans: FontLoader { source: Qt.resolvedUrl("../../../fonts/IBMPlexSans-Regular.ttf") }
    readonly property FontLoader plexLight: FontLoader { source: Qt.resolvedUrl("../../../fonts/IBMPlexSans-Light.ttf") }
    readonly property FontLoader plexCond: FontLoader { source: Qt.resolvedUrl("../../../fonts/IBMPlexSansCondensed-Medium.ttf") }
    readonly property FontLoader plexMono: FontLoader { source: Qt.resolvedUrl("../../../fonts/IBMPlexMono-Regular.ttf") }

    function face(loader, fallback) {
        return loader.status === FontLoader.Ready ? loader.font : fallback;
    }

    readonly property font systemFont: Kirigami.Theme.defaultFont
    readonly property bool monoType: isVfd || isScope || isCrt

    /*! The type scale (logical px), anchored to the Plasma grid unit. */
    readonly property real unit: theme.gridUnit
    readonly property int sizeLegend: Math.max(8, Math.round(unit * 0.5))
    readonly property int sizeData: Math.max(10, Math.round(unit * 0.62))
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.72))
    readonly property int sizeMid: Math.round(unit * (theme.compact ? 1.0 : 1.12))
    readonly property int sizeReadout: Math.round(unit * (theme.compact ? 1.5 : 1.7))

    function makeFont(base, size, spacing, caps, tabular) {
        return Qt.font({
            family: base.family,
            styleName: base.styleName,
            weight: base.weight,
            pixelSize: size,
            letterSpacing: spacing,
            capitalization: caps ? Font.AllUppercase : Font.MixedCase,
            features: tabular ? { "tnum": 1, "zero": 0 } : {},
        });
    }

    readonly property font legendFace: {
        if (monoType) {
            return face(geistMono, systemFont);
        }
        if (isThinkpad || isNixie) {
            return face(plexCond, systemFont);
        }
        return face(geistMedium, systemFont);
    }
    readonly property font bodyFace: {
        if (isCrt || isScope) {
            return face(plexMono, systemFont);
        }
        if (isVfd) {
            return face(geistMono, systemFont);
        }
        if (isThinkpad || isNixie) {
            return face(plexSans, systemFont);
        }
        return face(geistRegular, systemFont);
    }
    readonly property font dataFace: {
        if (isThinkpad || isNixie || isCrt || isScope) {
            return face(plexMono, systemFont);
        }
        return face(geistMono, systemFont);
    }
    readonly property font readoutFace: {
        if (isCrt || isScope) {
            return face(plexMono, systemFont);
        }
        if (isThinkpad || isNixie) {
            return face(plexLight, systemFont);
        }
        if (isVfd) {
            return face(geistMono, systemFont);
        }
        return face(geistLight, systemFont);
    }

    /*! Tiny caps legend, as silk-screened on a front panel. */
    readonly property font legendFont: makeFont(legendFace, sizeLegend, monoType ? sizeLegend * 0.08 : sizeLegend * 0.12,
                                                true, false)
    readonly property font bodyFont: makeFont(bodyFace, sizeBody, 0, false, false)
    readonly property font dataFont: makeFont(dataFace, sizeData, 0, false, true)
    readonly property font dataSmallFont: makeFont(dataFace, sizeLegend, 0, false, true)
    readonly property font midFont: makeFont(readoutFace, sizeMid, 0, false, true)
    readonly property font readoutFont: makeFont(readoutFace, sizeReadout, isBraun ? -sizeReadout * 0.02 : 0, false, true)

    // --- metrics ---------------------------------------------------------------------

    /*! Corner radius for glass and tags (theme radius; 0 keeps it square). */
    readonly property real radius: theme.isVector ? theme.radius : 0
    readonly property real tagRadius: Math.min(2, radius)
    readonly property real gap: snap(Kirigami.Units.smallSpacing)
    readonly property real gap2: snap(Kirigami.Units.smallSpacing * 2)
    readonly property real rowGap: snap(theme.compact ? Kirigami.Units.smallSpacing : Kirigami.Units.smallSpacing * 1.5)
    readonly property real glassPad: snap(theme.compact ? Kirigami.Units.smallSpacing * 1.5 : Kirigami.Units.smallSpacing * 2)
}
