/*
 * Footer part "refresh", Nothing: a round hairline button with the refresh glyph
 * in ink. Hover and keyboard focus lift the outline to ink, a press fills it
 * faintly, and the glyph goes muted while the daemon does not answer.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

import "../../components"

QQC2.AbstractButton {
    id: root

    required property PopupFooter footer

    readonly property NothingStyle style: NothingStyle {
        theme: root.footer.theme
    }
    readonly property string tip: i18nd(root.footer.domain,
        "Re-read the sun position and weather, then apply the result")

    text: i18ndc(root.footer.domain, "@action:button", "Update Now")
    enabled: root.footer.client.isConnected
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    padding: 0
    implicitWidth: root.style.snap(24)
    implicitHeight: root.style.snap(24)

    Accessible.role: Accessible.Button
    Accessible.name: root.text
    Accessible.description: root.tip

    QQC2.ToolTip.visible: root.hovered && root.tip.length > 0
    QQC2.ToolTip.text: root.tip
    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

    onClicked: root.footer.client.runOnce(true)

    background: Rectangle {
        radius: width / 2
        antialiasing: true
        color: root.down ? root.style.theme.alpha(root.style.ink, 0.12) : "transparent"
        border.width: root.style.hairline
        border.color: root.visualFocus || root.hovered ? root.style.ink : root.style.outline
    }

    contentItem: Item {
        Kirigami.Icon {
            readonly property real side: root.style.snap(root.style.unit * 0.8)

            anchors.centerIn: parent
            width: side
            height: side
            source: "view-refresh"
            isMask: true
            color: root.enabled ? root.style.ink : root.style.muted
        }
    }
}
