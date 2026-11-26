/*
 * Xbox 360 Blades backdrop overlay: brushed carbon-metal surface with curved
 * emerald energy swooshes, rendered into assets/xbox_backdrop.png.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property Theme theme

    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }

    clip: true

    Image {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.style.asset("xbox_backdrop.png"))
        fillMode: Image.PreserveAspectCrop
        smooth: true
        mipmap: true
    }
}
