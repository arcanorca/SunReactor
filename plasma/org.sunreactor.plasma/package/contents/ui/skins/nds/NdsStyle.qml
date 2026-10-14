/*
 * The Nintendo DS skin's design system, derived from one Theme: the console's
 * type (Nunito, bundled and OFL), its colours and the metrics its screens and
 * touch buttons share. Colours come from the theme: accent is the DS blue,
 * secondary accent the orange, bg the pearl shell. Screen, bezel and hinge greys
 * are the hardware's own and are fixed here.
 *
 * Sizes are in `gu` (the popup's grid unit), so the skin grows with the popup
 * and with the font DPI; the bezel, outline and hinge are fixed logical px.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- type ---------------------------------------------------------------------

    /*! Nunito Regular and Bold (the Bold file is the same family at weight 700). */
    readonly property FontLoader regularFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Nunito-Regular.ttf")
    }
    readonly property FontLoader boldFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Nunito-Bold.ttf")
    }

    /*! Nunito once it has loaded, a generic sans-serif until then. */
    readonly property string family: regularFace.status === FontLoader.Ready
        ? regularFace.font.family : "sans-serif"

    // --- metrics ------------------------------------------------------------------

    /*! The popup's grid unit, logical px. */
    readonly property real gu: Kirigami.Units.gridUnit
    /*! Panel corners: the theme's radius (14 px). */
    readonly property real radius: theme.radius > 0 ? theme.radius : 14
    /*! Thickness of the black bezel around each screen. */
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
    readonly property color pearl: theme.bg
    readonly property color line: theme.borderInactive
    readonly property color success: theme.success
    readonly property color warning: theme.warning
    readonly property color danger: theme.error

    readonly property color white: "#ffffff"
    readonly property color panelTop: "#ffffff"
    readonly property color panelBottom: "#e3eefc"
    readonly property color quietTop: "#f7f8fa"
    readonly property color quietBottom: "#e5e9ef"
    readonly property color bezelTop: "#3b414c"
    readonly property color bezelBottom: "#111318"
    readonly property color skyTop: "#b7dbff"
    readonly property color skyMid: "#dcefff"
    readonly property color skyBottom: "#f6fbff"
    readonly property color touchTop: "#fafcff"
    readonly property color touchMid: "#f0f5fb"
    readonly property color touchBottom: "#e5edf7"
    readonly property color hingeTop: "#f1f4f8"
    readonly property color hingeBottom: "#aab5c4"
    readonly property color hingeSlot: "#4b5563"
    readonly property color track: "#cfe0f7"

    /*! The same colour at a given opacity. */
    function alpha(c, a) {
        return Qt.rgba(c.r, c.g, c.b, a);
    }
}
