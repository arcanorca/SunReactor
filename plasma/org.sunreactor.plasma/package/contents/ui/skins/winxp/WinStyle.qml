/*
 * The Windows XP "Luna" design system, derived from one Theme.
 *
 * Colour comes from the palette. The surfaces take bg and fg; the frame, the
 * title band and the group captions take border_active; the curve, the links
 * and the selection take accent; the progress blocks take secondary_accent; the
 * close button and the default ring take error and warning. Luna's shades (the
 * title gradient, the caption-button faces, the button rim, the etched edges)
 * are mixes of those tokens, so a theme that extends winxp keeps the chrome in
 * its own colours. Luna's client white is the one fixed neutral.
 *
 * Type is IBM Plex Sans (standing in for Tahoma) with tabular figures. No
 * animation: every surface is drawn once and changes only with the daemon.
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
    /*! The frame and the etched edges: whole device pixels, close to one px. */
    readonly property real line: Math.max(1, Math.round(dpr)) / dpr

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // --- colour ----------------------------------------------------------------

    /*! Luna's client white: client panels, highlights, the top of every face. */
    readonly property color white: "#ffffff"
    readonly property color panel: theme.available ? theme.bg : Kirigami.Theme.backgroundColor
    readonly property color ink: theme.fg
    readonly property color inkMuted: theme.textMuted
    /*! The window frame, the title band's ends, the group captions. */
    readonly property color frame: theme.borderActive
    /*! The rim of the frame and of the buttons. */
    readonly property color frameDeep: theme.mix(theme.borderActive, theme.fg, 0.4)
    /*! The middle band of the title bar. */
    readonly property color frameLight: theme.mix(theme.borderActive, white, 0.32)
    readonly property color accent: theme.accent
    readonly property color green: theme.secondaryAccent
    readonly property color greenSheen: theme.mix(theme.secondaryAccent, white, 0.55)
    readonly property color orange: theme.warning
    readonly property color danger: theme.error
    readonly property color capBlueTop: theme.mix(theme.accent, white, 0.45)
    readonly property color capRedTop: theme.mix(theme.error, white, 0.4)
    /*! Etched group edges: a dark edge over a white one. */
    readonly property color etchDark: theme.mix(panel, theme.fg, 0.22)
    readonly property color etchLight: white
    /*! The sunken client edge of a list or a chart. */
    readonly property color clientEdge: theme.mix(panel, theme.fg, 0.5)
    /*! The groove of a trackbar. */
    readonly property color trough: theme.mix(panel, theme.fg, 0.1)
    /*! Graph paper and column rules. */
    readonly property color gridline: theme.mix(white, theme.borderInactive, 0.5)
    /*! Buttons: the hot face warms towards the orange-yellow, the pressed face darkens. */
    readonly property color hoverTop: theme.mix(white, theme.warning, 0.3)
    readonly property color hoverBottom: theme.mix(panel, theme.warning, 0.45)
    readonly property color pressTop: theme.mix(panel, theme.fg, 0.14)
    readonly property color pressBottom: theme.mix(panel, theme.fg, 0.22)

    // --- type ------------------------------------------------------------------

    readonly property FontLoader plexRegular: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexSans-Regular.ttf")
    }
    readonly property FontLoader plexSemi: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexSans-SemiBold.ttf")
    }

    readonly property string sans: plexRegular.status === FontLoader.Ready ? plexRegular.font.family : ""
    readonly property string sansBold: plexSemi.status === FontLoader.Ready ? plexSemi.font.family : sans

    /*! The type scale, anchored to the Plasma grid unit like the other skins. */
    readonly property real unit: Kirigami.Units.gridUnit
    readonly property int sizeSmall: Math.max(10, Math.round(unit * 0.6))
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.72))
    /*! The hero reading is at most 2.6 x the body size. */
    readonly property int sizeHero: Math.round(sizeBody * 2.2)

    readonly property font bodyFont: Qt.font({ family: sans, pixelSize: sizeBody })
    readonly property font boldFont: Qt.font({ family: sansBold, weight: Font.DemiBold, pixelSize: sizeBody })
    readonly property font smallFont: Qt.font({ family: sans, pixelSize: sizeSmall })
    readonly property font smallBoldFont: Qt.font({ family: sansBold, weight: Font.DemiBold, pixelSize: sizeSmall })
    readonly property font figureFont: Qt.font({ family: sans, pixelSize: sizeBody, features: { "tnum": 1 } })
    readonly property font heroFont: Qt.font({ family: sansBold, weight: Font.DemiBold, pixelSize: sizeHero })

    // --- metrics (logical px) ----------------------------------------------------

    readonly property real gap: snap(Kirigami.Units.smallSpacing)
    readonly property real gap2: snap(gap * 2)
    readonly property real pad: snap(gap * 1.5)
    /*! Title band height; the caption buttons sit in it. */
    readonly property real titleH: snap(sizeBody * 1.95)
    readonly property real capSize: snap(titleH * 0.58)
    /*! Window top corners (8 px, as Luna) and bottom corners. */
    readonly property real radiusTop: 8
    readonly property real radiusBottom: 3
    readonly property real radiusButton: 3
    /*! One row of a list, a field or a button's text line. */
    readonly property real rowH: snap(sizeBody * 1.22)

    // --- paths -------------------------------------------------------------------

    /*! Outline of a rectangle with corner radius rt on the top and rb on the bottom. */
    function framePath(x, y, w, h, rt, rb) {
        const t = Math.max(0, Math.min(rt, w / 2, h));
        const b = Math.max(0, Math.min(rb, w / 2, h));
        let d = "M " + x + " " + (y + t);
        d += " A " + t + " " + t + " 0 0 1 " + (x + t) + " " + y;
        d += " L " + (x + w - t) + " " + y;
        d += " A " + t + " " + t + " 0 0 1 " + (x + w) + " " + (y + t);
        d += " L " + (x + w) + " " + (y + h - b);
        if (b > 0) {
            d += " A " + b + " " + b + " 0 0 1 " + (x + w - b) + " " + (y + h);
        }
        d += " L " + (x + b) + " " + (y + h);
        if (b > 0) {
            d += " A " + b + " " + b + " 0 0 1 " + x + " " + (y + h - b);
        }
        return d + " Z";
    }

    /*! Caption-button glyph in a square of side s: 0 minimise, 1 maximise, 2 close. */
    function captionGlyph(kind, s) {
        const a = s * 0.3;
        const b = s * 0.7;
        switch (kind) {
        case 0:
            return "M " + a + " " + (s * 0.7) + " L " + b + " " + (s * 0.7);
        case 1:
            return "M " + a + " " + a + " L " + b + " " + a + " L " + b + " " + b + " L " + a + " " + b + " Z";
        default:
            return "M " + a + " " + a + " L " + b + " " + b + " M " + b + " " + a + " L " + a + " " + b;
        }
    }
}
