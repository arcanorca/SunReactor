/*
 * Footer part "refresh", cyberpunk: chamfered HUD button matching ChamferButton
 * geometry — muted rule outline lifting to yellow on hover, cyan focus hairline,
 * and subtle yellow face while pressed.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

import "../../components"
import "Geometry.js" as G

QQC2.AbstractButton {
    id: root

    required property PopupFooter footer

    readonly property CyberStyle style: CyberStyle {
        theme: root.footer.theme
    }
    readonly property real cut: style.snap(style.tabHeight * 0.5)
    readonly property string tip: i18nd(root.footer.domain,
        "Re-read the sun position and weather, then apply the result")

    text: i18ndc(root.footer.domain, "@action:button", "Update Now")
    enabled: root.footer.client.isConnected
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    padding: 0
    implicitWidth: root.style.snap(Math.round(root.style.unit * 1.45) + cut)
    implicitHeight: root.style.snap(Math.round(root.style.unit * 1.45))

    Accessible.role: Accessible.Button
    Accessible.name: root.text
    Accessible.description: root.tip

    QQC2.ToolTip.visible: root.hovered && root.tip.length > 0
    QQC2.ToolTip.text: root.tip
    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

    onClicked: root.footer.client.runOnce(true)

    background: Item {
        Shape {
            anchors.fill: parent
            ShapePath {
                strokeWidth: -1
                fillColor: root.down ? root.style.theme.alpha(root.style.yellow, 0.22)
                    : (root.hovered ? root.style.theme.alpha(root.style.yellow, 0.12) : root.style.panel)
                PathSvg { path: G.chamfer(root.width, root.height, root.cut, 0) }
            }
        }
        Shape {
            anchors.fill: parent
            ShapePath {
                strokeColor: root.visualFocus ? root.style.cyan
                    : (root.hovered ? root.style.yellow : root.style.rule)
                strokeWidth: root.visualFocus || root.hovered ? root.style.outline : root.style.hairline
                fillColor: "transparent"
                joinStyle: ShapePath.MiterJoin
                PathSvg { path: G.chamfer(root.width, root.height, root.cut, root.style.outline / 2) }
            }
        }
    }

    contentItem: Item {
        Kirigami.Icon {
            readonly property real side: root.style.snap(Math.round(Kirigami.Units.iconSizes.small * 0.85 * root.style.theme.scaleFactor))

            anchors.centerIn: parent
            width: side
            height: side
            source: "view-refresh"
            isMask: true
            color: !root.enabled ? root.style.faint
                : (root.hovered || root.visualFocus ? root.style.yellow : root.style.ink)
        }
    }
}
