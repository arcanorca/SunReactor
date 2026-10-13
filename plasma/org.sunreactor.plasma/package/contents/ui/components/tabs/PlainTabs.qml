/*
 * Tab bar, plain: Plasma's own tab buttons in a row, used without a daemon
 * theme (Plasma colours), and by any renderer without a skin of its own. The
 * selected page is the checked button; the buttons are not checkable, so the
 * selection only ever moves with `current`.
 *
 * Contract (every file in components/tabs/): `theme`, `tabs` = [{ id, text,
 * hint }] in order (hint "" for none), `current` = the selected id, and the
 * `picked(id)` signal when the user picks a page (click, Space, arrows).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3

import "../../engine"
import "../../modules/Modules.js" as Modules

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    Row {
        id: row

        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: root.tabs

            delegate: PlasmaComponents3.TabButton {
                id: tab

                required property var modelData

                text: modelData.hint ? modelData.text + "  " + modelData.hint : modelData.text
                checkable: false
                checked: modelData.id === root.current
                onClicked: root.picked(tab.modelData.id)

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: tab.checked
            }
        }
    }
}
