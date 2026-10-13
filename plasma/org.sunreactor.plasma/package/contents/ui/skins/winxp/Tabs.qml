/*
 * Tab bar, Windows XP: property-sheet tabs. The selected tab rises over the
 * page with a white top edge and XP's orange top stripe; the others sit lower
 * on the grey-beige face. Left and Right step between the pages.
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

    readonly property WinStyle style: WinStyle {
        theme: root.theme
    }
    readonly property real tabH: style.snap(style.sizeBody * 1.9)
    /*! How far an unselected tab sits below the selected one. */
    readonly property real lift: style.snap(style.gap * 0.75)

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    Row {
        id: row

        spacing: root.style.snap(root.style.gap * 0.5)

        Repeater {
            model: root.tabs

            delegate: Item {
                id: tab

                required property var modelData

                readonly property bool selected: modelData.id === root.current
                readonly property bool hovered: mouse.containsMouse

                width: legend.implicitWidth + 2 * root.style.pad
                height: root.tabH + root.lift

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: selected
                Accessible.onPressAction: root.picked(tab.modelData.id)

                Rectangle {
                    id: face

                    x: 0
                    y: tab.selected ? 0 : root.lift
                    width: tab.width
                    height: tab.selected ? tab.height : root.tabH
                    radius: 4
                    border.width: root.style.line
                    border.color: tab.selected ? root.style.frame : root.style.etchDark
                    gradient: Gradient {
                        GradientStop {
                            position: 0
                            color: tab.selected ? root.style.white : root.style.hoverBottom
                        }
                        GradientStop {
                            position: 1
                            color: tab.selected ? root.style.panel : root.style.pressBottom
                        }
                    }
                }

                // The orange stripe over the selected tab.
                Rectangle {
                    visible: tab.selected
                    x: root.style.line * 2
                    y: root.style.line * 2
                    width: tab.width - root.style.line * 4
                    height: root.style.snap(root.style.line * 3)
                    color: root.style.orange
                }

                Row {
                    id: legend

                    anchors.horizontalCenter: face.horizontalCenter
                    y: face.y + (face.height - height) / 2
                    spacing: root.style.gap

                    Text {
                        text: tab.modelData.text
                        textFormat: Text.PlainText
                        font: tab.selected ? root.style.boldFont : root.style.bodyFont
                        color: tab.selected || tab.hovered ? root.style.ink : root.style.inkMuted
                        Accessible.ignored: true
                    }
                    Text {
                        visible: text.length > 0
                        text: tab.modelData.hint || ""
                        textFormat: Text.PlainText
                        font: root.style.bodyFont
                        color: root.style.inkMuted
                        Accessible.ignored: true
                    }
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

    // The line the pages hang from, under the unselected tabs.
    Rectangle {
        x: 0
        y: row.height - root.style.line
        width: root.width
        height: root.style.line
        color: root.style.frame
    }
}
