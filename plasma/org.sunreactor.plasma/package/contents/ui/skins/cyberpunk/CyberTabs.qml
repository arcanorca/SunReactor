/*
 * Tab bar, cyberpunk: chamfered buttons, the selected page solid yellow. Left and
 * Right move the selection while the bar has focus.
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

    readonly property CyberStyle style: CyberStyle {
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

        spacing: root.style.gap

        Repeater {
            model: root.tabs

            delegate: ChamferButton {
                id: tab

                required property var modelData
                readonly property bool selected: modelData.id === root.current

                style: root.style
                text: modelData.hint ? modelData.text + "  " + modelData.hint : modelData.text
                active: selected

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: selected
                Accessible.onPressAction: root.picked(tab.modelData.id)

                onClicked: root.picked(tab.modelData.id)
            }
        }
    }
}
