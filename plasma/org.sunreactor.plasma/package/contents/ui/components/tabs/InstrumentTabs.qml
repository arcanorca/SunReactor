/*
 * Tab bar, instrument family: the panel legends of the instrument skins, the
 * same tiny caps as the section rules and Tag.qml. The selected page is ink over
 * an accent segment on the bar's hairline, the others are muted. The weather
 * tab's temperature follows its label in the data face. Left / Right move the
 * selection while the bar has focus; the focused selection takes the Tag outline.
 *
 *     BRIGHTNESS   WEATHER 15°C
 *     ━━━━━━━━━━━  ─────────────────────────────
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "../../modules/Modules.js" as Modules
import "../../skins/instrument"

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }
    readonly property real padY: style.snap(style.gap)

    implicitWidth: tabRow.implicitWidth
    implicitHeight: tabRow.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    // The hairline the legends sit on.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: root.style.line
        color: root.style.rule
    }

    Row {
        id: tabRow

        spacing: root.style.snap(root.style.gap2 * 1.5)

        Repeater {
            model: root.tabs

            delegate: Item {
                id: tab

                required property var modelData
                readonly property bool selected: modelData.id === root.current
                readonly property bool hovered: mouse.containsMouse

                implicitWidth: legend.implicitWidth
                implicitHeight: legend.implicitHeight + 2 * root.padY

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: selected
                Accessible.onPressAction: root.picked(tab.modelData.id)

                Row {
                    id: legend

                    y: root.padY
                    spacing: root.style.gap

                    Text {
                        text: tab.modelData.text
                        textFormat: Text.PlainText
                        font: root.style.legendFont
                        color: tab.selected || tab.hovered ? root.style.ink : root.style.inkMuted
                        Accessible.ignored: true
                    }

                    Text {
                        visible: text.length > 0
                        text: tab.modelData.hint || ""
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.inkMuted
                        Accessible.ignored: true
                    }
                }

                // The accent segment over the hairline, under the selected page.
                Rectangle {
                    visible: tab.selected
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: root.style.line2
                    color: root.style.accent
                }

                // Keyboard focus: the Tag outline around the selected page.
                Rectangle {
                    visible: tab.selected && root.activeFocus
                    anchors.fill: parent
                    anchors.margins: -root.style.gap
                    color: "transparent"
                    border.width: root.style.hairline
                    border.color: root.theme.borderActive
                    radius: root.style.tagRadius
                    antialiasing: radius > 0
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
