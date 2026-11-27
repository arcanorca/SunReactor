/*
 * The Nintendo DS skin's design system, derived from one Theme: the console's
 * type (unscii-16, bundled), its colours and the metrics its screens and
 * touch buttons share. Colours come from the theme: accent is the DS IPL accent,
 * secondary accent the orange, bg the pearl shell. Screen, bezel and hinge greys
 * are the hardware's own and are fixed here.
 *
 * Sizes are in `gu` (the popup's grid unit), so the skin grows with the popup
 * and with the font DPI; the bezel, outline and hinge are fixed logical px.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- type ---------------------------------------------------------------------

    /*! Pixelated DS System Menu face (unscii-16, crisp bitmap glyphs). */
    readonly property FontLoader regularFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/unscii-16.ttf")
    }
    readonly property FontLoader boldFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/unscii-16.ttf")
    }

    readonly property string family: regularFace.status === FontLoader.Ready
        ? regularFace.font.family : "monospace"

    // --- metrics ------------------------------------------------------------------

    /*! The popup's grid unit, logical px. */
    readonly property real gu: theme.gridUnit
    /*! Panel corners: crisp pixelated 2px step. */
    readonly property real radius: 2
    /*! Thickness of the black bezel around each screen (matches original DS theme). */
    readonly property int bezel: 6
    /*! Outline of a touch button, DS blue. */
    readonly property int outline: 2
    /*! The hinge strip between the two screens. */
    readonly property real hingeHeight: Math.round(gu * 0.6)

    // --- colours ------------------------------------------------------------------

    readonly property color ink: theme.fg
    readonly property color muted: theme.textMuted
    /*! DS blue: panel outlines, selected buttons, the curve. */
    readonly property color blue: theme.accent
    /*! DS orange: the sun, the next sun time, the warm accents. */
    readonly property color orange: theme.secondaryAccent
    readonly property color line: theme.borderInactive
    readonly property color success: theme.success
    readonly property color warning: theme.warning
    readonly property color danger: theme.error

    readonly property color white: "#ffffff"
    readonly property color frameBlack: "#000000"
    readonly property color cardSurface: "#ffffff"
    readonly property color cardScanline: "#efefef"
    readonly property color headerNeutral: "#eaeaea"
    readonly property color gridOuter: "#bebebe"
    readonly property color gridInner: "#d0d0d0"
    readonly property color bevelHighlight: "#ffffff"
    readonly property color bevelShadow: "#a8a8a8"
    readonly property color clockNumerals: "#b0b0b0"
    readonly property color clockHands: "#787878"
    readonly property color clockPivot: "#47484a"
    readonly property color clockSecondRef: "#8da0ae"
    readonly property color batteryOk: "#38b848"
    readonly property color batteryLow: "#d84848"
    /*! True when the active IPL accent is bright enough to require dark ink/frames. */
    readonly property bool lightBar: theme.luminance(blue) > 0.32
    /*! Contrast text colour over the active IPL accent bar / active button. */
    readonly property color barInk: lightBar ? "#111111" : "#ffffff"
    /*! Accent ink readable on white/light card surfaces even for bright IPL overrides (e.g. Yellow, Orange, Green). */
    readonly property color accentInk: theme.luminance(blue) > 0.30 ? theme.mix(blue, frameBlack, 0.50) : blue

    readonly property color panelTop: "#ffffff"
    readonly property color panelBottom: "#efefef"
    readonly property color quietTop: "#eaeaea"
    readonly property color quietBottom: "#dedede"
    readonly property color bezelTop: "#3b414c"
    readonly property color bezelBottom: "#111318"
    readonly property color skyTop: "#f9f9f9"
    readonly property color skyMid: "#f4f6f9"
    readonly property color skyBottom: "#f9f9f9"
    readonly property color touchTop: "#ffffff"
    readonly property color touchMid: "#fafbfd"
    readonly property color touchBottom: "#f3f6fa"
    readonly property color hingeTop: "#f1f4f8"
    readonly property color hingeBottom: "#aab5c4"
    readonly property color hingeSlot: "#4b5563"
    readonly property color track: "#dce4ee"
    /*! The speech bubble of the automation sentence, and its edge. */
    readonly property color cream: "#fff6d8"
    readonly property color creamEdge: "#e2cb8c"

    /*! The same colour at a given opacity. */
    function alpha(c, a) {
        return Qt.rgba(c.r, c.g, c.b, a);
    }
}
