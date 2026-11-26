/*
 * Nintendo Wii System Menu background overlay: iconic horizontal pinstripes
 * across the popup plus the curved silver-chrome bottom dock bar behind the footer.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property Theme theme

    property bool auditIgnore: true
    clip: true

    Image {
        anchors.fill: parent
        source: Qt.resolvedUrl("assets/wii_pinstripes.png")
        fillMode: Image.Tile
        opacity: 0.72
    }

    BorderImage {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Math.round(root.theme.gridUnit * 2.15)
        source: Qt.resolvedUrl("assets/wii_dock.png")
        border { left: 32; right: 32; top: 12; bottom: 0 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: true
    }
}
