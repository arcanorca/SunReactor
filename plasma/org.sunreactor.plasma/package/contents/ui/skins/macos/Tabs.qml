/*
 * Tab bar, macos: the popup's two pages as one native segmented control. Same
 * contract as components/tabs (tabs, current, picked): Left and Right move the
 * page while the bar has focus.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }

    implicitWidth: seg.implicitWidth
    implicitHeight: seg.implicitHeight

    Accessible.role: Accessible.PageTabList

    Segmented {
        id: seg

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        segments: root.tabs
        current: root.current
        pageTabs: true

        onPicked: id => root.picked(id)
    }
}
