/*
 * PlayStation 3 XMB footer refresh control: translucent glass pill with soft
 * white border and XMB radial hover bloom.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

import "../../components"

Item {
    id: root

    required property PopupFooter footer

    readonly property XmbStyle style: XmbStyle {
        theme: root.footer.theme
    }
    readonly property string label: i18ndc(footer.domain, "@action:button", "Update Now")

    implicitWidth: btn.implicitWidth
    implicitHeight: btn.implicitHeight

    QQC2.AbstractButton {
        id: btn

        implicitWidth: Math.round(root.style.unit * 1.65)
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
            source: Qt.resolvedUrl("assets/ps3_orb_btn.png")
            smooth: true
            mipmap: true
            opacity: btn.hovered || btn.visualFocus || btn.pressed ? 1.0 : 0.85
        }

        contentItem: Kirigami.Icon {
            anchors.centerIn: parent
            implicitWidth: Math.round(root.style.unit * 0.85)
            implicitHeight: implicitWidth
            source: "view-refresh"
            isMask: true
            color: root.style.ink
            opacity: btn.hovered || btn.pressed ? 1.0 : 0.85
        }
    }
}
