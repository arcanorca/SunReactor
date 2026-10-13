/*
 * Phosphor / filament / neon bloom for an emissive item, scaled by the theme's
 * widget.glow. Two blurred copies of `source` (a tight halo and a wide bloom)
 * are drawn behind it; place the Glow before the source item so it paints
 * underneath, and give it the source's geometry.
 *
 * Cost: the blur is rendered when the source changes, not per frame, so an
 * idle popup stays idle. Keep fast-changing things (hover cursors) out of the
 * source subtree.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects

Item {
    id: root

    required property Item source
    /*! 0..1, normally InstrumentStyle.glow. */
    property real strength: 0
    /*! Tints the bloom (null keeps the source's colours). */
    property color tint: "transparent"

    readonly property bool active: strength > 0.01 && source !== null && source.visible

    visible: active

    MultiEffect {
        anchors.fill: parent
        source: root.active ? root.source : null
        autoPaddingEnabled: true
        blurEnabled: true
        blurMax: 32
        blur: 1.0
        colorization: root.tint.a > 0 ? 1.0 : 0.0
        colorizationColor: root.tint
        opacity: Math.min(0.9, 0.25 + root.strength * 0.55)
    }

    MultiEffect {
        anchors.fill: parent
        source: root.active ? root.source : null
        autoPaddingEnabled: true
        blurEnabled: true
        blurMax: 8
        blur: 1.0
        brightness: 0.1
        colorization: root.tint.a > 0 ? 1.0 : 0.0
        colorizationColor: root.tint
        opacity: Math.min(1, 0.35 + root.strength * 0.6)
    }
}
