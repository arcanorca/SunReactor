/*
 * Xbox 360 footer refresh control: glossy 360 metallic pill button.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

import "../../components"

Item {
    id: root

    required property PopupFooter footer

    readonly property XboxStyle style: XboxStyle {
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

        background: Image {
            source: Qt.resolvedUrl(root.style.asset(btn.pressed || btn.hovered ? "xbox_orb_btn_active.png" : "xbox_orb_btn.png"))
            smooth: true
            mipmap: true
        }

        contentItem: Kirigami.Icon {
            anchors.centerIn: parent
            implicitWidth: Math.round(root.style.unit * 0.78)
            implicitHeight: implicitWidth
            source: "view-refresh"
            isMask: true
            color: btn.pressed || btn.hovered ? "#ffffff" : root.style.ink
        }
    }
}
