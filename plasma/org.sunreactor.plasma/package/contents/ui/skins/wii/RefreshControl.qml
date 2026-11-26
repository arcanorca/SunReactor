/*
 * Wii Menu circular glossy orb refresh button sitting on the curved Wii dock.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

import "../../components"

Item {
    id: root

    required property PopupFooter footer

    readonly property WiiStyle style: WiiStyle {
        theme: root.footer.theme
    }
    readonly property string label: i18ndc(footer.domain, "@action:button", "Update Now")

    implicitWidth: btn.implicitWidth
    implicitHeight: btn.implicitHeight

    QQC2.AbstractButton {
        id: btn

        implicitWidth: Math.round(root.style.unit * 1.85)
        implicitHeight: implicitWidth
        hoverEnabled: true
        focusPolicy: Qt.StrongFocus
        enabled: root.footer.client.isConnected

        Accessible.role: Accessible.Button
        Accessible.name: root.label

        QQC2.ToolTip.visible: hovered
        QQC2.ToolTip.text: i18nd(root.footer.domain, "Re-read the sun position and weather, then apply the result")
        QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

        onClicked: root.footer.client.runOnce(true)

        background: Item {
            Image {
                anchors.fill: parent
                source: Qt.resolvedUrl("assets/wii_orb.png")
                smooth: true
                mipmap: true
            }
            Rectangle {
                visible: btn.hovered || btn.visualFocus || btn.pressed
                anchors.fill: parent
                anchors.margins: 2
                radius: width / 2
                color: btn.pressed ? root.style.alpha(root.style.accent, 0.18) : "transparent"
                border.width: root.style.line2
                border.color: root.style.accent
                antialiasing: true
            }
        }

        contentItem: Kirigami.Icon {
            anchors.centerIn: parent
            implicitWidth: Math.round(root.style.unit * 0.9)
            implicitHeight: implicitWidth
            source: "view-refresh"
            isMask: true
            color: btn.hovered || btn.pressed ? root.style.accentInk : root.style.inkSoft
        }
    }
}
