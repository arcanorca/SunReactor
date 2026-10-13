/*
 * The tab bar, Wii style: the menu's bottom-corner pills as page tabs. The
 * chosen page is a white pill with a blue outline; the weather tab carries the
 * live temperature. Left and Right move the selection while the bar has focus
 * (the same contract as components/tabs).
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

    readonly property WiiStyle style: WiiStyle {
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

        spacing: root.style.gap2

        Repeater {
            model: root.tabs

            delegate: WiiPill {
                id: tab

                required property var modelData

                style: root.style
                label: modelData.text
                hint: modelData.hint || ""
                variant: "outline"
                current: modelData.id === root.current
                focused: current && root.activeFocus

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: tab.current
                Accessible.onPressAction: root.picked(tab.modelData.id)

                onClicked: root.picked(tab.modelData.id)
            }
        }
    }
}
