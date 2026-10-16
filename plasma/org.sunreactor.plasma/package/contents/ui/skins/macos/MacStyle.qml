/*
 * The macos skin's design system: Sonoma / Sequoia dark, as Control Center and
 * Notification Center draw their widgets.
 *
 * Every colour is a palette token or a translucent light or dark mixed from one
 * (no literal colour in any macos file). Sizes are logical px from the macOS type
 * scale: 13 body, 11 caption, light display figures. The faces are the bundled
 * Geist, which this skin owns (the theme's own sans is IBM Plex).
 *
 * Vibrancy is emulated without blur: a panel is a translucent light fill over the
 * background, with one device pixel of highlight along its top edge and one
 * device pixel of outer edge. Nothing here animates.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- device pixels -------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! One device pixel in logical px: every hairline of the skin. */
    readonly property real hairline: 1 / dpr

    /*! Rounds a logical length to whole device pixels. */
    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // --- colour --------------------------------------------------------------

    readonly property color panel: theme.bg
    readonly property color ink: theme.fg
    /*! Secondary labels (#98989d in the theme). */
    readonly property color inkSecondary: theme.textMuted
    /*! Sun glyph and sun arc, the palette's warning yellow. */
    readonly property color sun: theme.warning
    readonly property color accent: theme.accent
    readonly property color warning: theme.warning
    readonly property color ok: theme.success
    readonly property color danger: theme.error

    /*! Vibrancy material: white at 7% over the background. */
    readonly property color material: theme.alpha(ink, 0.07)
    /*! Inner highlight along the top edge: one device pixel, white at 12%. */
    readonly property color materialTop: theme.alpha(ink, 0.12)
    /*! Outer hairline of a panel: white at 8%. */
    readonly property color materialEdge: theme.alpha(ink, 0.08)
    /*! Separators between the rows of an inset list. */
    readonly property color separator: theme.alpha(ink, 0.10)
    /*! Slider fill: near-white at 90%. */
    readonly property color trackFill: theme.alpha(ink, 0.90)
    /*! Slider track behind the fill. */
    readonly property color trackRest: theme.alpha(ink, 0.10)
    /*! The sun glyph where the track is still empty. */
    readonly property color sunOff: theme.alpha(ink, 0.55)
    /*! Small gray controls (Auto, Range). */
    readonly property color chip: theme.alpha(ink, 0.12)
    /*! Capsule behind a segmented control. */
    readonly property color segmentTrack: theme.alpha(ink, 0.10)
    /*! The selected segment: a lighter rounded rect. */
    readonly property color segmentOn: theme.mix(panel, ink, 0.30)
    /*! Under the selected segment: the panel colour, so it darkens the edge below it. */
    readonly property color segmentShadow: theme.alpha(panel, 0.85)
    readonly property color gridline: theme.alpha(ink, 0.07)
    readonly property color tooltip: theme.mix(panel, ink, 0.14)
    readonly property color tooltipEdge: theme.alpha(ink, 0.12)

    // --- type ----------------------------------------------------------------

    readonly property FontLoader geistRegular: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Geist-Regular.ttf")
    }
    readonly property FontLoader geistMedium: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Geist-Medium.ttf")
    }
    readonly property FontLoader geistLight: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Geist-Light.ttf")
    }

    /*! A font of `loader`'s face, or the system face until the file has loaded. */
    function face(loader, size, tracking, tabular) {
        const family = loader.status === FontLoader.Ready
            ? loader.font.family : Kirigami.Theme.defaultFont.family;
        return Qt.font({
            family: family,
            pixelSize: size,
            letterSpacing: tracking,
            features: tabular ? { "tnum": 1 } : {}
        });
    }

    readonly property real scaleFactor: theme.scaleFactor > 0 ? theme.scaleFactor : 1.0

    /*! Body text, 13 px scaled. */
    readonly property font body: face(geistRegular, Math.round(13 * scaleFactor), 0, false)
    readonly property font bodyMedium: face(geistMedium, Math.round(13 * scaleFactor), 0, false)
    /*! Captions and secondary labels, 11 px scaled. */
    readonly property font caption: face(geistRegular, Math.round(11 * scaleFactor), 0, false)
    readonly property font captionMedium: face(geistMedium, Math.round(11 * scaleFactor), 0, false)
    /*! Figures in the lists and the tiles. */
    readonly property font value: face(geistRegular, Math.round(13 * scaleFactor), 0, true)
    readonly property font readout: face(geistRegular, Math.round(14 * scaleFactor), -0.1, true)
    /*! The hero percentage: 32 px scaled. */
    readonly property font hero: face(geistLight, Math.round(32 * scaleFactor), -0.64, true)
    /*! The temperature: light and large, tracked slightly tighter. */
    readonly property font temperature: face(geistLight, Math.round(40 * scaleFactor), -0.8, true)

    // --- metrics (logical px) ---------------------------------------------------

    readonly property int radiusCard: Math.round(12 * scaleFactor)
    readonly property int radiusList: Math.round(10 * scaleFactor)
    readonly property int radiusSegment: Math.round(7 * scaleFactor)
    /*! Horizontal inset of a list row, and the indent of its separators. */
    readonly property int padRow: Math.round(12 * scaleFactor)
    readonly property int padCard: Math.round(12 * scaleFactor)
    readonly property int gap: Math.round(6 * scaleFactor)
    readonly property int section: Math.round(10 * scaleFactor)
    readonly property int sliderHeight: Math.round(28 * scaleFactor)
    readonly property int segmentHeight: Math.round(26 * scaleFactor)
    readonly property int chipHeight: Math.round(18 * scaleFactor)
}
