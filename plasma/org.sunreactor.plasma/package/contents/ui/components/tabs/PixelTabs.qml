/*
 * Tab bar, pixel family: PixelButton tabs in the pixel style's idiom (reverse
 * video on the selected page, a quiet underline on the others) on the bar's
 * own art grid. The weather tab's temperature sits after its label in the tiny
 * face. Tab / Space / Enter as on any button; Left / Right move the selection.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "../../modules/Modules.js" as Modules
import "../../skins/pixel"

Item {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    PixelStyle {
        id: pstyle

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: 15
        regionName: "tabs"

        // The tabs sit on a strip of the panel paper, like an LCD title row:
        // a reverse-video tab then shows against the bezel of every kind.
        Rectangle {
            x: 0
            y: 0
            width: surface.artWidth
            height: surface.artHeight
            color: pstyle.paper
            antialiasing: false
        }

        // The bar's rule, under all the tabs.
        Rectangle {
            x: 0
            y: surface.artHeight - 1
            width: surface.artWidth
            height: 1
            color: pstyle.ghost
            antialiasing: false
        }

        Row {
            x: 2
            y: 1
            spacing: 6

            Repeater {
                model: root.tabs

                delegate: Row {
                    id: tab

                    required property var modelData

                    spacing: 2

                    PixelButton {
                        id: button

                        theme: root.theme
                        style: pstyle
                        text: tab.modelData.text
                        variant: "tab"
                        checked: tab.modelData.id === root.current
                        accessibleName: tab.modelData.text
                        onClicked: root.picked(tab.modelData.id)
                    }

                    // A Row positions its children itself, so the hint is centred
                    // in a plain item beside the button.
                    Item {
                        visible: (tab.modelData.hint || "").length > 0
                        width: hint.artWidth
                        height: button.height

                        PixelLabel {
                            id: hint

                            theme: root.theme
                            face: "tiny"
                            text: tab.modelData.hint || ""
                            color: pstyle.dim
                            y: Math.floor((parent.height - hint.capHeight) / 2)
                        }
                    }
                }
            }
        }
    }
}
