/*
 * Amiga page tabs: two Workbench gadgets, the chosen page lit (orange). The
 * brightness page is labelled "Light" (the day curve's short name) so both tabs
 * fit beside the theme button; the temperature is the Weather window's hero.
 * Art-grid based; this file is the skin's TABBAR entry (Modules.js).
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: 11
        regionName: "tabs"

        Row {
            id: row

            spacing: 2

            Repeater {
                model: root.tabs

                delegate: AmigaGadget {
                    id: gadget

                    required property var modelData

                    theme: root.theme
                    style: tokens
                    text: modelData.id === "brightness"
                        ? i18ndc("plasma_applet_org.sunreactor.plasma", "Short tab label of the brightness page in the Amiga skin", "Light")
                        : modelData.text
                    lit: root.current === modelData.id
                    onActivated: root.picked(modelData.id)
                }
            }
        }
    }
}
