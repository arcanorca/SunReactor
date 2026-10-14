/*
 * The page tabs of the Nintendo DS: two rounded touch buttons side by side at the
 * top of the shell. The chosen page is filled DS blue with white type; the other
 * is a pale panel with a blue outline. The weather tab carries the live
 * temperature. Left and Right move the selection while the bar has focus (the
 * same contract as components/tabs).
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "../../modules/Modules.js" as Modules

Item {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }

    implicitHeight: Math.round(style.gu * 2.3)

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    Row {
        anchors.fill: parent
        spacing: Math.round(root.style.gu * 0.45)

        Repeater {
            model: root.tabs

            delegate: NdsPanel {
                id: tab

                required property var modelData
                required property int index

                readonly property bool checked: modelData.id === root.current
                readonly property int count: Math.max(1, root.tabs.length)

                style: root.style
                kind: checked ? "ds" : "panel"
                width: Math.floor((root.width - (count - 1) * Math.round(root.style.gu * 0.45)) / count)
                height: root.height

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.checkable: true
                Accessible.checked: checked
                Accessible.onPressAction: root.picked(modelData.id)

                // The name, then the live hint (the temperature) in the smaller face.
                Row {
                    anchors.centerIn: parent
                    spacing: Math.round(root.style.gu * 0.35)

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: tab.modelData.text
                        font.family: root.style.family
                        font.weight: Font.Bold
                        font.pixelSize: Math.round(root.style.gu * 0.92)
                        color: tab.checked ? root.style.white : root.style.blue
                    }
                    Text {
                        visible: (tab.modelData.hint || "").length > 0
                        anchors.verticalCenter: parent.verticalCenter
                        text: tab.modelData.hint || ""
                        font.family: root.style.family
                        font.weight: Font.Bold
                        font.pixelSize: Math.round(root.style.gu * 0.72)
                        color: tab.checked ? root.style.white : root.style.muted
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.picked(tab.modelData.id)
                }
            }
        }
    }
}
