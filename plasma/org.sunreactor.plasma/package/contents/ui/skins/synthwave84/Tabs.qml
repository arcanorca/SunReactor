/*
 * Tab bar, synthwave84: sheared Orbitron legends. The selected page is pink
 * with a neon underline; the others are muted cyan. Left and Right move the
 * selection while the bar has focus (same contract as components/tabs).
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "../../modules/Modules.js" as Modules

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }

    implicitWidth: tabRow.implicitWidth
    implicitHeight: tabRow.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    Row {
        id: tabRow

        spacing: root.style.snap(root.style.gap2 * 2)

        Repeater {
            model: root.tabs

            delegate: Item {
                id: tab

                required property var modelData
                readonly property bool selected: modelData.id === root.current
                readonly property bool hovered: mouse.containsMouse

                implicitWidth: legend.implicitWidth + 2 * root.style.gap
                implicitHeight: legend.implicitHeight + 2 * root.style.gap

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: selected
                Accessible.onPressAction: root.picked(tab.modelData.id)

                Row {
                    id: legend

                    anchors.centerIn: parent
                    spacing: root.style.gap

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: tab.modelData.text
                        textFormat: Text.PlainText
                        font: root.style.headingFont
                        color: tab.selected ? root.style.pink
                            : (tab.hovered ? root.style.ink : root.style.cyan)
                        opacity: tab.selected || tab.hovered ? 1 : 0.7
                        transformOrigin: Item.Center
                        transform: Matrix4x4 {
                            matrix: Qt.matrix4x4(1, -root.style.shear, 0, 0,
                                                 0, 1, 0, 0,
                                                 0, 0, 1, 0,
                                                 0, 0, 0, 1)
                        }
                        Accessible.ignored: true
                    }

                    Text {
                        visible: text.length > 0
                        anchors.verticalCenter: parent.verticalCenter
                        text: tab.modelData.hint || ""
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.ink
                        Accessible.ignored: true
                    }
                }

                // The neon underline under the selected page.
                Rectangle {
                    visible: tab.selected
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: root.style.line2
                    color: root.style.pink
                }

                // Keyboard focus: a thin outline around the selected page.
                Rectangle {
                    visible: tab.selected && root.activeFocus
                    anchors.fill: parent
                    radius: root.style.radius
                    color: "transparent"
                    border.width: root.style.hairline
                    border.color: root.style.alpha(root.style.pink, 0.8)
                    antialiasing: true
                }

                MouseArea {
                    id: mouse

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.picked(tab.modelData.id)
                }
            }
        }
    }
}
