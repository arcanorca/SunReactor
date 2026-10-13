/*
 * A status indicator lamp: a small LED with a lens rim. Emissive flavours get
 * a halo from widget.glow; printed panels a flat lens.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property InstrumentStyle style
    property color color: style.ok
    property bool lit: true
    property real diameter: style.snap(style.sizeLegend * 0.75)

    implicitWidth: diameter
    implicitHeight: diameter

    Glow {
        anchors.fill: lens
        source: lens
        strength: root.lit && root.style.emissive ? root.style.glow : 0
    }

    Rectangle {
        id: lens

        anchors.fill: parent
        radius: root.style.isThinkpad ? 0 : width / 2
        antialiasing: radius > 0
        color: root.lit ? root.color : root.style.theme.alpha(root.color, 0.18)
        border.width: root.style.hairline
        border.color: root.style.theme.mix(root.color, root.style.panel, 0.45)
    }
}
