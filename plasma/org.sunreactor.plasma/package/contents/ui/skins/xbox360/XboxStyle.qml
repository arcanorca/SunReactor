/*
 * Xbox 360 Blades Dashboard design system: brushed carbon-slate surfaces,
 * metallic silver rims, glowing Xbox neon green (#80c342), and Chakra Petch
 * geometric console typography.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    readonly property real dpr: theme.grid.devicePixelRatio
    readonly property real line: Math.max(1, Math.round(dpr)) / dpr
    readonly property real line2: Math.max(2, Math.round(dpr * 1.5)) / dpr
    readonly property real stroke: Math.max(2, Math.round(dpr * 2.3)) / dpr

    function snap(v) {
        return Math.round(v * dpr) / dpr;
    }

    function alpha(c, a) {
        return theme.alpha(c, a);
    }

    // --- colours --------------------------------------------------------------
    readonly property bool isDark: theme.xboxDarkMode || theme.luminance(theme.bg) < 0.35
    readonly property string assetPrefix: isDark ? "assets/dark/" : "assets/"
    function asset(name) {
        return assetPrefix + name;
    }

    readonly property color bg: theme.bg
    readonly property color ink: theme.fg
    readonly property color inkSoft: theme.mix(theme.fg, theme.textMuted, 0.45)
    readonly property color muted: theme.textMuted
    readonly property color green: theme.accent
    readonly property color greenBright: theme.secondaryAccent
    readonly property color steel: theme.borderInactive
    readonly property color ok: theme.success
    readonly property color warning: theme.warning
    readonly property color danger: theme.error

    readonly property color grooveFill: isDark ? "#0d1115" : "#c0cad4"
    readonly property color grooveEdge: isDark ? "#3c4752" : "#7b8794"
    readonly property color grooveGlowTop: isDark ? "#86ee46" : "#76de3a"
    readonly property color grooveGlowBottom: isDark ? "#3d9b18" : "#388f16"
    readonly property color allBadgeFill: isDark ? "#46a620" : "#3a9416"
    readonly property color allBadgeEdge: isDark ? "#78d948" : "#2b7010"
    readonly property color indexBg: isDark ? "#1c2d17" : "#e6f4dc"
    readonly property color indexBgOff: isDark ? "#20272f" : "#dce2e8"
    readonly property color gridRule: isDark ? alpha("#ffffff", 0.11) : alpha("#5b6774", 0.22)
    readonly property color curveStroke: isDark ? "#7ce842" : "#44a61a"
    readonly property color curveGlow: isDark ? Qt.rgba(0.54, 0.94, 0.30, 0.38) : Qt.rgba(0.42, 0.84, 0.22, 0.28)
    readonly property color curveAreaTop: isDark ? Qt.rgba(0.45, 0.88, 0.25, 0.42) : Qt.rgba(0.45, 0.86, 0.25, 0.32)
    readonly property color curveAreaBottom: isDark ? Qt.rgba(0.32, 0.70, 0.14, 0.06) : Qt.rgba(0.32, 0.70, 0.14, 0.04)
    readonly property color hoverMarkerEdge: isDark ? "#ffffff" : "#182028"
    readonly property color sunMarkerEdge: isDark ? "#1a2026" : "#f4f7fa"
    function shadeFill(s) {
        return Qt.rgba(0, 0, 0, (isDark ? 0.38 : 0.16) * s);
    }

    // --- typography (Chakra Petch, OFL 1.1) -----------------------------------
    readonly property FontLoader regularFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/ChakraPetch-Regular.ttf")
    }
    readonly property FontLoader semiFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/ChakraPetch-SemiBold.ttf")
    }
    readonly property FontLoader boldFace: FontLoader {
        source: Qt.resolvedUrl("../../../fonts/ChakraPetch-Bold.ttf")
    }

    readonly property string family: regularFace.status === FontLoader.Ready
        ? regularFace.font.family : theme.bodyFont

    readonly property real unit: theme.gridUnit
    readonly property int sizeCaption: Math.max(9, Math.round(unit * 0.56))
    readonly property int sizeBody: Math.max(11, Math.round(unit * 0.72))
    readonly property int sizeTitle: Math.max(12, Math.round(unit * 0.82))
    readonly property int sizeValue: Math.max(13, Math.round(unit * 0.94))
    readonly property int sizeHero: Math.round(unit * 1.75)

    function fontOf(px, weight) {
        return Qt.font({
            family: root.family,
            pixelSize: px,
            weight: weight,
            features: { "tnum": 1 },
        });
    }

    readonly property font captionFont: fontOf(sizeCaption, Font.Normal)
    readonly property font captionBoldFont: fontOf(sizeCaption, Font.DemiBold)
    readonly property font bodyFont: fontOf(sizeBody, Font.Normal)
    readonly property font bodyBoldFont: fontOf(sizeBody, Font.DemiBold)
    readonly property font titleFont: fontOf(sizeTitle, Font.Bold)
    readonly property font valueFont: fontOf(sizeValue, Font.Bold)
    readonly property font heroFont: fontOf(sizeHero, Font.Bold)

    // --- metrics --------------------------------------------------------------
    readonly property real gap: snap(Kirigami.Units.smallSpacing)
    readonly property real gap2: snap(Kirigami.Units.smallSpacing * 2)
    readonly property real cardPad: snap(theme.compact ? Kirigami.Units.smallSpacing * 1.25 : Kirigami.Units.smallSpacing * 1.5)
    readonly property real rowGap: snap(Kirigami.Units.smallSpacing)
    readonly property real pillHeight: snap(unit * 1.55)
    readonly property real radius: theme.radius > 0 ? theme.radius : 10
}
