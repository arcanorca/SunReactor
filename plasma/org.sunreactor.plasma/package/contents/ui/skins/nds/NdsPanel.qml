/*
 * Nintendo DS 3D beveled touch button backed by 9-slice BorderImage assets
 * (ds_btn_panel.png, ds_btn_active.png, ds_btn_quiet.png).
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property NdsStyle style
    property string kind: "panel"
    property color outlineColor: style.blue
    property int outlineWidth: style.outline
    property real radius: style.radius

    default property alias content: stage.data

    BorderImage {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.kind === "ds" ? "assets/ds_btn_active.png"
            : (root.kind === "quiet" ? "assets/ds_btn_quiet.png" : "assets/ds_btn_panel.png"))
        border { left: 4; top: 4; right: 4; bottom: 4 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: false
    }

    Item {
        id: stage

        anchors.fill: parent
    }
}
