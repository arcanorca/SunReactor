/*
 * Xbox 360 metallic blade panel: 9-slice BorderImage (assets/xbox_card.png)
 * with brushed silver top rim and optional glowing green focus edge.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property XboxStyle style
    property bool lit: false
    property real pad: style.cardPad

    default property alias content: body.data

    BorderImage {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.style.asset("xbox_card.png"))
        border { left: 12; top: 10; right: 12; bottom: 10 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: true
    }

    Rectangle {
        visible: root.lit
        anchors.fill: parent
        radius: 10
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.green
        antialiasing: true
    }

    Item {
        id: body

        anchors.fill: parent
        anchors.margins: root.pad
    }
}
