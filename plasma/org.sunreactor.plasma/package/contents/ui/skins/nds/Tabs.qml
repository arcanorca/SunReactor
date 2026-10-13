/*
 * The page tabs of the Nintendo DS: the lower screen's folder tabs on their own
 * art grid. The chosen page is a blue folder tab that joins the screen below;
 * the other sits back on the rule. The weather tab carries the live
 * temperature. Left and Right move the selection while the bar has focus (the
 * same contract as components/tabs).
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "../../modules/Modules.js" as Modules
import "../pixel"
import "Nds.js" as Nds

Item {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: Nds.TABS_H
        regionName: "tabs"

        // The rule the tabs stand on.
        Rectangle {
            x: 0
            y: surface.artHeight - 1
            width: surface.artWidth
            height: 1
            color: root.theme.borderInactive
            antialiasing: false
        }

        Repeater {
            model: root.tabs

            delegate: Item {
                id: tab

                required property var modelData
                required property int index

                readonly property bool checked: modelData.id === root.current
                readonly property int count: Math.max(1, root.tabs.length)
                readonly property int tabWidth: Math.floor((surface.artWidth - 1 - (count - 1)) / count)

                x: index * (tabWidth + 1)
                y: checked ? 0 : 3
                width: tabWidth
                height: checked ? Nds.TABS_H : Nds.TABS_H - 3

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.checkable: true
                Accessible.checked: checked
                Accessible.onPressAction: root.picked(modelData.id)

                Repeater {
                    model: Nds.box([], 0, 0, tab.width, tab.height,
                                   tab.checked ? root.theme.accent : root.theme.bg,
                                   tab.checked ? root.theme.accent : root.theme.borderInactive)
                    delegate: Rectangle {
                        required property var modelData

                        x: modelData.x
                        y: modelData.y
                        width: modelData.w
                        height: modelData.h
                        color: modelData.c
                        antialiasing: false
                    }
                }

                // The name, then the live hint (the temperature) in the tiny face.
                readonly property int labelsWidth: name.artWidth + (hint.visible ? 3 + hint.artWidth : 0)

                PixelLabel {
                    id: name

                    theme: root.theme
                    x: Math.max(1, Math.floor((tab.width - tab.labelsWidth) / 2))
                    y: Math.floor((tab.height - capHeight) / 2)
                    face: "small"
                    capitals: true
                    color: tab.checked ? root.theme.bg : root.theme.fg
                    text: tab.modelData.text
                    maximumArtWidth: Math.max(0, tab.width - 2)
                }
                PixelLabel {
                    id: hint

                    theme: root.theme
                    visible: (tab.modelData.hint || "").length > 0
                    x: name.x + name.artWidth + 3
                    y: Math.floor((tab.height - capHeight) / 2) + 1
                    face: "tiny"
                    capitals: true
                    color: tab.checked ? root.theme.bg : root.theme.textMuted
                    text: tab.modelData.hint || ""
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
