/*
 * Xbox 360 Blades navigation tabs: curved brushed-metal blade tabs with
 * glowing emerald active edge and Chakra Petch headings.
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
    signal darkModeToggled(bool dark)

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }

    implicitWidth: barRow.implicitWidth
    implicitHeight: barRow.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    RowLayout {
        id: barRow

        anchors.fill: parent
        spacing: root.style.gap

        Row {
            id: tabRow

            spacing: root.style.gap2

            Repeater {
                model: root.tabs

                delegate: Item {
                    id: tab

                    required property var modelData
                    readonly property bool selected: modelData.id === root.current
                    readonly property bool hovered: mouse.containsMouse

                    readonly property real spineClear: Math.round(root.style.gap2 * 1.45)
                    readonly property real rightPad: Math.round(root.style.gap2 * 1.15)

                    implicitWidth: labelRow.implicitWidth + spineClear + rightPad
                    implicitHeight: Math.round(root.style.unit * 1.75)

                    Accessible.role: Accessible.PageTab
                    Accessible.name: modelData.text
                    Accessible.description: modelData.hint || ""
                    Accessible.checkable: true
                    Accessible.checked: tab.selected
                    Accessible.onPressAction: root.picked(tab.modelData.id)

                    BorderImage {
                        anchors.fill: parent
                        source: Qt.resolvedUrl(root.style.asset(
                            tab.selected
                                ? (tab.modelData.id === "weather" ? "xbox_blade_weather_active.png" : "xbox_blade_active.png")
                                : "xbox_blade_idle.png"))
                        border { left: 22; top: 10; right: 22; bottom: 10 }
                        horizontalTileMode: BorderImage.Stretch
                        verticalTileMode: BorderImage.Stretch
                        smooth: true
                    }

                    Row {
                        id: labelRow

                        x: tab.spineClear
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: root.style.gap

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: tab.modelData.text
                            textFormat: Text.PlainText
                            font: root.style.bodyBoldFont
                            color: tab.selected
                                ? (tab.modelData.id === "weather" ? root.style.weatherBlue : root.style.greenBright)
                                : (tab.hovered ? root.style.ink : root.style.inkSoft)
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: text.length > 0
                            text: tab.modelData.hint || ""
                            textFormat: Text.PlainText
                            font: root.style.captionBoldFont
                            color: tab.selected ? root.style.ink : root.style.muted
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

        Item {
            Layout.fillWidth: true
        }

        // In-theme Blades Light / Carbon Dark switcher (stays inside xbox360)
        Item {
            id: modeSwitch

            Layout.alignment: Qt.AlignVCenter
            Layout.rightMargin: root.style.gap
            implicitWidth: switchContent.implicitWidth + root.style.gap2 * 1.1
            implicitHeight: Math.round(root.style.unit * 1.38)
            activeFocusOnTab: true

            readonly property bool hovered: switchMouse.containsMouse

            Accessible.role: Accessible.Button
            Accessible.name: root.style.isDark
                ? i18nd(root.domain, "Switch Xbox 360 skin to Light Blades")
                : i18nd(root.domain, "Switch Xbox 360 skin to Carbon Dark Blades")
            Accessible.checkable: true
            Accessible.checked: root.style.isDark
            Accessible.onPressAction: toggleMode()

            function toggleMode() {
                const next = !root.style.isDark;
                root.theme.xboxDarkModeLocal = next;
                root.darkModeToggled(next);
            }

            Keys.onSpacePressed: toggleMode()
            Keys.onReturnPressed: toggleMode()

            BorderImage {
                anchors.fill: parent
                source: Qt.resolvedUrl(root.style.asset(modeSwitch.hovered ? "xbox_pill_called.png" : "xbox_pill_idle.png"))
                border { left: 14; top: 10; right: 14; bottom: 10 }
                horizontalTileMode: BorderImage.Stretch
                verticalTileMode: BorderImage.Stretch
                smooth: true
            }

            Row {
                id: switchContent

                anchors.centerIn: parent
                spacing: Math.max(3, root.style.gap * 0.5)

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(6, Math.round(root.style.unit * 0.36))
                    height: width
                    radius: width / 2
                    color: root.style.isDark ? "#f5d442" : root.style.green
                    border.width: 1
                    border.color: root.style.isDark ? "#b88a12" : root.style.steel
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.style.isDark
                        ? i18ndc(root.domain, "Xbox 360 light/dark mode switch label", "Light")
                        : i18ndc(root.domain, "Xbox 360 light/dark mode switch label", "Dark")
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.ink
                }
            }

            MouseArea {
                id: switchMouse

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: modeSwitch.toggleMode()
            }
        }
    }
}
