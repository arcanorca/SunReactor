/*
 * Wii Channel Tile backed by 9-slice BorderImage assets (wii_channel_idle.png
 * and wii_channel_lit.png with recessed silver bevel, gloss, and cyan aura).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property WiiStyle style
    property bool lit: false
    readonly property bool hovered: hover.hovered
    property real pad: style.tilePad

    default property alias content: body.data

    readonly property real shadowRoom: style.shadowRoom
    Layout.bottomMargin: shadowRoom

    BorderImage {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.lit || root.hovered ? "assets/wii_channel_lit.png" : "assets/wii_channel_idle.png")
        border { left: 16; top: 15; right: 16; bottom: 15 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: true
    }

    HoverHandler {
        id: hover
    }

    Item {
        id: body

        anchors.fill: parent
        anchors.margins: root.pad
    }
}
