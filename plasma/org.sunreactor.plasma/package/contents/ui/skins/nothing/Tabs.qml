/*
 * Tab bar, Nothing: the pages as round pills, the way Automation sets its mode.
 * The selected page is the filled white pill; the others are hairline outlines.
 *
 *   ( BRIGHTNESS )  ( WEATHER 15°C )
 *
 * Same contract as components/tabs/: `tabs` = [{ id, text, hint }], `current`,
 * and `picked(id)`. Left and Right move between pages, Space or Enter picks one.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

import "../../engine"
import "../../modules/Modules.js" as Modules

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    RowLayout {
        id: row

        spacing: root.style.gap

        Repeater {
            model: root.tabs

            delegate: Pill {
                id: tab

                required property var modelData

                style: root.style
                radio: true
                text: modelData.hint ? modelData.text + " " + modelData.hint : modelData.text
                accessibleName: modelData.text
                accessibleDescription: modelData.hint || ""
                active: modelData.id === root.current

                Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
                Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

                onClicked: root.picked(tab.modelData.id)
            }
        }
    }
}
