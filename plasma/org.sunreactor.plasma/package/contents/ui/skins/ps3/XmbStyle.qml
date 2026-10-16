/*
 * The XMB design system, derived from one Theme: white ink over the navy field,
 * blue accents, hairline separators at a low alpha, and soft white glows instead
 * of borders. Every colour comes from theme tokens; sizes follow the Plasma grid
 * unit, so the popup scales with the desktop. Type: Geist Light for labels and
 * the hero numerals, IBM Plex Sans Light for values (tabular figures).
 *
 * The hero numeral is at most 2.4 x the body size (the flagship rule: never
 * shouting, never a third of the popup wide).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- device pixels ---------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! One device pixel in logical px. */
    readonly property real hairline: 1 / dpr
    /*! A line of whole device pixels close to one logical px. */
    readonly property real line: Math.max(1, Math.round(dpr)) / dpr
    /*! Two device pixels: the curve stroke, the underline, the thumb groove. */
    readonly property real line2: Math.max(2, Math.round(dpr * 1.5)) / dpr

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // --- colour ----------------------------------------------------------------

    /*! Primary white: labels, values, the selected item. */
    readonly property color ink: theme.fg
    /*! Sub-text: 60 % white. */
    readonly property color inkSoft: theme.alpha(theme.fg, 0.6)
    /*! Items the pointer and focus are not on: the XMB dims them. */
    readonly property color inkDim: theme.alpha(theme.fg, 0.42)
    /*! Faint separators between list items. */
    readonly property color rule: theme.alpha(theme.fg, 0.12)
    /*! Slider groove and the unlit track. */
    readonly property color track: theme.alpha(theme.fg, 0.16)
    /*! The soft halo behind the item the pointer or focus is on. */
    readonly property color glow: theme.alpha(theme.fg, 0.08)
    readonly property color accent: theme.accent
    readonly property color accentSoft: theme.secondaryAccent
    readonly property color ok: theme.success
    readonly property color warning: theme.warning
    readonly property color danger: theme.error

    function alpha(c, a) {
        return theme.alpha(c, a);
    }

    // --- type ------------------------------------------------------------------

    readonly property FontLoader geistLight: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Geist-Light.ttf")
    }
    readonly property FontLoader plexLight: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexSans-Light.ttf")
    }

    function face(loader, fallback) {
        return loader.status === FontLoader.Ready ? loader.font : fallback;
    }

    readonly property font systemFont: Kirigami.Theme.defaultFont

    function makeFont(base, size, spacing, tabular) {
        return Qt.font({
            family: base.family,
            pixelSize: size,
            letterSpacing: spacing,
            features: tabular ? { "tnum": 1 } : {},
        });
    }

    /*! The type scale (logical px), anchored to the Plasma grid unit. */
    readonly property real unit: theme.gridUnit
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.72))
    readonly property int sizeLegend: Math.max(9, Math.round(sizeBody * 0.8))
    readonly property int sizeTab: Math.round(sizeBody * 1.2)
    readonly property int sizeHero: Math.round(sizeBody * 2.4)
    readonly property int sizeUnit: Math.round(sizeBody * 1.1)

    readonly property font labelFont: makeFont(face(geistLight, systemFont), sizeBody, 0, false)
    readonly property font legendFont: makeFont(face(geistLight, systemFont), sizeLegend,
                                                sizeLegend * 0.05, false)
    readonly property font tabFont: makeFont(face(geistLight, systemFont), sizeTab, 0, false)
    readonly property font valueFont: makeFont(face(plexLight, systemFont), sizeBody, 0, true)
    readonly property font heroFont: makeFont(face(geistLight, systemFont), sizeHero,
                                              -sizeHero * 0.02, true)
    readonly property font unitFont: makeFont(face(plexLight, systemFont), sizeUnit, 0, false)

    // --- metrics ---------------------------------------------------------------

    readonly property real gap: snap(Kirigami.Units.smallSpacing)
    readonly property real gap2: snap(Kirigami.Units.smallSpacing * 2)
    readonly property real gap3: snap(Kirigami.Units.smallSpacing * 3)
    /*! The side margin of every PS3 module, from each edge of the popup (logical px):
        no readout, value or axis label touches the popup's edge. */
    readonly property real sideMargin: 12
    /*! Hit height of a thin slider. */
    readonly property real sliderHeight: snap(sizeBody * 1.1)
}
