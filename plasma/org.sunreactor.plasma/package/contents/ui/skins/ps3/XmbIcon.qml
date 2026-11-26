/*
 * PlayStation 3 XMB Cross Media Bar category icons, backed by etched-crystal
 * glass assets (assets/xmb_cat_brightness.png, assets/xmb_cat_weather.png).
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    property string kind: "sun"
    property color color: "white"
    property real size: 22
    property bool lit: true

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    Image {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.kind === "cloud" ? "assets/xmb_cat_weather.png" : "assets/xmb_cat_brightness.png")
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
        opacity: root.lit ? 1.0 : 0.45
    }
}
