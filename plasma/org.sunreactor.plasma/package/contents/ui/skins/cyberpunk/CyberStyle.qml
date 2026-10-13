/*
 * Design tokens of the cyberpunk skin, derived from one Theme. Colours are the
 * palette: fg is the pale-yellow ink, accent the yellow of panels and live data,
 * secondary_accent the cyan of data lines and secondary values, error the red
 * kept for warnings and overrides only. Type: Orbitron for numerals, IBM Plex
 * Sans Condensed Medium for labels, IBM Plex Mono for technical readouts. The
 * hero value is 2.2 times the body size (never more than 2.2).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    // --- device pixels --------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! One device pixel. */
    readonly property real hairline: 1 / dpr
    /*! The panel outline: 1.5 device px. */
    readonly property real outline: 1.5 / dpr
    /*! A 2 device px stroke: traces and the now cursor. */
    readonly property real trace: 2 / dpr
    /*! Offset of the glitch copies on the hero value: 2 device px at 1.0, 3 at 2.0. */
    readonly property real glitch: Math.max(2, Math.round(dpr * 1.5)) / dpr

    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    // --- metrics (logical px) -------------------------------------------------

    readonly property real unit: Kirigami.Units.gridUnit
    readonly property bool compact: theme.compact
    /*! Chamfer of a plain strip. A titled panel is cut as deep as its tab is high. */
    readonly property real cut: snap(Math.round(unit * 0.7))
    readonly property real pad: snap(unit * (compact ? 0.5 : 0.7))
    readonly property real gap: snap(unit * 0.5)
    readonly property real gap2: snap(unit)
    readonly property real rowGap: snap(unit * (compact ? 0.35 : 0.45))
    /*! Height of a title tab (and the cut of a titled panel). */
    readonly property real tabHeight: snap(Math.round(sizeLabel * 1.8))
    readonly property real hatchStep: Math.max(4, Math.round(unit * 0.7))

    // --- type scale -------------------------------------------------------------

    readonly property int sizeTiny: Math.max(8, Math.round(unit * 0.5))
    readonly property int sizeLabel: Math.max(9, Math.round(unit * 0.6))
    readonly property int sizeData: Math.max(10, Math.round(unit * 0.66))
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.8))
    readonly property int sizeMid: Math.round(sizeBody * 1.5)
    readonly property int sizeHero: Math.round(sizeBody * 2.2)

    // --- faces --------------------------------------------------------------------

    readonly property FontLoader orbitron: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/Orbitron-Variable.ttf")
    }
    readonly property FontLoader plexCond: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexSansCondensed-Medium.ttf")
    }
    readonly property FontLoader plexMono: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/IBMPlexMono-Regular.ttf")
    }

    readonly property string displayFace: orbitron.status === FontLoader.Ready
        ? orbitron.font.family : theme.monoFont
    readonly property string labelFace: plexCond.status === FontLoader.Ready
        ? plexCond.font.family : theme.sansFont
    readonly property string techFace: plexMono.status === FontLoader.Ready
        ? plexMono.font.family : theme.monoFont

    function mkFont(family, size, weight, caps, spacing) {
        return Qt.font({
            family: family,
            pixelSize: size,
            weight: weight,
            capitalization: caps ? Font.AllUppercase : Font.MixedCase,
            letterSpacing: spacing
        });
    }

    readonly property font labelFont: mkFont(labelFace, sizeLabel, Font.Medium, true, sizeLabel * 0.1)
    readonly property font tabFont: mkFont(labelFace, sizeLabel, Font.DemiBold, true, sizeLabel * 0.12)
    readonly property font headFont: mkFont(labelFace, sizeBody, Font.DemiBold, true, sizeBody * 0.06)
    readonly property font bodyFont: mkFont(labelFace, sizeBody, Font.Medium, false, 0)
    readonly property font techFont: mkFont(techFace, sizeData, Font.Normal, false, 0)
    readonly property font tinyFont: mkFont(techFace, sizeTiny, Font.Normal, false, 0)
    readonly property font numFont: mkFont(displayFace, sizeData, Font.Medium, false, 0)
    readonly property font midFont: mkFont(displayFace, sizeMid, Font.DemiBold, false, 0)
    /*! A row's figure: the display face at a little above body size. */
    readonly property font figFont: mkFont(displayFace, Math.round(sizeBody * 1.15), Font.Medium, false, 0)
    readonly property font heroFont: mkFont(displayFace, sizeHero, Font.Bold, false, 0)

    // --- colour ---------------------------------------------------------------------

    /*! Navy panel: the background lifted a little towards the lavender muted ink. */
    readonly property color panel: theme.mix(theme.bg, theme.textMuted, 0.06)
    /*! The plot's dark grid ground. */
    readonly property color plot: theme.mix(theme.bg, Qt.rgba(0, 0, 0, 1), 0.35)
    /*! Diagonal hatch: barely there. */
    readonly property color hatch: theme.alpha(theme.textMuted, 0.07)
    readonly property color ink: theme.fg
    readonly property color yellow: theme.accent
    readonly property color cyan: theme.secondaryAccent
    readonly property color red: theme.error
    readonly property color muted: theme.textMuted
    readonly property color faint: theme.mix(panel, theme.textMuted, 0.45)
    readonly property color rule: theme.mix(panel, theme.textMuted, 0.3)
    readonly property color ruleFaint: theme.mix(panel, theme.textMuted, 0.14)
    readonly property color ghost: theme.mix(panel, theme.textMuted, 0.2)
    /*! Text on a solid yellow or red face: the navy background. */
    readonly property color onYellow: theme.bg
    readonly property color onRed: theme.bg
    /*! Bloom strength of emissive traces (widget.glow). */
    readonly property real glow: theme.glow
}
