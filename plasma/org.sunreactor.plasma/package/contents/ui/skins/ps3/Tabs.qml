/*
 * Tab bar, PlayStation 3: the XMB category row. Each page is an icon over its
 * label, centred; the selected one is larger and bright with a soft underline
 * glow, the others dim. Left and Right move the selection while the bar has
 * focus (the same contract as components/tabs).
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

    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    Row {
        id: row

        anchors.horizontalCenter: parent.horizontalCenter
        spacing: root.style.gap3 * 2

        Repeater {
            model: root.tabs

            delegate: Item {
                id: tab

                required property var modelData
                readonly property bool selected: modelData.id === root.current
                readonly property bool lit: selected || mouse.containsMouse
                readonly property real iconSize: root.style.snap(root.style.unit * (selected ? 1.2 : 0.95))

                implicitWidth: Math.max(glyph.width, label.implicitWidth) + root.style.gap2
                implicitHeight: glyph.height + label.height + root.style.gap * 2 + root.style.line2 * 3

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: selected
                Accessible.onPressAction: root.picked(tab.modelData.id)

                XmbIcon {
                    id: glyph

                    anchors.horizontalCenter: parent.horizontalCenter
                    kind: tab.modelData.id === "weather" ? "cloud" : "sun"
                    size: tab.iconSize
                    color: tab.lit ? root.style.ink : root.style.inkDim
                }

                Text {
                    id: label

                    anchors.top: glyph.bottom
                    anchors.topMargin: root.style.gap * 0.5
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: tab.modelData.text
                    textFormat: Text.PlainText
                    font: tab.selected ? root.style.tabFont : root.style.legendFont
                    color: tab.lit ? root.style.ink : root.style.inkDim
                    lineHeightMode: Text.FixedHeight
                    lineHeight: (tab.selected ? root.style.sizeTab : root.style.sizeLegend)
                    Accessible.ignored: true
                }

                // The selected page: a soft under-glow and a bright hairline under the word.
                Rectangle {
                    visible: tab.selected
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: label.bottom
                    anchors.topMargin: root.style.gap
                    width: label.width + root.style.gap2
                    height: root.style.snap(root.style.line2 * 3)
                    radius: height / 2
                    color: root.style.alpha(root.style.ink, 0.18)
                }
                Rectangle {
                    visible: tab.selected
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: label.bottom
                    anchors.topMargin: root.style.gap
                    width: label.width + root.style.gap2
                    height: root.style.line2
                    color: root.style.ink
                }

                // Keyboard focus on the selected page: a hairline ring of light.
                Rectangle {
                    visible: tab.selected && root.activeFocus
                    anchors.fill: parent
                    anchors.margins: -root.style.gap
                    radius: root.style.snap(root.style.unit * 0.6)
                    color: "transparent"
                    border.width: root.style.hairline
                    border.color: root.style.alpha(root.style.ink, 0.35)
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
